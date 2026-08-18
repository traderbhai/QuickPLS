#!/usr/bin/env python3
"""Run the current schema-v3 nonlinear-effects recovery matrix."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "nonlinear_effects_v1", "--gate", "engine"]))
