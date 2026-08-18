#!/usr/bin/env python3
"""CLI for fresh native qualification of established bounded methods."""

from __future__ import annotations

import argparse

from established_method_factory_common import METHODS, qualify_all


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("methods", nargs="*", choices=sorted(METHODS))
    args = parser.parse_args()
    outputs = qualify_all(args.methods or None)
    for output in outputs:
        print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
