#!/usr/bin/env python3
"""Run the current moderation invalid-interaction gate."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "moderation_v1", "--gate", "boundary"]))
