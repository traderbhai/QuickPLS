#!/usr/bin/env python3
"""Run the current nonlinear-effects fail-closed boundary gate."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "nonlinear_effects_v1", "--gate", "boundary"]))
