# QuickPLS Diagnostics And Logging Contract

## Status

The first runtime slice is implemented for the native desktop: the user must preview
redacted staging content before choosing a destination, cancellation is explicit,
and saving creates a new local ZIP without any upload or network call. The current
bundle contains only self-generated build/system metadata and bounded structured
session event codes.

Persistent rolling log files, crash capture, richer runtime/version probes, packaged
Windows acceptance, and support-review approval remain required. This document must
not be cited as proof that the commercial support gate is complete.

## User Control

- Logging is local and must not activate telemetry or remote submission.
- Diagnostic bundle creation is initiated by the user from the desktop application.
- The application shows included categories, excluded categories, and redaction
  counts before saving. It also renders the exact redacted system/build fields,
  redacted event rows, and manifest payload descriptors staged for the archive.
- Saving uses a native destination dialog as a user-directed path-selection step.
  The dialog is not an authorization boundary: the backend independently applies
  all path and new-file checks. Sending or attaching the saved bundle is a separate
  user action.
- The user can cancel at preview or save without changing the project.

## Log Retention

The implemented slice keeps at most 128 fixed-shape events in memory for the current
desktop session. It does not write a background log file, so there is currently no
disk-retention or rotation surface. The event fields are limited to timestamp,
sequence, severity, and a QuickPLS-owned stable code; event APIs do not accept user
labels, paths, parser messages, datasets, or project content.

The later persistent-log implementation must retain at most ten log files of at most
1 MiB each and rotate before a write would exceed the limit. Logs older than 30 days
must be removed at normal application startup. A crash or failure to rotate must not
prevent the application from opening a project.

## Default Diagnostic Contents

The archive may include only:

- QuickPLS version, channel, source/build identity, and artifact checksum when known;
- Windows version, architecture, locale, WebView2 version, and relevant runtime versions;
- timestamps, severity, stable event/error codes, method identifier/version, bounded
  duration and count measurements, and job correlation identifiers;
- installer/update state that does not contain credentials or complete URLs with
  query strings;
- a manifest containing file hashes, byte sizes, redaction counts, and the policy
  version used to create the bundle.

## Mandatory Exclusions And Redaction

The default bundle must exclude dataset rows and values, project/archive contents,
model labels entered by the user, exported results, credentials, tokens, cookies,
signing keys, environment-variable values, process command lines, memory dumps,
registry exports, and arbitrary user files.

Before serialization:

- full Windows user-profile and selected-file paths become `<redacted-path>/<basename>`;
- email addresses, access tokens, authorization headers, query strings, and URL
  fragments are removed;
- free-form error messages from parsers or operating-system APIs are normalized to
  stable error codes plus redacted summaries;
- filenames are omitted unless needed for file-type diagnosis, in which case only a
  sanitized basename and extension are included;
- cells, variable names, construct labels, paths, and project titles are not included
  in event payloads.

Redaction occurs before data is written to the diagnostic ZIP. Preview must read the
redacted staging content, never the raw logs.

## Archive Safety

The implemented ZIP has exactly three relative POSIX entries:

- `metadata/system.json`;
- `logs/events.jsonl`;
- `manifest.json`.

Entries use stored (uncompressed) ZIP records to avoid compression-ratio ambiguity.
Each entry is limited to 256 KiB, the uncompressed total is limited to 512 KiB, and
the completed stored ZIP is limited to 520 KiB. The entry count is fixed at three.
The payload-entry descriptors in `manifest.json`
record SHA-256 and byte size; the save result records the SHA-256 and byte size of the
completed archive. The manifest also records schema/policy versions, creation time,
QuickPLS version, limits, redaction counts, `localOnly: true`, and
`networkAccessed: false`.

The backend accepts only a drive-letter-rooted fixed local Windows `.zip` path. It rejects
relative and drive-relative paths, UNC paths, removable and RAM drives, mapped network drives, verbatim/device
namespaces, reserved DOS device names, trailing-space/period components, colons or
alternate data streams after the drive prefix, current/parent-directory components,
symbolic-link/junction/reparse-point ancestors, and an existing destination. It
canonicalizes and opens the selected parent as a no-share directory guard, then
creates the file with create-new, open-reparse-point, and no-share semantics. Before
writing, QuickPLS verifies from that exact open file handle that the object is a
regular non-reparse file, its Windows-resolved final path is the selected canonical
path, the selected path still resolves to the same volume/file identity, and its
resolved drive is still fixed and local. A parent replacement can
therefore only fail creation or final-path verification; archive bytes are not sent
to the replacement target. If post-open verification, write, or synchronization
fails, an empty or partial new file may remain. QuickPLS reports the failure and never
reopens, overwrites, replaces, or path-deletes it. No project or dataset extension is
accepted as a destination. Drive type and final path are checked through Windows
filesystem APIs; packaged acceptance must still confirm uncommon Windows
filesystem-provider behavior.

QuickPLS builds the complete bounded ZIP in memory first. SHA-256 and byte size are
calculated over those exact bytes before destination creation. The same newly created
file handle receives `write_all` followed by `sync_all`; the implementation does not
reopen, reread, or replace the destination to determine its result.

The preview is a server-side redacted staging snapshot identified by an atomic,
single-use preview ID. Save removes the matching staging object under the state lock
before path validation or any write; a failed save therefore requires a fresh
preview. A refresh names the prior ID explicitly: the backend removes only that ID and inserts
a separately owned staging object under a new unique ID while holding the same lock.
Independently created previews never overwrite one another and the in-memory pending
set is bounded to four. Each pending preview expires after 15 minutes; expired entries
are pruned before create or consume. If four unexpired abandoned previews remain, a
new preview atomically evicts the oldest before inserting its own staging object, so
renderer loss cannot permanently exhaust the preview capacity. Explicit cancellation
or Save-dialog cancellation consumes the matching ID. Leaving the Settings workspace
also cancels its staged preview. A concurrent save, refresh, or cancellation cannot
reuse an expired, evicted, or already consumed snapshot or mutate another ID's
staging.

## Required Tests

Implemented unit/component coverage includes:

1. Bearer-token, email, URL query/fragment, and Windows-path redaction.
2. Redaction before ZIP serialization and absence of supplied secret-pattern values
   from decoded archive entries.
3. A fixed entry allowlist, count/size limits, payload SHA-256/size descriptors, and
   stored ZIP entries.
4. Local drive-rooted `.zip`, namespace/ADS/reparse, existing-file, create-new race,
   exclusive-handle/final-path identity, parent-swap rejection, regular-file checks,
   and exact in-memory archive-byte checks. Partial-write retention and error UX still
   need disk-fault injection coverage.
5. Preview-before-save, inspectable staged contents, atomic one-time preview IDs,
   concurrent save/refresh/cancel behavior, expiration, recovery after more than four
   abandoned previews, explicit preview cancellation, neutral Save-dialog
   cancellation, desktop-only controls, and accessible tables plus status/alert
   semantics.

Still required for release qualification:

1. Negative tests using the candidate build and a corpus of dataset values, variable
   names, construct labels, project titles, and raw parser messages.
2. Persistent rotation, age cleanup, disk-full, and concurrent-write tests after
   persistent logging exists.
3. Packaged no-network observation during launch, preview, cancellation, and save.
4. Packaged keyboard/screen-reader acceptance and destination edge cases, including
   Windows junction/reparse-point coverage.
5. Packaged Windows acceptance under standard and administrator accounts.

The `support.docs_diagnostics` gate remains pending until the exact target candidate
passes these tests and a reviewer approves the resulting evidence.
