"""Build QuickPLS v0.9 release executable and unsigned NSIS installer.

Tauri produces the release executable. On this Windows workstation, Tauri's
bundler can fail after NSIS extraction with `Access is denied`; when that
happens and the release executable exists, this script creates a minimal
unsigned NSIS installer with the same project-local NSIS tool.
"""

import json
import os
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v09_build_release.json"
VERSION = "0.9.0-rc.1"


def npm():
    return "npm.cmd" if os.name == "nt" else "npm"


def run(command, timeout=1800):
    started = time.perf_counter()
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "elapsed_seconds": round(time.perf_counter() - started, 4),
        "stdout_tail": proc.stdout[-5000:],
        "stderr_tail": proc.stderr[-5000:],
    }


def write_fallback_nsi(script, output_installer, release_exe, release_dll):
    output_installer.parent.mkdir(parents=True, exist_ok=True)
    dll_lines = ""
    if release_dll.exists():
        dll_lines = f'  File "{release_dll}"\n'
    content = f"""
Unicode true
Name "QuickPLS"
OutFile "{output_installer}"
InstallDir "$LOCALAPPDATA\\QuickPLS"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!include MUI2.nsh
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "QuickPLS" SEC01
  SetOutPath "$INSTDIR"
  File "{release_exe}"
{dll_lines}  WriteUninstaller "$INSTDIR\\Uninstall QuickPLS.exe"
  CreateShortcut "$DESKTOP\\QuickPLS.lnk" "$INSTDIR\\quickpls-desktop.exe"
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\\QuickPLS.lnk"
  Delete "$INSTDIR\\quickpls-desktop.exe"
  Delete "$INSTDIR\\quickpls_desktop_lib.dll"
  Delete "$INSTDIR\\Uninstall QuickPLS.exe"
  RMDir "$INSTDIR"
SectionEnd
""".strip()
    script.write_text(content, encoding="utf-8")


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    release_exe = ROOT / "target" / "release" / "quickpls-desktop.exe"
    release_dll = ROOT / "target" / "release" / "quickpls_desktop_lib.dll"
    bundle_dir = ROOT / "target" / "release" / "bundle" / "nsis"
    installer = bundle_dir / f"QuickPLS_{VERSION}_x64-setup.exe"
    nsi = bundle_dir / "quickpls-v09-rc1-fallback.nsi"
    makensis = ROOT / "target" / ".tauri" / "nsis-3.11" / "Bin" / "makensis.exe"

    tauri = run([npm(), "run", "tauri", "--", "build"], timeout=1800)
    fallback_used = False
    fallback = None
    if not tauri["passed"]:
        if not release_exe.exists():
            report = {
                "schema_version": 1,
                "target": "QuickPLS v0.9.0-rc.1 release build",
                "passed": False,
                "tauri": tauri,
                "fallback_used": False,
                "reason": "Tauri build failed before producing release executable.",
            }
            OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
            print(json.dumps(report, indent=2))
            raise SystemExit(1)
        if not makensis.exists():
            report = {
                "schema_version": 1,
                "target": "QuickPLS v0.9.0-rc.1 release build",
                "passed": False,
                "tauri": tauri,
                "fallback_used": False,
                "reason": "Release executable exists but project-local makensis.exe was not found.",
            }
            OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
            print(json.dumps(report, indent=2))
            raise SystemExit(1)
        write_fallback_nsi(nsi, installer, release_exe, release_dll)
        fallback_used = True
        fallback = run([str(makensis), str(nsi)], timeout=600)

    passed = release_exe.exists() and (
        any(bundle_dir.glob("*.exe")) and (tauri["passed"] or (fallback and fallback["passed"]))
    )
    report = {
        "schema_version": 1,
        "target": "QuickPLS v0.9.0-rc.1 release build",
        "passed": passed,
        "version": VERSION,
        "tauri": tauri,
        "fallback_used": fallback_used,
        "fallback": fallback,
        "release_executable": str(release_exe.relative_to(ROOT)),
        "nsis_installers": [str(path.relative_to(ROOT)) for path in bundle_dir.glob("*.exe")],
        "note": "Fallback is used only when Tauri's NSIS wrapper fails after producing the release executable. The installer remains unsigned.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
