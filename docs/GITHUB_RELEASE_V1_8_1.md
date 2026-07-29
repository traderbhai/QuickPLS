# GitHub Release v1.8.1

Tag: `v1.8.1`

Release title:

```text
QuickPLS v1.8.1
```

Assets to upload:

```text
D:\QuickPLS\target\release\artifacts\QuickPLS_1.8.1_v1_8_1_method_applicability_guided_setup_20260729-140437_x64_setup.exe
D:\QuickPLS\target\release\artifacts\QuickPLS_1.8.1_v1_8_1_method_applicability_guided_setup_20260729-140437_x64_portable.exe
D:\QuickPLS\target\release\artifacts\QuickPLS_1.8.1_v1_8_1_method_applicability_guided_setup_20260729-140437_x64_checksums.txt
```

If using GitHub CLI:

```powershell
gh auth login
gh release create v1.8.1 `
  "D:\QuickPLS\target\release\artifacts\QuickPLS_1.8.1_v1_8_1_method_applicability_guided_setup_20260729-140437_x64_setup.exe" `
  "D:\QuickPLS\target\release\artifacts\QuickPLS_1.8.1_v1_8_1_method_applicability_guided_setup_20260729-140437_x64_portable.exe" `
  "D:\QuickPLS\target\release\artifacts\QuickPLS_1.8.1_v1_8_1_method_applicability_guided_setup_20260729-140437_x64_checksums.txt" `
  --title "QuickPLS v1.8.1" `
  --notes-file "D:\QuickPLS\docs\RELEASE_NOTES_V1_8_1.md"
```

The release should state that the installer is unsigned and that QuickPLS is proprietary source-available software.
