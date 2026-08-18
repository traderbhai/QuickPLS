#!/usr/bin/env python3
"""Run the current moderated-mediation invalid-mapping gate."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "moderated_mediation_v1", "--gate", "boundary"]))
