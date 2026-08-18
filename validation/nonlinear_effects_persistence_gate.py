#!/usr/bin/env python3
"""Report the bounded nonlinear-effects archive qualification blocker."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "nonlinear_effects_v1", "--gate", "persistence"]))
