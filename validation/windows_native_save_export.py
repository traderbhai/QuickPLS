"""Drive one owned Windows Save dialog and verify the resulting QuickPLS XLSX.

This helper is intentionally narrow: it never sends global keystrokes, never
selects a dialog by localized title, and refuses to continue unless one visible
quickpls-desktop.exe window and one dialog owned by that window are found.
It emits JSON Lines so the Node acceptance harness can wait for readiness before
opening the modal dialog and retain structured failure evidence.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import time
import traceback
import zipfile
from pathlib import Path
from typing import Any
from xml.etree import ElementTree


EXPECTED_EXECUTABLE = "quickpls-desktop.exe"
DEFAULT_EXPECTED_SHEETS = ("Aggregate mediation effects boo",)
DEFAULT_EXPECTED_SHARED_STRINGS = (
    "Aggregate mediation effects bootstrap inference",
    "Total indirect effect (aggregate)",
    "Run provenance",
)
REQUIRED_XLSX_PARTS = (
    "[Content_Types].xml",
    "xl/workbook.xml",
    "xl/sharedStrings.xml",
)
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
    parser.add_argument(
        "--expected-sheet",
        action="append",
        dest="expected_sheets",
        help="Required worksheet name. Repeat for every worksheet that must be present.",
    )
    parser.add_argument(
        "--expected-shared-string",
        action="append",
        dest="expected_shared_strings",
        help="Required exact shared-string value. Repeat for every value that must be present.",
    )
    return parser.parse_args()


def resolve_expectations(args: argparse.Namespace) -> tuple[tuple[str, ...], tuple[str, ...]]:
    def normalize(
        supplied: list[str] | None,
        defaults: tuple[str, ...],
        option: str,
    ) -> tuple[str, ...]:
        if supplied is None:
            return defaults
        values = tuple(value.strip() for value in supplied)
        if not values or any(not value for value in values):
            raise GateFailure(f"{option} values must be non-empty.")
        if len(set(values)) != len(values):
            raise GateFailure(f"{option} values must be unique: {values}")
        return values

    return (
        normalize(args.expected_sheets, DEFAULT_EXPECTED_SHEETS, "--expected-sheet"),
        normalize(
            args.expected_shared_strings,
            DEFAULT_EXPECTED_SHARED_STRINGS,
            "--expected-shared-string",
        ),
    )


def validate_target(target_arg: str, results_root_arg: str) -> tuple[Path, Path]:
    supplied = Path(target_arg)
    if not supplied.is_absolute():
        raise GateFailure("The XLSX target must be absolute.")
    target = supplied.resolve(strict=False)
    results_root = Path(results_root_arg).resolve(strict=True)
    try:
        target.relative_to(results_root)
    except ValueError as error:
        raise GateFailure("The XLSX target must resolve inside validation/results.") from error
    if target == results_root or target.suffix.lower() != ".xlsx":
        raise GateFailure("The XLSX target must name an .xlsx file under validation/results.")
    if target.exists():
        raise GateFailure(f"The XLSX target already exists: {target}")
    if not target.parent.is_dir():
        raise GateFailure(f"The XLSX target parent does not exist: {target.parent}")
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
    expected_pid: int | None = None,
    expected_executable: str | None = None,
) -> tuple[Any, dict[str, Any]]:
    expected_executable_path = (
        Path(expected_executable).resolve(strict=True)
        if expected_executable is not None
        else None
    )
    expected_executable_name = (
        expected_executable_path.name.lower()
        if expected_executable_path is not None
        else EXPECTED_EXECUTABLE
    )
    candidates: list[tuple[Any, dict[str, Any]]] = []
    inspected: list[dict[str, Any]] = []
    for window in Desktop(backend="win32").windows(visible_only=True):
        pid = int(window.element_info.process_id)
        title = window.window_text()
        try:
            executable = executable_path(pid, win32api, win32con, win32process)
            executable_name = Path(executable).name.lower()
        except Exception as error:  # A foreign elevated process may reject inspection.
            executable = None
            executable_name = None
            if title.startswith("QuickPLS"):
                inspected.append({"pid": pid, "handle": int(window.handle), "title": title, "executableError": str(error)})
            continue
        if executable_name != expected_executable_name:
            continue
        info = {
            "pid": pid,
            "handle": int(window.handle),
            "title": title,
            "executable": executable,
        }
        inspected.append(info)
        if (
            title == expected_title
            and (expected_pid is None or pid == expected_pid)
            and (
                expected_executable_path is None
                or Path(executable).resolve(strict=True)
                == expected_executable_path
            )
        ):
            candidates.append((window, info))

    if len(candidates) != 1:
        raise GateFailure(
            f"Expected exactly one visible {expected_executable_name} main window with title "
            f"{expected_title!r}; found {len(candidates)}. "
            f"Inspected process windows: {json.dumps(inspected, ensure_ascii=False)}"
        )
    window, info = candidates[0]
    return window, info


def owned_dialogs(
    pid: int,
    main_handle: int,
    Desktop: Any,
    win32con: Any,
    win32gui: Any,
) -> tuple[list[Any], list[dict[str, Any]]]:
    owned: list[Any] = []
    inspected: list[dict[str, Any]] = []
    for window in Desktop(backend="win32").windows(class_name="#32770", visible_only=True):
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
            rows.append({
                "className": control.class_name(),
                "controlId": int(control.control_id()),
                "text": control.window_text()[:160],
            })
        except Exception as error:
            rows.append({"error": str(error)})
    return rows


def exact_dialog_controls(dialog: Any) -> tuple[Any, Any, list[dict[str, Any]]]:
    controls = dialog.descendants()
    filename_controls = [
        control for control in controls
        if control.class_name() == "Edit" and int(control.control_id()) == 1001
    ]
    save_controls = [
        control for control in controls
        if control.class_name() == "Button" and int(control.control_id()) == 1
    ]
    summary = control_summary(dialog)
    if len(filename_controls) != 1 or len(save_controls) != 1:
        raise GateFailure(
            "The owned Save dialog did not expose exactly one filename Edit control ID 1001 "
            f"and one Save Button control ID 1. Controls: {json.dumps(summary, ensure_ascii=False)}"
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
    settle_seconds: float = FILENAME_SETTLE_SECONDS,
    max_attempts: int = FILENAME_SET_ATTEMPTS,
    button_click_message: int = BUTTON_CLICK_MESSAGE,
) -> None:
    """Set and verify the filename before activating the bound Save button.

    ``EditWrapper.set_edit_text`` does not wait for the dialog thread to become
    idle.  The Windows common-file dialog can therefore restore its initial
    filename while it is still finishing setup.  Require two settled reads of
    the exact target and retry the edit before allowing the Save action.
    """

    target_text = str(target)
    diagnostics.update({
        "filenameBefore": filename_control.window_text(),
        "targetText": target_text,
        "setAttempts": [],
        "filenameVerified": False,
        "saveButtonVisible": bool(save_control.is_visible()),
        "saveButtonEnabled": bool(save_control.is_enabled()),
        "submissionMethod": None,
    })
    if not diagnostics["saveButtonVisible"] or not diagnostics["saveButtonEnabled"]:
        raise GateFailure("The bound Save button is not visible and enabled.")

    dialog.set_focus()
    dialog.wait_for_idle()
    for attempt_number in range(1, max_attempts + 1):
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
            sleep(min(settle_seconds, remaining))
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
            "The Save dialog filename did not remain equal to the exact XLSX target "
            f"after {len(diagnostics['setAttempts'])} attempts; last value: {last_text!r}."
        )

    # BM_CLICK is synchronous and remains scoped to the exact owned button.
    # pywinauto's ButtonWrapper.click() instead posts mouse messages, leaving a
    # second unobserved race immediately after the filename edit.
    save_control.send_message(button_click_message, 0, 0)
    diagnostics["submissionMethod"] = "BM_CLICK"


def wait_for_xlsx(
    target: Path,
    deadline: float,
    expected_sheets: tuple[str, ...],
    expected_shared_strings: tuple[str, ...],
) -> dict[str, Any]:
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        if target.is_file() and target.stat().st_size > 0:
            try:
                return verify_xlsx(target, expected_sheets, expected_shared_strings)
            except (zipfile.BadZipFile, EOFError, OSError) as error:
                last_error = error
        time.sleep(0.15)
    if last_error:
        raise GateFailure(f"The exported file never became a readable XLSX: {last_error}")
    raise GateFailure(f"The packaged app did not create the selected XLSX target: {target}")


def verify_xlsx(
    target: Path,
    expected_sheets: tuple[str, ...] = DEFAULT_EXPECTED_SHEETS,
    expected_shared_strings: tuple[str, ...] = DEFAULT_EXPECTED_SHARED_STRINGS,
) -> dict[str, Any]:
    with zipfile.ZipFile(target, "r") as workbook:
        bad_member = workbook.testzip()
        if bad_member:
            raise GateFailure(f"The XLSX ZIP contains a corrupt member: {bad_member}")
        members = set(workbook.namelist())
        missing_parts = [part for part in REQUIRED_XLSX_PARTS if part not in members]
        if missing_parts:
            raise GateFailure(f"The XLSX is missing required package parts: {missing_parts}")

        workbook_xml = ElementTree.fromstring(workbook.read("xl/workbook.xml"))
        sheets = [
            element.attrib.get("name", "")
            for element in workbook_xml.iter()
            if element.tag.rsplit("}", 1)[-1] == "sheet"
        ]
        missing_sheets = [value for value in expected_sheets if value not in sheets]
        if missing_sheets:
            if expected_sheets == DEFAULT_EXPECTED_SHEETS:
                raise GateFailure(
                    f"The XLSX does not contain the expected mediation sheet "
                    f"{DEFAULT_EXPECTED_SHEETS[0]!r}: {sheets}"
                )
            raise GateFailure(f"The XLSX is missing required QuickPLS worksheets: {missing_sheets}; found: {sheets}")

        shared_xml = ElementTree.fromstring(workbook.read("xl/sharedStrings.xml"))
        shared_strings = [
            "".join(element.itertext())
            for element in shared_xml.iter()
            if element.tag.rsplit("}", 1)[-1] == "si"
        ]
        missing_strings = [value for value in expected_shared_strings if value not in shared_strings]
        if missing_strings:
            raise GateFailure(f"The XLSX shared strings are missing required QuickPLS content: {missing_strings}")

    result = {
        "path": str(target),
        "size": target.stat().st_size,
        "sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
        "sheetNames": sheets,
        "requiredSharedStrings": list(expected_shared_strings),
    }
    if len(expected_sheets) == 1:
        result["requiredSheet"] = expected_sheets[0]
    else:
        result["requiredSheets"] = list(expected_sheets)
    return result


def main() -> int:
    args = parse_args()
    phase = "target_validation"
    dialog_handle = 0
    diagnostics: dict[str, Any] = {}
    try:
        expected_sheets, expected_shared_strings = resolve_expectations(args)
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
            args.window_title,
            Desktop,
            win32api,
            win32con,
            win32process,
        )
        diagnostics["mainWindow"] = main_info
        emit({
            "event": "ready",
            "passed": True,
            "phase": phase,
            "targetPath": str(target),
            "resultsRoot": str(results_root),
            "mainWindow": main_info,
        })

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
                raise GateFailure(f"Expected one owned #32770 Save dialog; found {len(candidates)}: {inspected_dialogs}")
            if len(candidates) == 1:
                dialog = candidates[0]
                break
            time.sleep(0.1)
        diagnostics["inspectedDialogs"] = inspected_dialogs
        if dialog is None:
            raise GateFailure("No visible #32770 dialog owned by the bound QuickPLS window appeared before timeout.")
        dialog_handle = int(dialog.handle)
        diagnostics["dialog"] = next(item for item in inspected_dialogs if item["handle"] == dialog_handle)

        phase = "exact_control_binding"
        filename_control, save_control, controls = exact_dialog_controls(dialog)
        diagnostics["controls"] = controls
        if target.exists():
            raise GateFailure(f"The XLSX target appeared before the Save action and will not be overwritten: {target}")

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

        phase = "xlsx_creation_and_readback"
        try:
            workbook = wait_for_xlsx(target, deadline, expected_sheets, expected_shared_strings)
        except Exception:
            try:
                diagnostics["saveSubmission"]["filenameAfterWait"] = filename_control.window_text()
                diagnostics["saveSubmission"]["dialogVisibleAfterWait"] = bool(dialog.is_visible())
            except Exception as state_error:
                diagnostics["saveSubmission"]["postSubmissionStateError"] = str(state_error)
            raise
        emit({
            "event": "complete",
            "passed": True,
            "phase": phase,
            "targetPath": str(target),
            "mainWindow": main_info,
            "dialog": diagnostics["dialog"],
            "boundControls": {"filenameEditControlId": 1001, "saveButtonControlId": 1},
            "saveSubmission": diagnostics["saveSubmission"],
            "workbook": workbook,
        })
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
        emit({
            "event": "complete",
            "passed": False,
            "phase": phase,
            "targetPath": args.target,
            "error": {"type": type(error).__name__, "message": str(error)},
            "diagnostics": diagnostics,
            "traceback": traceback.format_exc(),
        })
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
