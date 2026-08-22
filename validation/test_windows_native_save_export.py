from __future__ import annotations

import unittest
from tempfile import TemporaryDirectory
from pathlib import Path
from unittest.mock import patch

from validation.windows_native_save_export import (
    BUTTON_CLICK_MESSAGE,
    GateFailure,
    submit_save_dialog,
    visible_quickpls_window,
)


class FakeClock:
    def __init__(self) -> None:
        self.now = 0.0

    def monotonic(self) -> float:
        return self.now

    def sleep(self, seconds: float) -> None:
        self.now += seconds


class FakeDialog:
    def __init__(self) -> None:
        self.focus_calls = 0
        self.wait_for_idle_calls = 0

    def set_focus(self) -> None:
        self.focus_calls += 1

    def wait_for_idle(self) -> None:
        self.wait_for_idle_calls += 1


class FakeFilenameControl:
    def __init__(self, reads_by_attempt: list[list[str]]) -> None:
        self.reads_by_attempt = reads_by_attempt
        self.set_calls: list[str] = []
        self.keyboard_focus_calls = 0
        self.read_index = 0

    def window_text(self) -> str:
        if not self.set_calls:
            return "quickpls-result-tables"
        attempt_index = len(self.set_calls) - 1
        scripted_reads = self.reads_by_attempt[attempt_index]
        index = min(self.read_index, len(scripted_reads) - 1)
        self.read_index += 1
        return scripted_reads[index]

    def set_keyboard_focus(self) -> None:
        self.keyboard_focus_calls += 1

    def set_edit_text(self, text: str) -> None:
        self.set_calls.append(text)
        self.read_index = 0


class FakeSaveControl:
    def __init__(self, *, visible: bool = True, enabled: bool = True) -> None:
        self.visible = visible
        self.enabled = enabled
        self.sent_messages: list[tuple[int, int, int]] = []

    def is_visible(self) -> bool:
        return self.visible

    def is_enabled(self) -> bool:
        return self.enabled

    def send_message(self, message: int, wparam: int, lparam: int) -> None:
        self.sent_messages.append((message, wparam, lparam))


class FakeMainWindow:
    def __init__(self, pid: int, title: str, handle: int = 100) -> None:
        self.element_info = type("ElementInfo", (), {"process_id": pid})()
        self._title = title
        self.handle = handle

    def window_text(self) -> str:
        return self._title


class FakeDesktop:
    windows_by_call: list[FakeMainWindow] = []

    def __init__(self, *, backend: str) -> None:
        if backend != "win32":
            raise AssertionError(f"Unexpected backend: {backend}")

    def windows(self, *, visible_only: bool) -> list[FakeMainWindow]:
        if not visible_only:
            raise AssertionError("Only visible windows may be inspected.")
        return self.windows_by_call


class VisibleQuickPlsWindowTests(unittest.TestCase):
    def test_exact_owner_executable_basename_replaces_legacy_name_filter(self) -> None:
        with TemporaryDirectory() as directory:
            portable = Path(directory) / "QuickPLS_2.55.0_unsigned-preview_x64_portable.exe"
            portable.write_bytes(b"portable")
            window = FakeMainWindow(4321, "QuickPLS")
            FakeDesktop.windows_by_call = [window]

            with patch(
                "validation.windows_native_save_export.executable_path",
                return_value=str(portable),
            ):
                selected, info = visible_quickpls_window(
                    "QuickPLS",
                    FakeDesktop,
                    object(),
                    object(),
                    object(),
                    expected_pid=4321,
                    expected_executable=str(portable),
                )

            self.assertIs(selected, window)
            self.assertEqual(info["executable"], str(portable))

    def test_legacy_calls_still_require_quickpls_desktop_basename(self) -> None:
        with TemporaryDirectory() as directory:
            versioned = Path(directory) / "QuickPLS_versioned_portable.exe"
            versioned.write_bytes(b"portable")
            FakeDesktop.windows_by_call = [FakeMainWindow(4321, "QuickPLS")]

            with patch(
                "validation.windows_native_save_export.executable_path",
                return_value=str(versioned),
            ), self.assertRaisesRegex(GateFailure, "quickpls-desktop.exe"):
                visible_quickpls_window(
                    "QuickPLS",
                    FakeDesktop,
                    object(),
                    object(),
                    object(),
                )


