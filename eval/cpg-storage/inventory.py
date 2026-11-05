#!/usr/bin/env python3
"""Inventory a saved self-check and verify every document's publication seal.

Uses the system libxxhash for the producer's XXH3-64 transport checksum. Byte
breakdowns use compact UTF-8 JSON; actual file sizes are recorded separately.
Only one input document is loaded at a time.
"""

import argparse
from collections import Counter, defaultdict
import ctypes
import ctypes.util
import hashlib
import json
from pathlib import Path


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    root = args.run.resolve()
    lib = ctypes.CDLL(ctypes.util.find_library("xxhash"))
    lib.XXH3_64bits.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    lib.XXH3_64bits.restype = ctypes.c_uint64
    manifests = sorted(root.rglob("documents.manifest.json"))
    sealed = {}
    for path in manifests:
        manifest = json.loads(path.read_text())
        if manifest["schema"] != "rstyle.cpg-documents.manifest/1":
            raise RuntimeError(f"Unexpected document manifest: {path}")
        for entry in manifest["documents"]:
            if Path(entry["file"]).name != entry["file"]:
                raise RuntimeError("Manifest names a nested path")
            target = path.parent / entry["file"]
            if target in sealed:
                raise RuntimeError(f"Duplicate seal: {target}")
            sealed[target] = entry
    documents = sorted(root.rglob("*.cpg.json"))
    if not documents or set(documents) != set(sealed):
        raise RuntimeError("Document inventory disagrees with manifests")

    files = []
    groups = defaultdict(lambda: {"files": 0, "bytes": 0})
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        size = path.stat().st_size
        category = "cpg" if path in sealed else path.name
        groups[category]["files"] += 1
        groups[category]["bytes"] += size
        files.append({"path": str(path.relative_to(root)), "bytes": size})

    fields = Counter()
    function_fields = Counter()
    function_extensions = Counter()
    table_counts = Counter()
    table_unique = defaultdict(dict)
    table_bytes = Counter()
    units = defaultdict(lambda: {"documents": 0, "bytes": 0, "functions": 0, "nodes": 0})
    targets = {}
    for path in root.glob("unit-*.json"):
        ledger = json.loads(path.read_text())
        targets[Path(ledger["directory"]).name] = ledger["target"]["name"]
    inputs = []
    for index, path in enumerate(documents):
        raw = path.read_bytes()
        entry = sealed[path]
        checksum = f"xxh3-64:{lib.XXH3_64bits(raw, len(raw)):016x}"
        if len(raw) != entry["bytes"] or checksum != entry["checksum"]:
            raise RuntimeError(f"Document seal mismatch: {path}")
        digest = hashlib.sha256(raw).hexdigest()
        value = json.loads(raw)
        unit = value["unit"]
        key = str(path.parent.relative_to(root))
        summary = units[key]
        summary["target"] = targets.get(path.parent.name, path.parent.name)
        summary["documents"] += 1
        summary["bytes"] += len(raw)
        functions = unit.get("functions") or []
        summary["functions"] += len(functions)
        summary["nodes"] += sum(len(f.get("nodes", [])) for f in functions)
        for name, content in unit.items():
            fields[name] += len(compact(content))
        for function in functions:
            for name, content in function.items():
                function_fields[name] += len(compact(content))
            for name, content in (function.get("extensions") or {}).items():
                function_extensions[name] += len(compact(content))
        for name in ["files", "definitions", "types", "values", "imports", "namespaces", "instances"]:
            for record in unit.get(name, []):
                encoded = compact(record)
                table_counts[name] += 1
                table_bytes[name] += len(encoded)
                table_unique[name][hashlib.sha256(encoded).hexdigest()] = len(encoded)
        inputs.append({"path": str(path), "relative_path": str(path.relative_to(root)),
                       "bytes": len(raw), "sha256": digest, "seal_verified": True})
        if index % 100 == 0:
            print(f"Inventoried {index + 1}/{len(documents)}", flush=True)

    tables = {name: {"record_occurrences": count, "distinct_records": len(table_unique[name]),
                     "record_bytes": table_bytes[name], "distinct_record_bytes": sum(table_unique[name].values())}
              for name, count in table_counts.items()}
    result = {"run_directory": str(root), "documents": len(documents), "manifest_count": len(manifests),
              "all_document_seals_verified": True,
              "capture_bytes": sum(f["bytes"] for f in files), "cpg_bytes": groups["cpg"]["bytes"],
              "file_groups": dict(groups), "unit_field_compact_json_bytes": dict(fields),
              "function_field_compact_json_bytes": dict(function_fields),
              "function_extension_compact_json_bytes": dict(function_extensions),
              "table_records": tables, "units": dict(units), "inputs": inputs, "files": files,
              "breakdown_method": "Compact UTF-8 JSON value lengths per unit field. Table record bytes exclude array commas/brackets. Exact distinct canonical record contents are hashed across the capture; these counts do not imply semantic identity. Actual file sizes and verified seals are separate."}
    args.out.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps({k: result[k] for k in ["documents", "cpg_bytes", "capture_bytes", "manifest_count"]}))


if __name__ == "__main__":
    main()
