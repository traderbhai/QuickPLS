"""Drive one QuickPLS-owned Windows Save dialog and verify its diagnostic ZIP.

The helper binds an exact visible ``quickpls-desktop.exe`` main window and the
single ``#32770`` dialog owned by that window. It never sends global keystrokes,
never searches by localized dialog title, and refuses an existing or out-of-scope
target. JSON Lines separate readiness from the final archive attestation.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import time
import traceback
import zipfile
from pathlib import Path
from typing import Any


EXPECTED_EXECUTABLE = "quickpls-desktop.exe"
EXPECTED_ENTRIES = (
    "metadata/system.json",
    "logs/events.jsonl",
    "manifest.json",
)
PAYLOAD_ENTRIES = EXPECTED_ENTRIES[:2]
MAX_ENTRY_BYTES = 256 * 1024
MAX_UNCOMPRESSED_BYTES = 512 * 1024
MAX_ARCHIVE_BYTES = 520 * 1024
FILENAME_SETTLE_SECONDS = 0.10
FILENAME_STABLE_READS = 2
FILENAME_SET_ATTEMPTS = 3
BUTTON_CLICK_MESSAGE = 0x00F5  # Win32 BM_CLICK.


class GateFailure(RuntimeError):
    """Expected fail-closed validation or automation failure."""


def emit(payload: dict[str, Any]) -> None:
    print(json.dumps(payload, ensure_ascii=False, sort_keys=True), flush=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    parser.add_argument("--results-root", required=True)
    parser.add_argument("--window-title", required=True)
    parser.add_argument("--timeout-seconds", type=float, default=45.0)
    return parser.parse_args()


def validate_target(target_arg: str, results_root_arg: str) -> tuple[Path, Path]:
    supplied = Path(target_arg)
    if not supplied.is_absolute():
        raise GateFailure("The diagnostic ZIP target must be absolute.")
    target = supplied.resolve(strict=False)
    results_root = Path(results_root_arg).resolve(strict=True)
    try:
        target.relative_to(results_root)
    except ValueError as error:
        raise GateFailure(
            "The diagnostic ZIP target must resolve inside validation/results."
        ) from error
    if target == results_root or target.suffix.lower() != ".zip":
        raise GateFailure(
            "The diagnostic ZIP target must name a .zip file under validation/results."
        )
    if target.exists():
        raise GateFailure(f"The diagnostic ZIP target already exists: {target}")
    if not target.parent.is_dir():
        raise GateFailure(f"The diagnostic ZIP target parent does not exist: {target.parent}")
    return target, results_root


def executable_path(pid: int, win32api: Any, win32con: Any, win32process: Any) -> str:
    access = win32con.PROCESS_QUERY_INFORMATION | win32con.PROCESS_VM_READ
    handle = win32api.OpenProcess(access, False, pid)
    try:
        return str(win32process.GetModuleFileNameEx(handle, 0))
    finally:
        handle.Close()


def visible_quickpls_window(
    expected_title: str,
    Desktop: Any,
    win32api: Any,
    win32con: Any,
    win32process: Any,
) -> tuple[Any, dict[str, Any]]:
    candidates: list[tuple[Any, dict[str, Any]]] = []
    inspected: list[dict[str, Any]] = []
    for window in Desktop(backend="win32").windows(visible_only=True):
        pid = int(window.element_info.process_id)
        title = window.window_text()
        try:
            executable = executable_path(pid, win32api, win32con, win32process)
        except Exception as error:  # A foreign elevated process may reject inspection.
            if title.startswith("QuickPLS"):
                inspected.append(
                    {
                        "pid": pid,
                        "handle": int(window.handle),
                        "title": title,
                        "executableError": str(error),
                    }
                )
            continue
        if Path(executable).name.lower() != EXPECTED_EXECUTABLE:
            continue
        info = {
            "pid": pid,
            "handle": int(window.handle),
            "title": title,
            "executable": executable,
        }
        inspected.append(info)
        if title == expected_title:
            candidates.append((window, info))
    if len(candidates) != 1:
        raise GateFailure(
            f"Expected exactly one visible {EXPECTED_EXECUTABLE} main window with title "
            f"{expected_title!r}; found {len(candidates)}. Inspected: "
            f"{json.dumps(inspected, ensure_ascii=False)}"
        )
    return candidates[0]


def owned_dialogs(
    pid: int,
    main_handle: int,
    Desktop: Any,
    win32con: Any,
    win32gui: Any,
) -> tuple[list[Any], list[dict[str, Any]]]:
    owned: list[Any] = []
    inspected: list[dict[str, Any]] = []
    for window in Desktop(backend="win32").windows(
        class_name="#32770", visible_only=True
    ):
        if int(window.element_info.process_id) != pid:
            continue
        handle = int(window.handle)
        direct_owner = int(win32gui.GetWindow(handle, win32con.GW_OWNER) or 0)
        root_owner = int(win32gui.GetAncestor(handle, win32con.GA_ROOTOWNER) or 0)
        info = {
            "handle": handle,
            "pid": pid,
            "title": window.window_text(),
            "className": window.class_name(),
            "directOwner": direct_owner,
            "rootOwner": root_owner,
        }
        inspected.append(info)
        if direct_owner == main_handle or root_owner == main_handle:
            owned.append(window)
    return owned, inspected


def control_summary(dialog: Any) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for control in dialog.descendants()[:120]:
        try:
            rows.append(
                {
                    "className": control.class_name(),
                    "controlId": int(control.control_id()),
                    "text": control.window_text()[:160],
                }
            )
        except Exception as error:
            rows.append({"error": str(error)})
    return rows


def exact_dialog_controls(dialog: Any) -> tuple[Any, Any, list[dict[str, Any]]]:
    controls = dialog.descendants()
    filename_controls = [
        control
        for control in controls
        if control.class_name() == "Edit" and int(control.control_id()) == 1001
    ]
    save_controls = [
        control
        for control in controls
        if control.class_name() == "Button" and int(control.control_id()) == 1
    ]
    summary = control_summary(dialog)
    if len(filename_controls) != 1 or len(save_controls) != 1:
        raise GateFailure(
            "The owned Save dialog did not expose exactly one filename Edit control "
            "ID 1001 and one Save Button control ID 1. Controls: "
            f"{json.dumps(summary, ensure_ascii=False)}"
        )
    return filename_controls[0], save_controls[0], summary


def submit_save_dialog(
    dialog: Any,
    filename_control: Any,
    save_control: Any,
    target: Path,
    deadline: float,
    diagnostics: dict[str, Any],
    *,
    monotonic: Any = time.monotonic,
    sleep: Any = time.sleep,
    button_click_message: int = BUTTON_CLICK_MESSAGE,
) -> None:
    target_text = str(target)
    diagnostics.update(
        {
            "filenameBefore": filename_control.window_text(),
            "targetText": target_text,
            "setAttempts": [],
            "filenameVerified": False,
            "saveButtonVisible": bool(save_control.is_visible()),
            "saveButtonEnabled": bool(save_control.is_enabled()),
            "submissionMethod": None,
        }
    )
    if not diagnostics["saveButtonVisible"] or not diagnostics["saveButtonEnabled"]:
        raise GateFailure("The bound Save button is not visible and enabled.")
    dialog.set_focus()
    dialog.wait_for_idle()
    for attempt_number in range(1, FILENAME_SET_ATTEMPTS + 1):
        if monotonic() >= deadline:
            break
        attempt: dict[str, Any] = {"attempt": attempt_number, "settledReads": []}
        diagnostics["setAttempts"].append(attempt)
        filename_control.set_keyboard_focus()
        filename_control.set_edit_text(target_text)
        dialog.wait_for_idle()
        stable = True
        for _ in range(FILENAME_STABLE_READS):
            remaining = deadline - monotonic()
            if remaining <= 0:
                stable = False
                break
            sleep(min(FILENAME_SETTLE_SECONDS, remaining))
            observed = filename_control.window_text()
            attempt["settledReads"].append(observed)
            if observed != target_text:
                stable = False
        if stable and len(attempt["settledReads"]) == FILENAME_STABLE_READS:
            diagnostics["filenameAfterSet"] = target_text
            diagnostics["filenameVerified"] = True
            break
    if not diagnostics["filenameVerified"]:
        last_text = filename_control.window_text()
        diagnostics["filenameAfterSet"] = last_text
        raise GateFailure(
            "The Save dialog filename did not remain equal to the exact ZIP target "
            f"after {len(diagnostics['setAttempts'])} attempts; last: {last_text!r}."
        )
    save_control.send_message(button_click_message, 0, 0)
    diagnostics["submissionMethod"] = "BM_CLICK"


def exact_json_object(value: Any, keys: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise GateFailure(f"{label} fields drifted: {sorted(value) if isinstance(value, dict) else type(value).__name__}")
    return value


def verify_bundle(target: Path) -> dict[str, Any]:
    archive_bytes = target.read_bytes()
    if not (0 < len(archive_bytes) <= MAX_ARCHIVE_BYTES):
        raise GateFailure(f"The diagnostic archive size is outside bounds: {len(archive_bytes)}")
    with zipfile.ZipFile(target, "r") as archive:
        bad_member = archive.testzip()
        if bad_member:
            raise GateFailure(f"The diagnostic ZIP contains a corrupt member: {bad_member}")
        infos = archive.infolist()
        names = tuple(info.filename for info in infos)
        if names != EXPECTED_ENTRIES:
            raise GateFailure(f"Diagnostic entry order/set drifted: {names}")
        if any(
            info.is_dir()
            or info.filename.startswith(("/", "\\"))
            or "\\" in info.filename
            or ".." in Path(info.filename).parts
            for info in infos
        ):
            raise GateFailure("A diagnostic entry is not a safe relative POSIX file.")
        if any(info.compress_type != zipfile.ZIP_STORED for info in infos):
            raise GateFailure("Every diagnostic ZIP entry must use stored compression.")
        if any(info.file_size > MAX_ENTRY_BYTES for info in infos):
            raise GateFailure("A diagnostic ZIP entry exceeds the 256 KiB limit.")
        contents = {info.filename: archive.read(info.filename) for info in infos}
    uncompressed_bytes = sum(len(value) for value in contents.values())
    if uncompressed_bytes > MAX_UNCOMPRESSED_BYTES:
        raise GateFailure("The diagnostic ZIP exceeds the 512 KiB uncompressed limit.")

    system = exact_json_object(
        json.loads(contents[EXPECTED_ENTRIES[0]]),
        {
            "schemaVersion",
            "quickplsVersion",
            "releaseChannel",
            "sourceRevision",
            "osFamily",
            "architecture",
            "desktopRuntime",
            "locale",
            "webview2Version",
            "userDataIncluded",
            "networkAccessed",
        },
        "metadata/system.json",
    )
    if system["schemaVersion"] != 1 or system["userDataIncluded"] is not False or system["networkAccessed"] is not False:
        raise GateFailure("System diagnostic privacy flags drifted.")
    event_lines = contents[EXPECTED_ENTRIES[1]].decode("utf-8").splitlines()
    if not event_lines:
        raise GateFailure("The diagnostic event log is empty.")
    events = [
        exact_json_object(
            json.loads(line), {"timestamp", "sequence", "severity", "code"}, "event row"
        )
        for line in event_lines
    ]
    if any(not isinstance(row["sequence"], int) or row["sequence"] < 1 for row in events):
        raise GateFailure("Diagnostic event sequences are invalid.")
    manifest = exact_json_object(
        json.loads(contents[EXPECTED_ENTRIES[2]]),
        {
            "schemaVersion",
            "policyVersion",
            "createdAt",
            "quickplsVersion",
            "entries",
            "redactionCounts",
            "redactionTotal",
            "archiveLimits",
            "localOnly",
            "networkAccessed",
        },
        "manifest.json",
    )
    if manifest["schemaVersion"] != 1 or manifest["policyVersion"] != "quickpls-diagnostics-v1":
        raise GateFailure("Diagnostic manifest identity drifted.")
    if manifest["localOnly"] is not True or manifest["networkAccessed"] is not False:
        raise GateFailure("Diagnostic manifest locality flags drifted.")
    limits = manifest["archiveLimits"]
    expected_limits = {
        "maximumEntries": 3,
        "maximumEntryBytes": MAX_ENTRY_BYTES,
        "maximumUncompressedBytes": MAX_UNCOMPRESSED_BYTES,
        "maximumArchiveBytes": MAX_ARCHIVE_BYTES,
        "compression": "stored",
    }
    if limits != expected_limits:
        raise GateFailure(f"Diagnostic manifest archive limits drifted: {limits}")
    descriptors = manifest["entries"]
    if not isinstance(descriptors, list) or [row.get("name") for row in descriptors] != list(PAYLOAD_ENTRIES):
        raise GateFailure(f"Diagnostic payload descriptors drifted: {descriptors}")
    for descriptor in descriptors:
        payload = contents[descriptor["name"]]
        if descriptor != {
            "name": descriptor["name"],
            "sha256": hashlib.sha256(payload).hexdigest(),
            "bytes": len(payload),
        }:
            raise GateFailure(f"Diagnostic payload descriptor mismatch: {descriptor}")
    counts = manifest["redactionCounts"]
    expected_count_keys = {
        "windowsPaths",
        "emailAddresses",
        "urlQueriesOrFragments",
        "bearerTokens",
    }
    if not isinstance(counts, dict) or set(counts) != expected_count_keys or any(
        not isinstance(value, int) or value < 0 for value in counts.values()
    ):
        raise GateFailure(f"Diagnostic redaction counts drifted: {counts}")
    if manifest["redactionTotal"] != sum(counts.values()):
        raise GateFailure("Diagnostic redaction total does not match its categories.")

    decoded = b"\n".join(contents.values()).decode("utf-8")
    forbidden_patterns = {
        "windows_drive_path": r"(?i)(?:^|[^a-z0-9])[a-z]:[\\/]",
        "email_address": r"(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}",
        "unredacted_bearer": r"(?i)\bbearer\s+(?!<redacted-token>)[a-z0-9._~+/=-]+",
        "url_query_or_fragment": r"(?i)https?://[^\s\"'<>?#]+[?#]",
    }
    forbidden_matches = [
        name for name, pattern in forbidden_patterns.items() if re.search(pattern, decoded)
    ]
    if forbidden_matches:
        raise GateFailure(f"Sensitive-pattern scan failed: {forbidden_matches}")
    return {
        "path": str(target),
        "size": len(archive_bytes),
        "sha256": hashlib.sha256(archive_bytes).hexdigest(),
        "entryNames": list(EXPECTED_ENTRIES),
        "entryCompression": ["stored"] * len(EXPECTED_ENTRIES),
        "entrySizes": {name: len(value) for name, value in contents.items()},
        "uncompressedBytes": uncompressed_bytes,
        "system": system,
        "events": events,
        "manifest": manifest,
        "forbiddenPatternMatches": forbidden_matches,
    }


def wait_for_bundle(target: Path, deadline: float) -> dict[str, Any]:
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        if target.is_file() and target.stat().st_size > 0:
            try:
                return verify_bundle(target)
            except (zipfile.BadZipFile, EOFError, OSError, json.JSONDecodeError) as error:
                last_error = error
        time.sleep(0.15)
    if last_error:
        raise GateFailure(f"The diagnostic ZIP never became readable: {last_error}")
    raise GateFailure(f"The packaged app did not create the selected ZIP: {target}")


def main() -> int:
    args = parse_args()
    phase = "target_validation"
    dialog_handle = 0
    diagnostics: dict[str, Any] = {}
    try:
        target, results_root = validate_target(args.target, args.results_root)
        if os.name != "nt":
            raise GateFailure("Native Save dialog automation requires Windows.")
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
        diagnostics["mainWindow"] = main_info
        emit(
            {
                "event": "ready",
                "passed": True,
                "phase": phase,
                "targetPath": str(target),
                "resultsRoot": str(results_root),
                "mainWindow": main_info,
            }
        )

        deadline = time.monotonic() + max(5.0, args.timeout_seconds)
        phase = "owned_dialog_binding"
        inspected_dialogs: list[dict[str, Any]] = []
        dialog = None
        while time.monotonic() < deadline:
            candidates, inspected_dialogs = owned_dialogs(
                int(main_info["pid"]),
                int(main_window.handle),
                Desktop,
                win32con,
                win32gui,
            )
            if len(candidates) > 1:
                raise GateFailure(
                    f"Expected one owned #32770 Save dialog; found {len(candidates)}: {inspected_dialogs}"
                )
            if len(candidates) == 1:
                dialog = candidates[0]
                break
            time.sleep(0.1)
        diagnostics["inspectedDialogs"] = inspected_dialogs
        if dialog is None:
            raise GateFailure(
                "No visible #32770 dialog owned by the bound QuickPLS window appeared."
            )
        dialog_handle = int(dialog.handle)
        diagnostics["dialog"] = next(
            item for item in inspected_dialogs if item["handle"] == dialog_handle
        )

        phase = "exact_control_binding"
        filename_control, save_control, controls = exact_dialog_controls(dialog)
        diagnostics["controls"] = controls
        if target.exists():
            raise GateFailure(
                f"The ZIP target appeared before Save and will not be overwritten: {target}"
            )
        phase = "save_dialog_submission"
        diagnostics["saveSubmission"] = {}
        submit_save_dialog(
            dialog,
            filename_control,
            save_control,
            target,
            deadline,
            diagnostics["saveSubmission"],
            button_click_message=win32con.BM_CLICK,
        )
        phase = "diagnostic_zip_creation_and_readback"
        bundle = wait_for_bundle(target, deadline)
        emit(
            {
                "event": "complete",
                "passed": True,
                "phase": phase,
                "targetPath": str(target),
                "mainWindow": main_info,
                "dialog": diagnostics["dialog"],
                "boundControls": {
                    "filenameEditControlId": 1001,
                    "saveButtonControlId": 1,
                },
                "saveSubmission": diagnostics["saveSubmission"],
                "bundle": bundle,
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
                "targetPath": args.target,
                "error": {"type": type(error).__name__, "message": str(error)},
                "diagnostics": diagnostics,
                "traceback": traceback.format_exc(),
            }
        )
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
