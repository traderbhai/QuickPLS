#!/usr/bin/env python3
"""Verify a saved two-stage HOC archive and its member checksums."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "higher_order_v1", "--gate", "persistence"]))
