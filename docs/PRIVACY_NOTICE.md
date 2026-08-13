# QuickPLS Product Privacy Notice

Status: product notice drafted; qualified legal review is still required before the
QuickPLS 3 stable release.

## Local Processing

QuickPLS is designed to process datasets, models, projects, calculations, and
exports on the user's Windows device. It does not require an account, activation
server, cloud computation, or cloud storage. Product telemetry is disabled by
default, and the application must not make a network request merely because it was
launched.

QuickPLS project files and exports remain in locations chosen by the user. Users
control their own backup, retention, deletion, and institutional handling of those
files.

## Optional Network Activity

An update check may contact the configured update host only after the user requests
it or explicitly enables the documented setting. The request should contain only
the information needed to select and verify an update. The host or its infrastructure
provider may receive ordinary connection metadata such as IP address, request time,
user agent, requested path, and response status.

QuickPLS must disclose any future network feature before enabling it. No diagnostics,
dataset, project, model, result, or full local path may be uploaded as part of an
update check.

## Distribution And Website Logs

Official downloads, issues, and security advisories currently use GitHub. GitHub may
process account information, connection logs, download activity, issue content, and
attachments under its own privacy terms. QuickPLS does not control GitHub's service
logs or retention.

## Support Submissions

Public issues are visible to others. Submit only the minimum information required to
reproduce a problem, preferably using synthetic or anonymized data. Do not submit
credentials, confidential datasets, personal data, unpublished identifiable
research, or proprietary project material in a public issue.

Security reports use a private GitHub security advisory. Information voluntarily
submitted for support is used to reproduce, triage, communicate about, and resolve
the report. Public disclosure must remove private reporter and dataset information.

## Diagnostics

Diagnostic bundle creation is user-initiated and local. The default bundle is
designed to include product/build identity, operating environment, bounded event
logs, error codes, and redaction statistics. It excludes dataset values, project
contents, credentials, signing material, and full personal paths. The user must be
able to preview the manifest and cancel before saving or sharing the bundle.

The runtime diagnostic-bundle feature is not yet release-qualified; until it is,
users should provide manual privacy-safe reproduction details.

## User Choices

Users can operate QuickPLS offline, decline update checks, decline diagnostic
creation, inspect a diagnostic summary before saving, delete local application data
through documented Windows procedures, and choose whether to submit a support
report. Uninstall must preserve user projects unless the user separately deletes
them.

## Changes And Contact

Material changes to local processing, telemetry, update behavior, support handling,
or third-party services require an updated version of this notice and release-note
disclosure. Product/privacy questions use the public support route when they contain
no sensitive information; suspected security or privacy vulnerabilities use the
private security advisory route in [`SECURITY.md`](../SECURITY.md).

This notice describes intended product behavior. Packaged network-behavior testing
and qualified legal review remain mandatory before stable release.
