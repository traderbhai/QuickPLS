#!/usr/bin/env python3
"""Run the current two-stage HOC reference and metamorphic matrix."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "higher_order_v1", "--gate", "engine"]))
