#!/usr/bin/env python3
"""Generate shareable comparison reports from the retained trial measurements."""

import argparse
import csv
import hashlib
import html
import json
from pathlib import Path
import shutil

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np


MIB = 1024 * 1024
LABELS = {"json": "Shared JSON", "json-zstd": "Shared JSON + Zstd",
          "original-json": "Original JSON", "original-json-zstd": "Original JSON + Zstd",
          "flatbuffers": "FlatBuffers", "flexbuffers": "FlexBuffers", "protobuf": "Protobuf",
          "flatbuffers-zstd": "FlatBuffers + Zstd", "flexbuffers-zstd": "FlexBuffers + Zstd",
          "protobuf-zstd": "Protobuf + Zstd"}


def read(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def metric(row, operation, field="seconds", statistic="median"):
    return row[operation][field][statistic]


def ordered(data):
    rows = data["results"]
    return [label for label in ["original-json", "original-json-zstd", "json", "json-zstd", "flatbuffers", "flexbuffers", "protobuf",
                               data["fastest_binary"] + "-zstd"] if label in rows]


def plot_case(name, data, control, out):
    labels = ordered(data)
    rows = data["results"]
    names = [LABELS[label] for label in labels]
    colors = ["#b3b3b3", "#929292", "#718096", "#3468b3", "#167e8a", "#b46d26", "#8053a6", "#277f43"]
    plt.rcParams.update({"font.size": 9, "axes.spines.top": False, "axes.spines.right": False,
                         "svg.fonttype": "none", "font.family": "DejaVu Sans"})
    fig, axes = plt.subplots(2, 2, figsize=(13, 8.4), constrained_layout=True)
    sizes = [rows[label]["wire_bytes"] / MIB for label in labels]
    bars = axes[0, 0].barh(names, sizes, color=colors)
    axes[0, 0].invert_yaxis()
    axes[0, 0].bar_label(bars, fmt="%.2f", padding=3, fontsize=8)
    axes[0, 0].set(xlabel="Stored payload (MiB)", title="Disk footprint")
    axes[0, 0].set_xlim(0, max(sizes) * 1.17)
    y = np.arange(len(labels))
    for operation, offset, color in [("encode", -.18, "#3468b3"), ("decode", .18, "#e69a38")]:
        values = np.array([metric(rows[label], operation) * 1000 for label in labels])
        lo = values - np.array([metric(rows[label], operation, statistic="min") * 1000 for label in labels])
        hi = np.array([metric(rows[label], operation, statistic="max") * 1000 for label in labels]) - values
        axes[0, 1].barh(y + offset, values, .32, xerr=[lo, hi], color=color, label=operation.capitalize(),
                       error_kw={"elinewidth": .8, "capsize": 2})
    axes[0, 1].set(yticks=y, yticklabels=names, xlabel="Median time (ms); whiskers show min to max",
                   title="Native encode and owned decode")
    axes[0, 1].invert_yaxis()
    axes[0, 1].legend()
    for operation, offset, color in [("encode", -.18, "#3468b3"), ("decode", .18, "#e69a38")]:
        values = [metric(rows[label], operation, "peak_rss_bytes") / MIB for label in labels]
        axes[1, 0].barh(y + offset, values, .32, color=color, label=operation.capitalize())
    axes[1, 0].set(yticks=y, yticklabels=names, xlabel="Median stage peak RSS (MiB), including its input",
                   title="Peak process memory")
    axes[1, 0].invert_yaxis()
    axes[1, 0].legend()
    totals = [rows[label]["encode_plus_decode_seconds"] * 1000 for label in labels]
    bars = axes[1, 1].barh(names, totals, color=colors)
    axes[1, 1].bar_label(bars, fmt="%.1f", padding=3, fontsize=8)
    axes[1, 1].invert_yaxis()
    axes[1, 1].set(xlabel="Sum of median encode and decode time (ms)", title="Native codec timing comparison")
    axes[1, 1].set_xlim(0, max(totals) * 1.15)
    stats = data["prepare"]["stats"]
    environment = data["environment"]
    fig.suptitle(f"CPG storage: {name} — {stats['source_documents']} documents, "
                 f"{stats['source_functions']:,} functions, {stats['source_bytes']/MIB:.2f} MiB original JSON\n"
                 f"{environment['repetitions']} measured trials after {environment['warmups']} warmup(s) "
                 f"• CPU {environment['cpu_pin']} • Zstandard level {environment['zstd_level']} • native representations",
                 fontsize=12)
    fig.savefig(out / f"{name}.png", dpi=180)
    fig.savefig(out / f"{name}.svg")
    svg = out / f"{name}.svg"
    svg.write_text("\n".join(line.rstrip() for line in svg.read_text().splitlines()) + "\n")
    plt.close(fig)


def markdown_case(name, data, control):
    stats = data["prepare"]["stats"]
    original = stats["source_bytes"]
    rows = data["results"]
    fastest = data["fastest_binary"]
    compressed = rows[fastest + "-zstd"]
    smallest = min((label for label in rows if not label.startswith("original-json")), key=lambda label: rows[label]["wire_bytes"])
    lines = [f"## {name.capitalize()} results", "",
             f"The {stats['source_documents']} input document(s) contain **{stats['source_functions']:,} functions** "
             f"and **{stats['source_nodes']:,} syntax nodes**. Their original JSON occupies "
             f"**{original:,} bytes ({original/MIB:.2f} MiB)**.", "",
             f"**{LABELS[fastest]} has the lowest measured encode plus owned-decode time among the binary formats.** "
             f"Adding Zstandard produces {compressed['wire_bytes']/MIB:.2f} MiB, "
             f"{100*(1-compressed['wire_bytes']/original):.1f}% below the original JSON payload. "
             f"The smallest tested shared representation is {LABELS[smallest]} at "
             f"{rows[smallest]['wire_bytes']/MIB:.2f} MiB.", "",
             "| Time rank | Representation | Payload MiB | Encode ms | Decode ms | Encode plus decode ms | Encode peak MiB | Decode peak MiB |",
             "| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for label in sorted(ordered(data), key=lambda label: rows[label]["time_rank"]):
        row = rows[label]
        lines.append(f"| {row['time_rank']} | {LABELS[label]} | {row['wire_bytes']/MIB:.2f} | "
                     f"{metric(row,'encode')*1000:.2f} | {metric(row,'decode')*1000:.2f} | "
                     f"{row['encode_plus_decode_seconds']*1000:.2f} | "
                     f"{metric(row,'encode','peak_rss_bytes')/MIB:.2f} | {metric(row,'decode','peak_rss_bytes')/MIB:.2f} |")
    lines.extend(["", "Times and memory values are medians. The JSON records retain every sample, "
                  "minimum, maximum, and median absolute deviation. Original JSON encodes the producer's "
                  "JSON trees and decodes into the existing typed CPG Document model, one document at a time. "
                  "Shared variants encode and decode an interned arena. These ranks compare native representations "
                  "of the same content; normalization and conversion into analysis types are excluded.", "",
                  f"**Original JSON ranks {rows['original-json']['time_rank']} of {len(rows)} by encode plus decode; "
                  f"original JSON + Zstandard ranks {rows['original-json-zstd']['time_rank']} of {len(rows)}.**", "",
                  f"![{name.capitalize()} comparisons]({name}.png)", "",
                  f"The normalization step, including input I/O, hashing, statistics, and exact reconstruction checks, "
                  f"took {data['prepare']['normalization_seconds']:.3f} s and reached "
                  f"{data['prepare']['process_peak_rss_bytes']/MIB:.2f} MiB process peak RSS. "
                  "It is a separate one-time prototype cost, excluded from the per-codec timings.", "",
                  "### Sharing observed", "",
                  f"The source contains {stats['input_values']:,} JSON value occurrences, represented by "
                  f"{stats['unique_values']:,} distinct arena values. Repeated string and object-key bytes "
                  f"total {stats['input_string_bytes']/MIB:.2f} MiB before interning and "
                  f"{stats['unique_string_bytes']/MIB:.2f} MiB afterward. These are logical byte counts, "
                  "not independently compressed sizes or heap estimates.", "",
                  "| Unit table | Record occurrences | Exact distinct record contents |",
                  "| --- | ---: | ---: |"])
    for key, count in sorted(stats["table_occurrences"].items()):
        lines.append(f"| {key} | {count:,} | {stats['table_unique_records'][key]:,} |")
    ratio = compressed["wire_bytes"] / rows["original-json-zstd"]["wire_bytes"]
    lines.extend(["", f"Compared with direct compression of the original JSON, {LABELS[fastest]} + Zstandard "
                  f"uses {ratio:.3f} times the storage. This control matters: shared records and binary encoding "
                  "do not guarantee a smaller compressed file than repetitive JSON.", ""])
    return "\n".join(lines)


def render_html(cases, out):
    contents = {}
    for name, data, control in cases:
        rows = []
        for label in ordered(data):
            row = data["results"][label]
            rows.append({"name": LABELS[label], "rank": row["time_rank"], "mib": row["wire_bytes"]/MIB,
                         "encode": metric(row,"encode")*1000, "decode": metric(row,"decode")*1000,
                         "total": row["encode_plus_decode_seconds"]*1000,
                         "encode_peak": metric(row,"encode","peak_rss_bytes")/MIB,
                         "decode_peak": metric(row,"decode","peak_rss_bytes")/MIB})
        stats = data["prepare"]["stats"]
        contents[name] = {"chartfile": f"{name}.png", "rows": rows,
                          "context": f"{stats['source_documents']} documents · {stats['source_functions']:,} functions · "
                                     f"{stats['source_nodes']:,} syntax nodes · {stats['source_bytes']/MIB:.2f} MiB original JSON",
                          "winner": LABELS[data["fastest_binary"]],
                          "original_mib": stats["source_bytes"]/MIB,
                          "original_zstd": control["compressed_bytes"]/MIB}
    options = "".join(f'<option value="{html.escape(name)}"'+(' selected' if name == cases[-1][0] else '')+
                      f'>{html.escape(name.capitalize())}</option>' for name,_,_ in cases)
    template = r'''<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>CPG storage benchmark comparison</title>
<style>:root{color-scheme:light dark;font-family:var(--font-sans,system-ui,sans-serif);color:var(--foreground,CanvasText)}
*{box-sizing:border-box}body{margin:0;line-height:1.5}h1{font-size:21px;margin:0 0 10px}h2{font-size:17px;margin:18px 0 8px}
p{margin:8px 0;font-size:13px}select{font:inherit;padding:5px 9px;border-radius:var(--radius,6px);border:1px solid var(--border,#888)}
.scroll{overflow-x:auto}table{width:100%;border-collapse:collapse;font-size:12px}th,td{padding:8px 6px;border-bottom:1px solid var(--border,#9995);text-align:right;white-space:nowrap}
th:first-child,td:first-child,th:nth-child(2),td:nth-child(2){text-align:left}img{width:100%;height:auto;display:block;background:white;border-radius:var(--radius,6px)}
svg{display:block;width:100%;height:auto}svg text{fill:var(--foreground,CanvasText);font-family:var(--font-sans,system-ui,sans-serif);font-size:12px}
small,.muted{font-size:11px;color:var(--muted-foreground,#777)}a{color:var(--primary,#537fce)}li{font-size:13px;margin:5px 0}</style></head><body>
<h1>CPG storage benchmark comparison</h1>
<label for="dataset">Dataset </label><select id="dataset">OPTIONS</select><p id="context"></p>
<p><strong id="winner"></strong> has the lowest sum of median encode and fully materialized decode time among the uncompressed binary candidates.</p>
<h2>Stored payload</h2><svg id="sizeChart" role="img" aria-label="Stored payload in MiB"></svg>
<h2>Native codec time ranking</h2><p class="muted">Encode followed by decode. Labels show their sum in milliseconds. Original JSON uses the existing typed CPG reader; shared formats use the interned arena.</p><svg id="timeChart" role="img" aria-label="Native encode and owned decode time ranking in milliseconds"></svg>
<div class="scroll"><table><thead><tr><th>Rank</th><th>Representation</th><th>MiB</th><th>Encode ms</th><th>Decode ms</th><th>Total ms</th><th>Encode RSS MiB</th><th>Decode RSS MiB</th></tr></thead><tbody id="rows"></tbody></table></div>
<p id="control"></p>
<p><a id="chartfile">Open the exported chart with memory peaks and timing ranges</a></p>
<h2>How to interpret the measurements</h2>
<ul><li>Shared codecs serialize the same lossless interned arena. Original JSON encodes the producer's JSON trees and decodes into the real CPG Document model. This ranking includes representation differences, with the same original content preserved.</li>
<li>Seven measured trials follow one warmup, in randomized format order. Each operation uses a fresh Rust process pinned to CPU 19. The host is shared.</li>
<li>Codec timings exclude file I/O, input loading, normalization, and validation. Compressed variants include compression or decompression at Zstandard level 3, using one thread.</li>
<li>Original JSON processes documents individually; its times sum codec work and its RSS is the maximum document-stage peak. Shared formats process one whole archive. Data destruction is outside timings. Native zero-copy access and analysis integration are not measured.</li>
<li>Stage peak RSS includes the input model or wire buffer. Linux high-water RSS is reset after loading; normalization memory is reported separately.</li>
<li>The fresh input came from cpg emit on feat/cpg-document-routes with its recorded local changes. Private Cargo build files are excluded from payload sizes.</li></ul>
<p>See <a href="comparison.md">the detailed report</a>, <a href="measurements.csv">the measurement table</a>, and <a href="raw/">the retained raw records</a>.</p>
<script>const cases=DATA;const select=document.getElementById('dataset');
function bars(id,items,unit){const s=document.getElementById(id),left=177,right=76,width=720,step=29,max=Math.max(...items.map(i=>i.a+(i.b||0)));s.setAttribute('viewBox','0 0 '+width+' '+(items.length*step+5));
s.innerHTML=items.map((i,n)=>{const y=n*step+5,a=i.a/max*(width-left-right),b=(i.b||0)/max*(width-left-right);return '<text x="0" y="'+(y+13)+'">'+i.name+'</text><rect x="'+left+'" y="'+y+'" width="'+a+'" height="18" rx="2" fill="'+(i.color||'var(--chart-1,#5277c3)')+'"/>'+(b?'<rect x="'+(left+a)+'" y="'+y+'" width="'+b+'" height="18" fill="var(--chart-2,#d89632)"/>':'')+'<text x="'+(left+a+b+7)+'" y="'+(y+13)+'">'+(i.a+(i.b||0)).toFixed(2)+' '+unit+'</text>'}).join('');}
function show(){const d=cases[select.value];document.getElementById('context').textContent=d.context;
document.getElementById('winner').textContent=d.winner;document.getElementById('chartfile').href=d.chartfile;
bars('sizeChart',d.rows.map(r=>({name:r.name,a:r.mib})),'MiB');
const ranked=[...d.rows].sort((a,b)=>a.rank-b.rank);
bars('timeChart',ranked.map(r=>({name:r.rank+'. '+r.name,a:r.encode,b:r.decode})),'ms');
document.getElementById('rows').innerHTML=ranked.map(r=>'<tr><td>'+r.rank+'</td><td>'+r.name+'</td>'+[r.mib,r.encode,r.decode,r.total,r.encode_peak,r.decode_peak].map(n=>'<td>'+n.toFixed(2)+'</td>').join('')+'</tr>').join('');
document.getElementById('control').textContent='Both original-JSON rows now have measured encode and decode timings. The separate original-JSON compression size check is retained in the raw records.';}
select.addEventListener('change',show);show();</script></body></html>'''
    (out / "comparison.html").write_text(template.replace("OPTIONS",options).replace("DATA",json.dumps(contents).replace("</","<\\/")))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--producer", type=Path, required=True)
    parser.add_argument("directories", type=Path, nargs="+")
    args = parser.parse_args()
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    shutil.copy2(args.producer, out / "producer.json")
    source = Path(__file__).resolve().parent
    paths = [source / name for name in ["Cargo.toml", "Cargo.lock", "run.py", "controls.py", "report.py"]]
    paths.extend(sorted((source / "src").glob("*.rs")))
    paths.extend(sorted((source / "schema").glob("*")))
    sources = {str(path.relative_to(source)): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
    reader = source.parent.parent / "crates/rstyle-core/src/cpg/model.rs"
    sources["../../crates/rstyle-core/src/cpg/model.rs"] = hashlib.sha256(reader.read_bytes()).hexdigest()
    write(out / "benchmark-source.json", sources)
    cases = []
    for directory in args.directories:
        data = read(directory / "results.json")
        control = read(directory / "original-json-zstd.json")
        cases.append((directory.name,data,control))
        raw = out / "raw" / directory.name
        raw.mkdir(parents=True,exist_ok=True)
        for name in ["results.json","prepare.json","environment.json","selection.json","original-json-zstd.json","original-inventory.json"]:
            shutil.copy2(directory/name,raw/name)
        with (raw/"trials.jsonl").open("w") as stream:
            for path in sorted((directory/"trials").glob("*.json")):
                stream.write(json.dumps(read(path),sort_keys=True)+"\n")
        plot_case(directory.name,data,control,out)
    producer = read(args.producer)
    winners = {data['fastest_binary'] for _,data,_ in cases}
    if len(winners) == 1:
        speed_finding = f"{LABELS[next(iter(winners))]} is fastest for encode plus full owned decoding in every measured dataset."
    else:
        speed_finding = "The fastest binary format varies between the measured datasets."
    direct_compression_wins = all(data['results']['original-json-zstd']['wire_bytes'] <= min(row['wire_bytes'] for row in data['results'].values())
                                  for _,data,control in cases)
    if direct_compression_wins:
        size_finding = ("Direct Zstandard compression of the original JSON gives the smallest payloads.** "
            "The shared arena reduces uncompressed storage, but its extra references and record tuples do not "
            "improve compressed size over that simple control. For the current disk-space goal, direct compression "
            "is the first production experiment to pursue; the fastest binary codec remains a candidate for a reader "
            "designed to consume the shared arena directly.")
    else:
        size_finding = ("The storage comparison depends on the dataset and representation.** "
            "Compare each shared representation with the direct original-JSON compression control before choosing "
            "a production storage layout.")
    header = "\n".join(["# CPG storage benchmark comparison", "",
        "This study compares a lossless shared-table representation of fresh rstyle CPG output in JSON, "
        "FlatBuffers, FlexBuffers, and Protobuf. JSON and the fastest uncompressed binary codec are also "
        "tested with Zstandard level 3. Original JSON is now timed both plain and compressed, using the producer's "
        "JSON-tree encoder and the existing typed CPG reader model. All variants appear in the native timing rank. "
        "The production CPG writer and readers are unchanged.", "",
        "**"+speed_finding+" "+size_finding, "",
        "## Input and source provenance", "",
        f"The producer was built from `{producer['branch']}` at `{producer['head']}` with local changes. "
        f"The source inventory SHA256 is `{producer['source_tree_sha256']}`. "
        "`producer.json` records the exact binary hashes and dirty file inventory; the local artifact "
        "directory also retains the source patch, untracked-source archive, frozen binaries, emitted documents, and emit logs.", "",
        "Input is a fresh `cpg emit` over the actual workspace. The core dataset is its real rstyle-core "
        "compilation document; the workspace dataset contains all seven emitted documents. "
        "Neither dataset is synthetic or duplicated to manufacture scale. Private Cargo build artifacts "
        "are excluded. These inputs do not establish a byte breakdown for the previously estimated 3.1 GB capture.", ""])
    methods = "\n".join(["## Method and practical limits", "",
        "- Every shared-arena codec receives identical strings, scalar values, child references, and document roots. "
        "The prototype interns exact strings and JSON substructures into a shared arena. "
        "It does not merge compiler entities by display name or discard observations, provenance, source text, or unknown fields.",
        "- Original JSON uses serde_json::Value serialization as the actual producer does, and deserializes with "
        "the existing rstyle-core CPG Document model imported directly from its source file, whose hash is recorded. "
        "It processes one document at a time and sums codec time across documents. Its peak RSS is the maximum "
        "document-stage peak. Typed documents are kept alive through the timing boundary and destroyed afterward. "
        "The strict typed decoder must succeed. A separate generic parse checks canonical wire-document "
        "SHA256 outside timing after the typed output is dropped; it is not part of the reported decode time.",
        "- Preparation reconstructs every original JSON document and checks semantic JSON equality. "
        "Shared results are checked with a SHA256 over every materialized arena field against the prepared model. "
        "Original JSON results are checked against a separately computed canonical document inventory. "
        "Original document SHA256 values and deterministic wire hashes are retained.",
        "- Seven measured repetitions follow one warmup. Format order is randomized with seed 20261105. "
        "Each encode and decode runs in a separate optimized Rust process pinned to CPU 19. "
        "The host is shared; cgroup CPU and memory counters are retained. Whiskers show the observed range, not a confidence interval.",
        "- Fastest means the lowest sum of median encode time and median full owned-decode time among "
        "FlatBuffers, FlexBuffers, and Protobuf. The winning codec is then measured with compression in additional trials. "
        "Read-heavy workloads can choose a different weighting.",
        "- Timings cover encoding to owned wire buffers and decoding to each representation's fully owned output. "
        "File I/O, loading the common model, normalization, digest calculation, validation, and buffer destruction "
        "are outside those intervals. Compressed variants include the compression/decompression stage. "
        "This is not an end-to-end compiler or rstyle-analysis benchmark.",
        "- The overall rank compares native representations of the same CPG information: original typed documents "
        "versus an interned shared arena. It includes representation and codec effects. "
        "Preparation is excluded for both; no claim is made that the existing analysis can consume the arena "
        "without additional integration. The separate binary winner selection remains restricted to the three uncompressed binary codecs.",
        "- All compression uses Zstandard level 3 with one thread. Rust codecs use zstd 0.14.3 with "
        "libzstd 1.5.7. The original-JSON size control uses python-zstandard 0.25.0 with the same libzstd version, "
        "one independent frame per original document, and verified byte-for-byte reconstruction.",
        "- Stage peak RSS is sampled from Linux VmHWM after resetting the high-water mark with clear_refs=5. "
        "It includes the loaded common model for encode, or wire buffer for decode. Preparation peaks are reported "
        "separately and can exceed codec peaks. Extra retained allocator memory and builder capacity remain part of process RSS.",
        "- FlatBuffers uses generated vectors of fixed-size arena value structs and transfers its native builder "
        "buffer without an added full-buffer copy. Protobuf uses prost messages with packed child IDs. "
        "FlexBuffers uses its Rust Serde serializer with default key sharing. JSON uses compact value tuples. "
        "No binary codec wraps JSON bytes. Pinned versions and schemas accompany the harness.",
        "- Shared-format results apply to this common lossless arena and Rust implementations. A domain-specific typed binary CPG "
        "schema, native zero-copy access, independently compressed function chunks, other languages, "
        "different compression levels, and on-disk query indexes are not measured here. "
        "The current archive is monolithic; production chunking and bounded-memory normalization need a separate design.", "",
        "## Reproduction and retained measurements", "",
        "See [the benchmark instructions](../../README.md) for build, capture, trial, and report commands. "
        "`measurements.csv` contains medians, ranges, median absolute deviations, sizes, and memory peaks. "
        "`raw/<dataset>/trials.jsonl` contains every warmup and measured operation. "
        "`raw/<dataset>/results.json` retains input hashes, environment, selection, and all aggregate samples. "
        "The HTML report works offline, and PNG/SVG charts are included for sharing.", ""])
    (out/"comparison.md").write_text(header+"\n"+"\n".join(markdown_case(*case) for case in cases)+"\n"+methods)
    with (out/"measurements.csv").open("w",newline="") as stream:
        fields=["dataset","format","representation","time_rank","operation","payload_bytes","median_seconds","min_seconds","max_seconds",
                "mad_seconds","median_peak_rss_bytes","median_baseline_rss_bytes","median_io_seconds"]
        writer=csv.DictWriter(stream,fieldnames=fields,lineterminator="\n");writer.writeheader()
        for name,data,_ in cases:
            for label in ordered(data):
                row=data["results"][label]
                for operation in ["encode","decode"]:
                    t=row[operation]["seconds"]
                    writer.writerow(dict(dataset=name,format=label,representation=row["representation"],time_rank=row["time_rank"],operation=operation,payload_bytes=row["wire_bytes"],
                        median_seconds=t["median"],min_seconds=t["min"],max_seconds=t["max"],mad_seconds=t["mad"],
                        median_peak_rss_bytes=metric(row,operation,"peak_rss_bytes"),
                        median_baseline_rss_bytes=metric(row,operation,"baseline_rss_bytes"),
                        median_io_seconds=metric(row,operation,"io_seconds")))
    render_html(cases,out)
    print(out/"comparison.md")
    print(out/"comparison.html")


if __name__ == "__main__":
    main()
