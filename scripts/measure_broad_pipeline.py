"""Run the matched pipeline harness on fanout, many-pair and larger-graph fixtures.

Usage: python3 scripts/measure_broad_pipeline.py before|after BIN_ROOT OUTPUT
Uses the same warmup, ordinary/diagnostic separation and exact parity assertions
as measure_library_resolution.py. All generated sources are dependency-free.
"""
from measure_library_resolution import main


def workloads():
    fanout = ["pub fn level0(x: i32) -> bool { x > 0 }"]
    for level in range(1, 8):
        calls = " && ".join([f"level{level - 1}(x)"] * 6)
        fanout.append(f"pub fn level{level}(x: i32) -> bool {{ {calls} }}")
    many = ["pub struct Data { pub value: i32 }",
            "pub fn leaf(x: &Data) -> i32 { x.value }"]
    many.extend(f"pub fn caller{index}(x: &Data) -> i32 {{ leaf(x) }}" for index in range(48))
    graphs = []
    for index in range(8):
        updates = "\n".join(f"x = x + {1 + (step + index) % 3};" for step in range(16))
        graphs.append(f"pub fn graph{index}(mut x: i32) -> i32 {{\n{updates}\nx\n}}")
    return [("shared-fanout", "hir-helpers-v1", "\n".join(fanout), 8),
            ("many-pairs", "hir-data-v2", "\n".join(many), 49),
            ("larger-graphs", "r0", "\n".join(graphs), 8)]


if __name__ == "__main__":
    main(workloads())
