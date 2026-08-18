#!/usr/bin/env python3
"""Verify a saved mediation archive and its member checksums."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "mediation_v1", "--gate", "persistence"]))
