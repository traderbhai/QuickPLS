#!/usr/bin/env python3
"""Run the current schema-v3 moderated-mediation recovery matrix."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "moderated_mediation_v1", "--gate", "engine"]))
