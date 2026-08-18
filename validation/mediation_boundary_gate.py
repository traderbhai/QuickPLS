#!/usr/bin/env python3
"""Run the current mediation identity and fail-closed boundary gate."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "mediation_v1", "--gate", "boundary"]))
