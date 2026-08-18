#!/usr/bin/env python3
try:
    from validation.phase2_release_packaged_common import main_for
except ModuleNotFoundError:
    from phase2_release_packaged_common import main_for


if __name__ == "__main__":
    raise SystemExit(main_for("ipma_v1"))
