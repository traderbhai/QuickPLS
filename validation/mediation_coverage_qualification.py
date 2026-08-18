#!/usr/bin/env python3
"""Run current mediation recovery, invariance, and degradation coverage."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "mediation_v1", "--gate", "engine"]))
