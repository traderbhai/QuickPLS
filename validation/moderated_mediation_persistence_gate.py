#!/usr/bin/env python3
"""Report the bounded moderated-mediation archive qualification blocker."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "moderated_mediation_v1", "--gate", "persistence"]))
