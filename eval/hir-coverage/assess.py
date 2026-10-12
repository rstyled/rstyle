"""Compare frozen source selections; never inspect matching predictions."""
import argparse
import collections
import hashlib
import json
from pathlib import Path

from tree_sitter import Language, Parser
import tree_sitter_rust

PARSER = Parser(Language(tree_sitter_rust.language()))
SOURCE_BLOCKERS = {
    "call_expression": "rust.call-summaries",
    "macro_invocation": "rust.macros",
    "type_cast_expression": "rust.integer-casts",
    "match_expression": "rust.match",
    "while_expression": "rust.loops",
    "loop_expression": "rust.loops",
    "for_expression": "rust.for-iteration",
    "closure_expression": "rust.closures",
    "reference_expression": "rust.abstract-memory",
    "index_expression": "rust.abstract-memory",
    "await_expression": "rust.async",
    "unsafe_block": "rust.unsafe",
    "const_block": "rust.const-blocks",
}


def source_blockers(source):
    data = source.encode()
    found = []

    def visit(node, outer=True):
        capability = SOURCE_BLOCKERS.get(node.type)
        if capability:
            found.append({"capability": capability, "syntax": node.type,
                          "start": node.start_byte, "end": node.end_byte,
                          "source": data[node.start_byte:node.end_byte].decode()})
        if node.type == "function_item" and not outer:
            found.append({"capability": "rust.blocks", "syntax": "nested_item",
                          "start": node.start_byte, "end": node.end_byte})
            return
        if node.type == "closure_expression":
            return
        for child in node.children:
            visit(child, outer and node.type != "function_item")

    visit(PARSER.parse(data).root_node)
    return found


def assess(selection, reports):
    outcomes = []
    for selected in selection:
        report = reports[selected["package"]]
        records = [record for record in report.get("functions", [])
                   if record["facts"]["build"]["frontend_options"].get("source_file", "")
                   .endswith("/" + selected["path"])
                   and selected["start"] <= record["facts"]["fn_span"]["start"] < selected["end"]]
        outcome = "unknown"
        reasons = ["Selected source definition absent from captured HIR; conditional presence or unresolved capture"]
        if not report.get("capture_complete"):
            outcome, reasons = "failed", report.get("capture_failures", ["Capture unavailable"])
        elif len(records) == 1:
            outcome, reasons = records[0]["outcome"], records[0]["reasons"]
        elif len(records) > 1:
            outcome, reasons = "failed", ["Ambiguous selected-definition correspondence"]
        item = dict(selected)
        item.pop("source")
        item.update(outcome=outcome, reasons=reasons,
                    source_blockers=source_blockers(selected["source"]),
                    frontend_diagnostics=[d for record in records
                                          for d in record["facts"]["source_diagnostics"]],
                    source_scan_complete=bool(records) and all(
                        record["facts"]["source_scan_complete"] for record in records))
        outcomes.append(item)
    counts = collections.Counter(item["outcome"] for item in outcomes)
    combinations = collections.Counter(tuple(sorted(set(
        blocker["capability"] for blocker in item["source_blockers"]))) for item in outcomes)
    return {"functions": outcomes, "counts": dict(counts),
            "source_blocker_combinations": [{"capabilities": list(combo), "functions": n}
                                            for combo, n in sorted(combinations.items())],
            "claim": "Measured selected-source eligibility; source blockers are syntax inventory, not resolved admission or hypothetical coverage gain"}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("stage")
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    selection = json.loads(Path(__file__).with_name("selection.json").read_text())
    reports = {}
    identities = {}
    for package, version in sorted(set((f["package"], f["version"]) for f in selection)):
        path = args.root / f"{package}-{version}" / f"{args.stage}.stdout.json"
        data = path.read_bytes()
        reports[package] = json.loads(data) if data else {"capture_complete": False}
        identities[package] = hashlib.sha256(data).hexdigest()
    result = assess(selection, reports)
    result.update(stage=args.stage, report_sha256=identities)
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
