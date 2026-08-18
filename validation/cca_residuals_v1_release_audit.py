#!/usr/bin/env python3
try:
    from validation.phase2_release_packaged_common import audit_main_for
except ModuleNotFoundError:
    from phase2_release_packaged_common import audit_main_for


if __name__ == "__main__":
    raise SystemExit(audit_main_for("cca_residuals_v1"))
