# QuickPLS customer-language contract

This contract separates product language from the internal scientific
qualification system. It is a Wave 0 foundation for the Standard, Labs,
Legacy, and Internal information architecture.

## Product surfaces

- **Standard** contains release-qualified capability cells that are explicitly
  approved for their documented QuickPLS scope. Coverage remains separately
  recorded as full or partial, and a partial cell requires a concrete nonempty
  scope statement. The UI states those data/model requirements and never
  exposes evidence-ladder terminology.
- **Experimental Labs** is disabled by default. Enabling it displays one
  warning per feature and session: “Experimental methods may change and should
  be independently checked before final reporting.” Each Labs method has one
  Experimental chip and a Method Details explanation.
- **Legacy** keeps historical archives readable but does not expose
  discontinued methods in normal Calculate.
- **Internal** includes validation reports, manifests, source hashes, developer
  diagnostics, and qualification workflows. Internal terminology is allowed
  here, but not in normal setup, results, reports, or customer exports.

## Approved vocabulary

| Internal wording | Customer wording |
|---|---|
| Validated scope | Supported setup or Requirements |
| Calculation scope | Analysis details |
| Native-qualified / release-qualified | Never displayed; use Supported or Experimental |
| Candidate / unqualified | Experimental, only in Labs |
| Promotion evidence pending | Never displayed |
| Packaged evidence | Never displayed |
| Method versions and source hashes | Run Details |
| Validation plan when it means cross-validation | Cross-validation design |
| Bounded native scope | State the actual requirement |
| Repeated historical method warnings | One project-level historical-run banner |

Scientifically relevant limitations are retained, but moved to the right
layer: requirements in Setup, interpretation warnings in Results, assumptions
and references in Method Details, and reproducibility metadata in Run Details
and the export provenance appendix.

## Automated inventory and gate

Run the deterministic migration inventory without blocking development:

```powershell
python validation/customer_language_contract.py
```

Run the product-finalization gate:

```powershell
python validation/customer_language_contract.py --strict
```

The scanner matches customer-facing governance phrases, including variants of
validated setup, bounded workflow, candidate output, and current or packaged
evidence. Ordinary code identifiers such as `candidate` and `evidence`, and
scientifically appropriate phrases such as “cross-validation design” or
“reliability evidence,” are not prohibited on their own. Tests, stories,
specs, generated output, and fixtures are excluded. The strict gate currently
passes with zero occurrences in production UI source; any reintroduction fails
the product foundation gate.

## Completion conditions

The language track is complete only when:

1. Strict scanning reports zero prohibited occurrences in production UI code.
2. Standard setup and results snapshots contain none of the internal terms.
3. Labs warning frequency is one per feature/session.
4. Historical results show one project-level banner rather than repeated table
   warnings.
5. Blocked configurations identify both the incompatible object and the
   corrective action.
6. Method Details and Run Details remain reachable from setup and results.
