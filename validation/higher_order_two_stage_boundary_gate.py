#!/usr/bin/env python3
"""Run the current two-stage HOC invalid-declaration gate."""
from phase3_workflow_factory import main

raise SystemExit(main(["--method", "higher_order_v1", "--gate", "boundary"]))
