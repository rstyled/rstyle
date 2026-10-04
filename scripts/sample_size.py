#!/usr/bin/env python3
"""Sample size for estimating a proportion at 95% / 99% confidence (stdlib only).

n0 = z^2 p (1 - p) / e^2, finite-population corrected: n = n0 / (1 + (n0 - 1) / N).
p = 0.5 is the worst case (largest n); pass `--p` with an observed rate to size for a rare cell.

Caveat: submissions of one problem are correlated, so n submissions carry less information than n
independent draws (design effect). Treat n as a floor and check the seed-to-seed spread.

Usage: scripts/sample_size.py [--population 53822] [--p 0.5] [--margins 0.05,0.03,0.02,0.01]
"""
import argparse, math
from statistics import NormalDist

CONFIDENCES = (95, 99)


def z_score(confidence):
    """Two-sided critical value: 95 -> 1.960, 99 -> 2.576."""
    return NormalDist().inv_cdf(0.5 + confidence / 200)


def sample_size(population, confidence, margin, p=0.5):
    """Smallest n (rounded up) whose proportion estimate is within +-margin at `confidence` %."""
    n0 = z_score(confidence) ** 2 * p * (1 - p) / margin**2
    if population is None:
        return math.ceil(n0)
    return min(population, math.ceil(n0 / (1 + (n0 - 1) / population)))


def table(population, margins, p):
    lines = [f"population={population} p={p}", "margin  " + "  ".join(f"{c}%".rjust(7) for c in CONFIDENCES)]
    for m in margins:
        row = "  ".join(str(sample_size(population, c, m, p)).rjust(7) for c in CONFIDENCES)
        lines.append(f"{m:<7} {row}")
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--population", type=int, default=53822, help="eligible submissions (kept after filters)")
    ap.add_argument("--p", type=float, default=0.5)
    ap.add_argument("--margins", default="0.05,0.03,0.02,0.01")
    a = ap.parse_args()
    print(table(a.population, [float(m) for m in a.margins.split(",")], a.p))


if __name__ == "__main__":
    main()
