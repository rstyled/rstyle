"""Validators for current compiler report evaluation and independent labels."""
import json
from pathlib import Path

MAX_BYTES = 64 * 1024 * 1024

KINDS = frozenset(("mutation", "io", "network", "global", "unsafe", "time", "random", "panic", "unknown"))
PATH_KINDS = frozenset(("io", "network", "time", "random", "panic"))


def named(name, expected):
    return name == expected or name.endswith("::" + expected)


def diagnostic_assessment(report, metadata, name, rule):
    roster = [entry for entry in metadata.get("functions", []) if named(entry["name"], name)]
    if len(roster) != 1:
        return None, "recorded function roster missing or ambiguous"
    entry = roster[0]
    diagnostics = report.get("diagnostics", report.get("violations", []))
    if not isinstance(diagnostics, list):
        return None, "recorded diagnostic list unavailable"
    hits = [diagnostic for diagnostic in diagnostics
            if diagnostic.get("rule_id") == rule
            and diagnostic.get("path") == entry["source_file"]
            and diagnostic.get("line") == entry["line"]]
    if len(hits) > 1:
        return None, "recorded diagnostic duplicated across contexts"
    excluded = entry.get("excluded", False)
    if not isinstance(excluded, bool):
        return None, "recorded exclusion declaration malformed"
    if excluded:
        if metadata.get("complete") is not True:
            return None, "recorded exclusion completeness unavailable"
        if hits:
            return None, "recorded diagnostic contradicts declared exclusion"
        return {"outcome": "not_evaluated", "excluded": True}, None
    if hits:
        return {"outcome": "finding", "actual": hits[0].get("actual"), "limit": hits[0].get("limit")}, None
    if metadata.get("complete") is not True:
        return None, "recorded assessment completeness unavailable"
    return {"outcome": "evaluated_no_finding"}, None


def nonnegative_integer(value):
    return type(value) is int and value >= 0


def policy_error(policy, *, depths=(2,)):
    if not isinstance(policy, dict) or set(policy) != {"max_density_milli", "min_statements", "min_lines", "transitive_depth", "weights", "paths", "exclude"}:
        return "effective density policy missing or malformed"
    if any(not nonnegative_integer(policy[key]) for key in ("max_density_milli", "min_statements", "min_lines", "transitive_depth")) or policy["transitive_depth"] not in depths:
        return "density numeric policy malformed or unsupported"
    weights, paths = policy["weights"], policy["paths"]
    if not isinstance(weights, dict) or set(weights) != KINDS or any(not nonnegative_integer(value) for value in weights.values()):
        return "complete density weights missing or malformed"
    if not isinstance(paths, dict) or set(paths) != PATH_KINDS or any(not string_list(value) for value in paths.values()) or not string_list(policy["exclude"]):
        return "complete density paths or exclusions missing or malformed"
    return None


def string_list(value):
    return isinstance(value, list) and all(isinstance(item, str) and item for item in value)


def counts_valid(counts):
    return isinstance(counts, dict) and set(counts) <= KINDS and all(nonnegative_integer(count) and count > 0 for count in counts.values())


def label_error(case, policy, *, known_unknown_events=False):
    expected = case.get("expected", {})
    if not isinstance(expected, dict) or not counts_valid(expected.get("events")):
        return "independent event counts missing or malformed"
    if any(type(expected.get(key, False)) is not bool for key in ("complete", "exempt")) or "complete" not in expected:
        return "independent completeness or exemption malformed"
    if any(type(case.get(key, False)) is not bool for key in ("suppressed", "excluded")):
        return "independent source policy label malformed"
    if any(not nonnegative_integer(expected.get(key)) for key in ("statements", "code_lines")):
        return "independent source denominator malformed"
    if not expected["complete"]:
        if expected.get("actual") is not None or expected.get("outcome") != "not_evaluated" or expected.get("exempt", False):
            return "incomplete label cannot authorize numeric or clean results"
        return None
    if expected["events"].get("unknown", 0) and not known_unknown_events:
        return "unknown events cannot authorize complete labels"
    exempt = expected["statements"] < policy["min_statements"] or expected["code_lines"] < policy["min_lines"]
    actual = None if exempt else sum(count * policy["weights"][kind] for kind, count in expected["events"].items()) * 1000 // max(1, expected["statements"])
    outcome = "finding" if actual is not None and actual > policy["max_density_milli"] else "evaluated_no_finding"
    if (expected.get("exempt", False) != exempt or expected.get("actual") != actual
            or (actual is not None and not nonnegative_integer(expected.get("actual"))) or expected.get("outcome") != outcome):
        return "independent label contradicts density arithmetic or policy"
    return None



def require(condition, message):
    if not condition:
        raise ValueError(message)


def integer(value, name):
    require(type(value) is int and value >= 0, 'invalid nonnegative integer: ' + name)
    return value


def boolean(value, name):
    require(type(value) is bool, 'invalid boolean: ' + name)
    return value


def unique(rows, field, name):
    require(isinstance(rows, list), 'missing list: ' + name)
    result = {}
    for row in rows:
        require(isinstance(row, dict), 'invalid row: ' + name)
        key = row.get(field)
        require(isinstance(key, (str, int)) and type(key) is not bool,
                'invalid key: ' + name)
        require(key not in result, 'duplicate key: ' + name)
        result[key] = row
    return result


def relative_path(value, root, label=False):
    require(isinstance(value, str) and value != '', 'source path unavailable')
    path = Path(value)
    require('..' not in path.parts and '.' not in value.split('/'), 'source path traversal')
    if label:
        require(not path.is_absolute(), 'label path must be workspace-relative')
    if path.is_absolute():
        try:
            path = path.relative_to(root)
        except ValueError as error:
            raise ValueError('source path outside workspace') from error
    return path.as_posix()


def load(path):
    with Path(path).open('rb') as stream:
        data = stream.read(MAX_BYTES + 1)
    require(len(data) <= MAX_BYTES, 'input exceeds 64 MiB bound')
    def object_pairs(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, 'duplicate JSON object key: ' + key)
            result[key] = value
        return result

    return json.loads(data, object_pairs_hook=object_pairs)
