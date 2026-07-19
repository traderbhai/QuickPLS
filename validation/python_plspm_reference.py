"""Development-only GPL reference runner. Never package this environment with QuickPLS."""
import json
import sys
from pathlib import Path

import pandas as pd
from plspm.config import Config, MV, Structure
from plspm.mode import Mode
from plspm.plspm import Plspm
from plspm.scheme import Scheme

ROOT = Path(__file__).resolve().parent


def run_variant(label, mode, scheme):
    structure = Structure()
    structure.add_path(["x"], ["y"])
    config = Config(structure.path(), scaled=False)
    config.add_lv("x", mode, MV("x1"), MV("x2"))
    config.add_lv("y", mode, MV("y1"), MV("y2"))
    model = Plspm(data, config, scheme, iterations=3000, tolerance=1e-12)
    paths = model.path_coefficients()
    outer = model.outer_model()
    return {
        "variant": label,
        "mode": mode.name,
        "scheme": scheme.name,
        "paths": [{"source": "x", "target": "y", "value": float(paths.loc["y", "x"])}],
        "outer": [
            {
                "construct": "x" if str(index) in ["x1", "x2"] else "y",
                "indicator": str(index),
                "loading": float(row["loading"]),
                "weight": float(row["weight"]),
            }
            for index, row in outer.iterrows()
        ],
    }


data = pd.read_csv(ROOT / "fixtures" / "simple_reflective.csv")
data = (data - data.mean()) / data.std(ddof=1)
report = {
    "engine": "python-plspm-0.5.7",
    "input": "validation/fixtures/simple_reflective.csv",
    "weight_note": "plspm outer weights use a different normalization than QuickPLS/cSEM; compare path coefficients and loadings.",
    "variants": [
        run_variant("PATH_MODE_A", Mode.A, Scheme.PATH),
        run_variant("MODE_B", Mode.B, Scheme.PATH),
        run_variant("FACTOR", Mode.A, Scheme.FACTORIAL),
    ],
}

encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
if len(sys.argv) == 2:
    Path(sys.argv[1]).write_text(encoded, encoding="utf-8")
else:
    print(encoded, end="")
