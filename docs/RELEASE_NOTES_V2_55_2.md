# QuickPLS 2.55.2 Release Notes

QuickPLS 2.55.2 fixes the editing transition found in the bundled Corporate
Reputation sample. Choosing **Edit Model** from Results now returns the source
model to the editable SEM canvas instead of retaining the locked result-view
presentation mode.

All bundled samples now open in editable Model mode. Completed-result diagrams
remain intentionally read-only in Results, so calculated evidence cannot be
mistaken for an editable model definition.

This patch retains the full 344-case Corporate Reputation sample, its
eight-construct mixed formative/reflective model, 13 structural paths, stored
QuickPLS result, and 48 three-decimal SmartPLS reference checks introduced in
2.55.1.

The local application, installer, portable desktop executable, and CLI share
version 2.55.2. Local release files are unsigned; verify their SHA-256 hashes
before distribution or installation.