class SubmitSaveDialogTests(unittest.TestCase):
    target = Path(r"D:\QuickPLS\validation\results\native-export.xlsx")

    def invoke(
        self,
        filename: FakeFilenameControl,
        save: FakeSaveControl | None = None,
    ) -> tuple[FakeDialog, FakeSaveControl, dict[str, object]]:
        dialog = FakeDialog()
        save = save or FakeSaveControl()
        diagnostics: dict[str, object] = {}
        clock = FakeClock()
        submit_save_dialog(
            dialog,
            filename,
            save,
            self.target,
            deadline=5.0,
            diagnostics=diagnostics,
            monotonic=clock.monotonic,
            sleep=clock.sleep,
        )
        return dialog, save, diagnostics

    def test_clicks_only_after_two_exact_settled_reads(self) -> None:
        expected = str(self.target)
        filename = FakeFilenameControl([[expected, expected]])

        dialog, save, diagnostics = self.invoke(filename)

        self.assertEqual(dialog.focus_calls, 1)
        self.assertEqual(dialog.wait_for_idle_calls, 2)
        self.assertEqual(filename.keyboard_focus_calls, 1)
        self.assertEqual(filename.set_calls, [expected])
        self.assertEqual(save.sent_messages, [(BUTTON_CLICK_MESSAGE, 0, 0)])
        self.assertTrue(diagnostics["filenameVerified"])
        self.assertEqual(diagnostics["submissionMethod"], "BM_CLICK")

    def test_retries_when_the_dialog_restores_its_initial_filename(self) -> None:
        expected = str(self.target)
        filename = FakeFilenameControl([
            [expected, "quickpls-result-tables"],
            [expected, expected],
        ])

        _, save, diagnostics = self.invoke(filename)

        self.assertEqual(filename.set_calls, [expected, expected])
        self.assertEqual(save.sent_messages, [(BUTTON_CLICK_MESSAGE, 0, 0)])
        self.assertEqual(len(diagnostics["setAttempts"]), 2)

    def test_refuses_to_click_when_exact_filename_never_settles(self) -> None:
        expected = str(self.target)
        filename = FakeFilenameControl([
            [expected, "quickpls-result-tables"],
            [expected, "quickpls-result-tables"],
            [expected, "quickpls-result-tables"],
        ])
        save = FakeSaveControl()
        diagnostics: dict[str, object] = {}
        clock = FakeClock()

        with self.assertRaisesRegex(GateFailure, "did not remain equal"):
            submit_save_dialog(
                FakeDialog(),
                filename,
                save,
                self.target,
                deadline=5.0,
                diagnostics=diagnostics,
                monotonic=clock.monotonic,
                sleep=clock.sleep,
            )

        self.assertEqual(save.sent_messages, [])
        self.assertFalse(diagnostics["filenameVerified"])
        self.assertEqual(diagnostics["filenameAfterSet"], "quickpls-result-tables")

    def test_refuses_to_edit_or_click_an_unavailable_save_button(self) -> None:
        expected = str(self.target)
        filename = FakeFilenameControl([[expected, expected]])
        save = FakeSaveControl(enabled=False)
        diagnostics: dict[str, object] = {}
        clock = FakeClock()

        with self.assertRaisesRegex(GateFailure, "not visible and enabled"):
            submit_save_dialog(
                FakeDialog(),
                filename,
                save,
                self.target,
                deadline=5.0,
                diagnostics=diagnostics,
                monotonic=clock.monotonic,
                sleep=clock.sleep,
            )

        self.assertEqual(filename.set_calls, [])
        self.assertEqual(save.sent_messages, [])


if __name__ == "__main__":
    unittest.main()
