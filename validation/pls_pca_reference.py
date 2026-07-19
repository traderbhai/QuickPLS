"""Independent PCA reference for QuickPLS validation.

This script does not import QuickPLS. It computes PCA block scores from the
sample covariance eigensystem, applies the documented orientation and score
normalization conventions, then fits the one-path structural regression.
"""

import json
from pathlib import Path

import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parent
DATA = ROOT / "fixtures" / "simple_reflective.csv"
OUTPUT = ROOT / "results" / "pls_pca_numpy_reference.json"


def sample_standardize(frame):
    return (frame - frame.mean(axis=0)) / frame.std(axis=0, ddof=1)


def sample_cov(left, right):
    return float(np.cov(left, right, ddof=1)[0, 1])


def sample_cor(left, right):
    return float(np.corrcoef(left, right)[0, 1])


def sample_sd(values):
    return float(np.std(values, ddof=1))


def pca_block(columns):
    matrix = columns.to_numpy(dtype=float)
    covariance = np.cov(matrix, rowvar=False, ddof=1)
    values, vectors = np.linalg.eigh(covariance)
    vector = vectors[:, int(np.argmax(values))]
    if vector.sum() < 0:
        vector = -vector
    score = matrix @ vector
    reference = matrix @ np.ones(len(vector))
    association = sample_cov(score, reference)
    if association < -1e-15 or (abs(association) <= 1e-15 and vector.sum() < 0):
        vector = -vector
        score = -score
    vector = vector / sample_sd(score)
    score = matrix @ vector
    score = (score - score.mean()) / sample_sd(score)
    return vector, score


def main():
    data = sample_standardize(pd.read_csv(DATA))
    blocks = {"x": ["x1", "x2"], "y": ["y1", "y2"]}
    weights = {}
    scores = {}
    loadings = {}
    for construct, indicators in blocks.items():
        weight, score = pca_block(data[indicators])
        scores[construct] = score
        for indicator, value in zip(indicators, weight):
            weights[indicator] = float(value)
            loadings[indicator] = sample_cor(data[indicator], score)
    path = sample_cov(scores["x"], scores["y"]) / sample_cov(scores["x"], scores["x"])
    report = {
        "engine": "numpy-eigh",
        "input": "validation/fixtures/simple_reflective.csv",
        "variant": "PCA",
        "paths": [{"source": "x", "target": "y", "value": float(path)}],
        "outer": [
            {
                "construct": "x" if indicator in ["x1", "x2"] else "y",
                "indicator": indicator,
                "loading": loadings[indicator],
                "weight": weights[indicator],
            }
            for indicator in ["x1", "x2", "y1", "y2"]
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
