#!/usr/bin/env python3
"""Verify a saved moderation archive and its member checksums."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "moderation_v1", "--gate", "persistence"]))
