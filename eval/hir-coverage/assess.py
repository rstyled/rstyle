"""Compare frozen source selections; never inspect matching predictions."""
import collections
import correspondence

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
    rosters = {package: correspondence.contexts(report) for package, report in reports.items()}
    if len({(item['package'], item['version'], item['path'], item['start'], item['end'])
            for item in selection}) != len(selection):
        raise ValueError('Duplicate frozen selected-definition identity')
    for selected in selection:
        report = reports[selected["package"]]
        decisions, records = correspondence.map_definition(selected, report, rosters[selected['package']])
        outcome, reasons = correspondence.aggregate(decisions)
        item = dict(selected)
        item.pop("source")
        item.update(outcome=outcome, reasons=reasons,
                    compilation_contexts=decisions,
                    source_blockers=source_blockers(selected["source"]),
                    frontend_diagnostics=[d for record in records
                                          for d in record["facts"]["source_diagnostics"]],
                    source_scan_complete=bool(records) and all(
                        record["facts"]["source_scan_complete"] for record in records))
        if len(records) == 1:
            item.update(function_key=records[0]["key"], captured_source_digest=records[0]["facts"]["source"]["digest"],
                        **correspondence.evidence_navigation(records[0]))
        outcomes.append(item)
    counts = collections.Counter(item["outcome"] for item in outcomes)
    combinations = collections.Counter(tuple(sorted(set(
        blocker["capability"] for blocker in item["source_blockers"]))) for item in outcomes)
    context_counts = collections.Counter(decision['outcome'] for item in outcomes
                                         for decision in item['compilation_contexts'])
    return {"assessment_version": 4, "functions": outcomes, "counts": dict(counts),
            "denominators": dict(all_frozen_selected=len(selection),
                                 selected_function_contexts=sum(context_counts.values()),
                                 active_function_contexts=sum(decision['active'] for item in outcomes
                                                              for decision in item['compilation_contexts']),
                                 captured_records=sum(decision['captured_records'] for item in outcomes
                                                      for decision in item['compilation_contexts']),
                                 assessed_function_contexts=sum(decision['captured_records'] > 0 for item in outcomes
                                                               for decision in item['compilation_contexts']),
                                 independently_expected_in_scope=None,
                                 proven_inactive_function_contexts=context_counts['inactive']),
            "context_counts": dict(context_counts),
            "source_blocker_combinations": [{"capabilities": list(combo), "functions": n}
                                            for combo, n in sorted(combinations.items())],
            "claim": "Measured selected-source eligibility; source blockers are syntax inventory, not resolved admission or hypothetical coverage gain"}
