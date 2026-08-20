"""Drive one QuickPLS-owned Windows Open/Save dialog without global keystrokes.

This is the generic companion to ``windows_native_save_export.py``.  It reuses
that helper's exact main-window/process ownership checks, but deliberately does
not contain method or workbook logic.  The caller supplies an absolute source
or new destination below an explicit allowed root and receives JSON-lines
evidence for the bound controls and final bytes.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import time
import traceback
from pathlib import Path
from typing import Any

try:
    from validation.windows_native_save_export import (
        GateFailure,
        control_summary,
        emit,
        owned_dialogs,
        visible_quickpls_window,
    )
except ModuleNotFoundError:  # Direct ``python validation/...py`` execution.
    from windows_native_save_export import (
        GateFailure,
        control_summary,
        emit,
        owned_dialogs,
        visible_quickpls_window,
    )


FILENAME_CONTROL_IDS = frozenset({1001, 1148})


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--mode",
        choices=("open", "save", "save-cancel", "assert-absent"),
        required=True,
    )
    parser.add_argument("--target", required=True)
    parser.add_argument("--allowed-root", required=True)
    parser.add_argument("--window-title", required=True)
    parser.add_argument(
        "--extension", action="append", dest="extensions", required=True
    )
    parser.add_argument("--timeout-seconds", type=float, default=45.0)
    return parser.parse_args()


def validate_target(args: argparse.Namespace) -> tuple[Path, Path, tuple[str, ...]]:
    supplied = Path(args.target)
    if not supplied.is_absolute():
        raise GateFailure("The file-dialog target must be absolute.")
    root = Path(args.allowed_root).resolve(strict=True)
    target = supplied.resolve(strict=args.mode == "open")
    try:
        target.relative_to(root)
    except ValueError as error:
        raise GateFailure(
            "The file-dialog target must remain below --allowed-root."
        ) from error
    extensions = tuple(
        sorted({value.strip().lower().lstrip(".") for value in args.extensions})
    )
    if not extensions or any(
        not value or not value.replace("_", "").isalnum() for value in extensions
    ):
        raise GateFailure("--extension values must be non-empty file extensions.")
    if target.suffix.lower().lstrip(".") not in extensions:
        raise GateFailure(f"The target extension must be one of {extensions}.")
    if args.mode == "open":
        if not target.is_file() or target.is_symlink() or target.stat().st_size <= 0:
            raise GateFailure("Open targets must be non-empty regular files.")
    elif target.exists() or not target.parent.is_dir():
        raise GateFailure(
            "Save targets must be new files with an existing parent directory."
        )
    return target, root, extensions


def exact_controls(
    dialog: Any, require_cancel: bool
) -> tuple[Any, Any, Any | None, list[dict[str, Any]]]:
    controls = dialog.descendants()
    edits = [
        control
        for control in controls
        if control.class_name() == "Edit"
        and int(control.control_id()) in FILENAME_CONTROL_IDS
    ]
    actions = [
        control
        for control in controls
        if control.class_name() == "Button" and int(control.control_id()) == 1
    ]
    cancels = [
        control
        for control in controls
        if control.class_name() == "Button" and int(control.control_id()) == 2
    ]
    summary = control_summary(dialog)
    if len(edits) != 1 or len(actions) != 1 or (require_cancel and len(cancels) != 1):
        raise GateFailure(
            "The owned common-file dialog did not expose exactly one filename Edit "
            "(ID 1001 or 1148), one action Button ID 1, and the requested "
            f"Cancel Button ID 2: {json.dumps(summary)}"
        )
    return edits[0], actions[0], cancels[0] if cancels else None, summary


def submit(
    dialog: Any, edit: Any, action: Any, target: Path, deadline: float, win32con: Any
) -> dict[str, Any]:
    if not action.is_visible() or not action.is_enabled():
        raise GateFailure("The owned file-dialog action is not visible and enabled.")
    target_text = str(target)
    attempts: list[dict[str, Any]] = []
    dialog.set_focus()
    dialog.wait_for_idle()
    verified = False
    for number in range(1, 4):
        if time.monotonic() >= deadline:
            break
        edit.set_keyboard_focus()
        edit.set_edit_text(target_text)
        dialog.wait_for_idle()
        observed: list[str] = []
        for _ in range(2):
            time.sleep(0.1)
            observed.append(edit.window_text())
        attempts.append({"attempt": number, "settledReads": observed})
        if observed == [target_text, target_text]:
            verified = True
            break
    if not verified:
        raise GateFailure(
            f"The filename field did not retain the exact target: {attempts}"
        )
    action.send_message(win32con.BM_CLICK, 0, 0)
    return {
        "filenameControlId": int(edit.control_id()),
        "actionControlId": int(action.control_id()),
        "filenameVerified": True,
        "submissionMethod": "BM_CLICK",
        "setAttempts": attempts,
    }


def cancel_save(
    dialog: Any, edit: Any, cancel: Any, target: Path, deadline: float, win32con: Any
) -> dict[str, Any]:
    if not cancel.is_visible() or not cancel.is_enabled():
        raise GateFailure(
            "The owned file-dialog Cancel action is not visible and enabled."
        )
    target_text = str(target)
    attempts: list[dict[str, Any]] = []
    dialog.set_focus()
    dialog.wait_for_idle()
    verified = False
    for number in range(1, 4):
        if time.monotonic() >= deadline:
            break
        edit.set_keyboard_focus()
        edit.set_edit_text(target_text)
        dialog.wait_for_idle()
        observed: list[str] = []
        for _ in range(2):
            time.sleep(0.1)
            observed.append(edit.window_text())
        attempts.append({"attempt": number, "settledReads": observed})
        if observed == [target_text, target_text]:
            verified = True
            break
    if not verified:
        raise GateFailure(
            f"The filename field did not retain the cancelled target: {attempts}"
        )
    cancel.send_message(win32con.BM_CLICK, 0, 0)
    return {
        "filenameControlId": int(edit.control_id()),
        "cancelControlId": int(cancel.control_id()),
        "filenameVerified": True,
        "submissionMethod": "BM_CLICK",
        "setAttempts": attempts,
    }


def wait_for_completion(
    mode: str, dialog: Any, target: Path, deadline: float
) -> dict[str, Any]:
    prior_size = -1
    stable_reads = 0
    while time.monotonic() < deadline:
        try:
            dialog_visible = bool(dialog.is_visible())
        except Exception:
            # A destroyed Win32 dialog wrapper raises instead of exposing
            # WindowSpecification.exists(); destruction is the terminal state.
            dialog_visible = False
        if mode == "open" and not dialog_visible:
            break
        if mode == "save" and target.is_file() and target.stat().st_size > 0:
            size = target.stat().st_size
            stable_reads = stable_reads + 1 if size == prior_size else 0
            prior_size = size
            if not dialog_visible and stable_reads >= 2:
                break
        if mode == "save-cancel" and not dialog_visible:
            if target.exists():
                raise GateFailure("Cancelled Save dialog published a partial file.")
            break
        time.sleep(0.15)
    else:
        raise GateFailure(f"The {mode} dialog did not reach a stable terminal state.")
    if mode == "save-cancel":
        return {
            "path": str(target),
            "exists": False,
            "dialogClosed": True,
            "cancelledBeforePublication": True,
        }
    payload = target.read_bytes()
    return {
        "path": str(target),
        "size": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
        "dialogClosed": True,
    }


def main() -> int:
    args = parse_args()
    phase = "target_validation"
    dialog_handle = 0
    diagnostics: dict[str, Any] = {}
    try:
        target, root, extensions = validate_target(args)
        if os.name != "nt":
            raise GateFailure("Native common-file dialog automation requires Windows.")
        phase = "dependency_import"
        from pywinauto import Desktop
        import win32api
        import win32con
        import win32gui
        import win32process

        phase = "main_window_binding"
        main_window, main_info = visible_quickpls_window(
            args.window_title, Desktop, win32api, win32con, win32process
        )
        emit(
            {
                "event": "ready",
                "passed": True,
                "phase": phase,
                "mode": args.mode,
                "targetPath": str(target),
                "allowedRoot": str(root),
                "extensions": extensions,
                "mainWindow": main_info,
            }
        )
        deadline = time.monotonic() + max(5.0, args.timeout_seconds)
        phase = "owned_dialog_binding"
        inspected: list[dict[str, Any]] = []
        dialog = None
        if args.mode == "assert-absent":
            while time.monotonic() < deadline:
                candidates, inspected = owned_dialogs(
                    int(main_info["pid"]),
                    int(main_window.handle),
                    Desktop,
                    win32con,
                    win32gui,
                )
                if candidates:
                    raise GateFailure(
                        "A native common-file dialog appeared across the UI export-cancel boundary: "
                        f"{inspected}"
                    )
                time.sleep(0.05)
            if target.exists():
                raise GateFailure("UI export cancellation published a partial file.")
            emit(
                {
                    "event": "complete",
                    "passed": True,
                    "phase": "owned_dialog_absence",
                    "mode": args.mode,
                    "mainWindow": main_info,
                    "nativeDialogObserved": False,
                    "file": {
                        "path": str(target),
                        "exists": False,
                        "cancelledBeforePublication": True,
                    },
                }
            )
            return 0
        while time.monotonic() < deadline:
            candidates, inspected = owned_dialogs(
                int(main_info["pid"]),
                int(main_window.handle),
                Desktop,
                win32con,
                win32gui,
            )
            if len(candidates) > 1:
                raise GateFailure(
                    f"Expected one owned common-file dialog; found {len(candidates)}: {inspected}"
                )
            if len(candidates) == 1:
                dialog = candidates[0]
                break
            time.sleep(0.1)
        if dialog is None:
            raise GateFailure("No owned common-file dialog appeared before timeout.")
        dialog_handle = int(dialog.handle)
        diagnostics["inspectedDialogs"] = inspected
        phase = "exact_control_binding"
        edit, action, cancel, controls = exact_controls(
            dialog, require_cancel=args.mode == "save-cancel"
        )
        diagnostics["controls"] = controls
        phase = "dialog_submission"
        submission = (
            cancel_save(dialog, edit, cancel, target, deadline, win32con)
            if args.mode == "save-cancel" and cancel is not None
            else submit(dialog, edit, action, target, deadline, win32con)
        )
        phase = "file_completion"
        file_evidence = wait_for_completion(args.mode, dialog, target, deadline)
        emit(
            {
                "event": "complete",
                "passed": True,
                "phase": phase,
                "mode": args.mode,
                "mainWindow": main_info,
                "dialogHandle": dialog_handle,
                "submission": submission,
                "file": file_evidence,
            }
        )
        return 0
    except Exception as error:
        if dialog_handle:
            try:
                import win32con
                import win32gui

                if win32gui.IsWindow(dialog_handle):
                    win32gui.PostMessage(dialog_handle, win32con.WM_CLOSE, 0, 0)
            except Exception as close_error:
                diagnostics["dialogCloseError"] = str(close_error)
        emit(
            {
                "event": "complete",
                "passed": False,
                "phase": phase,
                "mode": args.mode,
                "targetPath": args.target,
                "error": {"type": type(error).__name__, "message": str(error)},
                "diagnostics": diagnostics,
                "traceback": traceback.format_exc(),
            }
        )
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
