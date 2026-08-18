# QuickPLS 3 External Beta Protocol

This protocol turns external beta into a measurable, privacy-safe release gate.
It does not claim that a beta has run. The current machine contract is
`planned`, and QuickPLS remains ineligible for competitor-grade stable claims.

The executable pre-beta forms, pseudonymization and deletion procedure,
six-journey scoring rubric, candidate/lifecycle placeholder, and exit checklist
are documented in [`QUICKPLS_3_BETA_OPERATIONS_KIT.md`](QUICKPLS_3_BETA_OPERATIONS_KIT.md).
Passing that kit's dry run validates preparation only; it cannot create beta
evidence or make `beta_ready` true.

## Entry conditions

- Every publicly advertised beta capability has current method-factory evidence.
- The exact beta candidate is Authenticode-signed and timestamped.
- Clean install, upgrade/recovery, uninstall, and offline first launch pass.
- Support, private vulnerability reporting, privacy, known-issues, and rollback
  routes are operating.
- An independent scientific reviewer has either cleared material findings or
  narrowed the beta claims.

## Cohort and privacy

Recruit 15-25 pseudonymous participants across at least five independent
institutions or groups. Include at least five experienced SmartPLS users and at
least five researchers new to SEM. Do not store names, email addresses, or raw
participant datasets in the repository evidence. Store consent externally under
a stable record ID; use privacy-safe workflow descriptions and synthetic or
participant-approved derived evidence only.

The beta must include at least 30 real workflows. Every workflow records the
same six core journeys: import, model authoring, calculation, interpretation,
export, and save/reopen. Developer assistance is recorded per journey rather
than inferred after the fact.

## Defect and scientific disposition

All P0/P1 defects must be closed before exit. Any reproducible data loss or
archive corruption is an immediate no-go. Every material numerical discrepancy
must be fixed, explained within the bounded contract, resolved by narrowing the
claim, or remain explicitly release-blocking.

Beta acceptance never substitutes for a method's scientific promotion ladder.
It tests product usability and workflow integrity on the exact candidate.

## Final rerun and approval

After the last beta activity and fix, rerun the signed-candidate lifecycle gate.
The final candidate entry must contain the exact rehashed candidate-manifest
descriptor, not a `signed: true` flag or opaque report ID. The validator opens
that manifest, rehashes every artifact and signature report, reruns SignTool and
Windows leaf-certificate/PE identity inspection, and verifies the detached CMS
channel and protected-build attestations against the frozen signer.

The final lifecycle entry is the `{path, size, sha256}` descriptor of a strict
JSON report. That report binds the candidate-manifest hash, every candidate
artifact digest, every PE signature-report digest, and the signing identity. It
must record a disconnected Windows environment and successful clean install,
portable launch, reinstall, supported upgrade, interruption recovery, bad-
signature rejection, rollback, offline full-installer recovery, uninstall, and
archive save/reopen phases. Approval must occur after the exact rehashed report
and after all beta activity. A planned or running contract exits successfully
for monitoring but reports `beta_ready: false`; `--require-ready` fails until
every threshold and approval is genuine.

Commands:

```powershell
python validation/quickpls_beta_operations_kit.py
python -m unittest validation.test_quickpls_beta_operations_kit
python validation/quickpls_external_beta.py
python validation/quickpls_external_beta.py --require-ready
python -m unittest validation.test_quickpls_external_beta
```
