#!/usr/bin/env python3
"""Verify QuickPLS' native unsaved-close prompt for one wrapper-owned PID."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import time
import traceback
from pathlib import Path
from typing import Any


SUITE_ID = "quickpls_v255_windows_unsaved_close_guard_v1"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_new(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8") as stream:
        json.dump(payload, stream, indent=2)
        stream.write("\n")


def executable_path(pid: int, win32api: Any, win32con: Any, win32process: Any) -> Path:
    handle = win32api.OpenProcess(
        win32con.PROCESS_QUERY_INFORMATION | win32con.PROCESS_VM_READ, False, pid
    )
    try:
        return Path(str(win32process.GetModuleFileNameEx(handle, 0))).resolve(strict=True)
    finally:
        handle.Close()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--owner-pid", type=int, required=True)
    parser.add_argument("--owner-executable", type=Path, required=True)
    parser.add_argument("--candidate-sha256", required=True)
    parser.add_argument("--screenshot", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    args = parser.parse_args()
    report_path = args.report.resolve()
    screenshot = args.screenshot.resolve()
    phase = "preflight"
    dialog_handle = 0
    try:
        if os.name != "nt":
            raise RuntimeError("The unsaved-close guard requires Windows.")
        candidate = args.owner_executable.resolve(strict=True)
        if sha256(candidate).lower() != args.candidate_sha256.lower():
            raise RuntimeError("The exact candidate bytes changed before close verification.")
        if args.owner_pid <= 0 or report_path.exists() or screenshot.exists():
            raise RuntimeError("PID must be positive and report/screenshot targets must be new.")
        screenshot.parent.mkdir(parents=True, exist_ok=True)

        from pywinauto import Desktop
        import win32api
        import win32con
        import win32gui
        import win32process

        phase = "exact_main_window"
        candidates = []
        inspected = []
        for window in Desktop(backend="win32").windows(visible_only=True):
            if int(window.element_info.process_id) != args.owner_pid:
                continue
            observed_executable = executable_path(
                args.owner_pid, win32api, win32con, win32process
            )
            info = {
                "pid": args.owner_pid,
                "handle": int(window.handle),
                "title": window.window_text(),
                "class_name": window.class_name(),
                "executable": str(observed_executable),
            }
            inspected.append(info)
            if (
                observed_executable == candidate
                and window.window_text().startswith("QuickPLS - ")
                and window.window_text().endswith(" *")
            ):
                candidates.append((window, info))
        if len(candidates) != 1:
            raise RuntimeError(
                f"Expected one exact dirty QuickPLS main window; found {len(candidates)}: {inspected}"
            )
        main_window, main_info = candidates[0]
        main_handle = int(main_window.handle)

        phase = "owned_close_request"
        win32gui.PostMessage(main_handle, win32con.WM_CLOSE, 0, 0)
        deadline = time.monotonic() + max(5.0, args.timeout_seconds)
        dialog = None
        inspected_dialogs = []
        while time.monotonic() < deadline:
            owned = []
            inspected_dialogs = []
            for window in Desktop(backend="win32").windows(
                class_name="#32770", visible_only=True
            ):
                if int(window.element_info.process_id) != args.owner_pid:
                    continue
                handle = int(window.handle)
                direct_owner = int(win32gui.GetWindow(handle, win32con.GW_OWNER) or 0)
                root_owner = int(win32gui.GetAncestor(handle, win32con.GA_ROOTOWNER) or 0)
                info = {
                    "handle": handle,
                    "title": window.window_text(),
                    "direct_owner": direct_owner,
                    "root_owner": root_owner,
                }
                inspected_dialogs.append(info)
                if direct_owner == main_handle or root_owner == main_handle:
                    owned.append(window)
            if len(owned) > 1:
                raise RuntimeError(f"More than one exact-PID owned dialog appeared: {inspected_dialogs}")
            if len(owned) == 1:
                dialog = owned[0]
                break
            time.sleep(0.1)
        if dialog is None:
            raise RuntimeError(f"No owned close dialog appeared: {inspected_dialogs}")
        dialog_handle = int(dialog.handle)

        phase = "exact_prompt_contract"
        controls = dialog.descendants()
        button_rows = [
            (control, control.window_text().strip().replace("&", ""))
            for control in controls
            if control.class_name() == "Button" and control.window_text().strip()
        ]
        button_texts = [text for _, text in button_rows]
        expected_buttons = ["Save", "Don't Save", "Cancel"]
        if sorted(button_texts) != sorted(expected_buttons):
            raise RuntimeError(
                f"Unsaved-close buttons are not exact: {button_texts}"
            )
        static_text = " ".join(
            control.window_text().strip()
            for control in controls
            if control.class_name() == "Static" and control.window_text().strip()
        )
        if (
            dialog.window_text() != "QuickPLS"
            or "Save changes to " not in static_text
            or " before closing?" not in static_text
        ):
            raise RuntimeError(
                f"Unsaved-close title/message is not exact: {dialog.window_text()!r} / {static_text!r}"
            )
        dialog.capture_as_image().save(screenshot, format="PNG")
        if screenshot.stat().st_size <= 1000:
            raise RuntimeError("The native prompt screenshot is empty or implausibly small.")

        phase = "cancel_and_survival"
        cancel = next(control for control, text in button_rows if text == "Cancel")
        cancel.send_message(win32con.BM_CLICK, 0, 0)
        while time.monotonic() < deadline:
            if not win32gui.IsWindow(dialog_handle):
                break
            time.sleep(0.1)
        if win32gui.IsWindow(dialog_handle):
            raise RuntimeError("Cancel did not close the native unsaved-close prompt.")
        if not win32gui.IsWindow(main_handle):
            raise RuntimeError("Cancel closed the wrapper-owned QuickPLS window.")
        if executable_path(args.owner_pid, win32api, win32con, win32process) != candidate:
            raise RuntimeError("The exact candidate process did not survive Cancel.")

        payload = {
            "schema_version": 1,
            "suite_id": SUITE_ID,
            "passed": True,
            "candidate": {
                "pid": args.owner_pid,
                "path": str(candidate),
                "sha256": args.candidate_sha256.upper(),
            },
            "main_window": main_info,
            "dialog": {
                "handle": dialog_handle,
                "title": "QuickPLS",
                "message": static_text,
                "buttons": expected_buttons,
                "direct_or_root_owned": True,
            },
            "cancel_kept_exact_pid_alive": True,
            "screenshot": {"path": str(screenshot), "sha256": sha256(screenshot)},
            "failures": [],
        }
        write_new(report_path, payload)
        return 0
    except Exception as error:
        if dialog_handle:
            try:
                import win32con
                import win32gui

                if win32gui.IsWindow(dialog_handle):
                    win32gui.PostMessage(dialog_handle, win32con.WM_CLOSE, 0, 0)
            except Exception:
                pass
        failure = {
            "schema_version": 1,
            "suite_id": SUITE_ID,
            "passed": False,
            "phase": phase,
            "error": {"type": type(error).__name__, "message": str(error)},
            "traceback": traceback.format_exc(),
        }
        if not report_path.exists():
            write_new(report_path, failure)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
