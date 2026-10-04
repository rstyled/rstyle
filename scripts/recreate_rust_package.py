#!/usr/bin/env python3
"""Recreate the CodeNet Rust training package from source.

Single-file pipeline: FETCH -> EXTRACT (Rust-only) -> BUILD (training-ready).

  python3 recreate_rust_package.py
  python3 recreate_rust_package.py --skip-extract   # reuse existing Project_CodeNet/
  python3 recreate_rust_package.py --skip-build     # extract only
  python3 recreate_rust_package.py --tar /tmp/Project_CodeNet.tar.gz --keep-tar

Default streams the 8.3GB tarball (no full download stored); only the Rust
subset (~2.6GB code + 1.2GB metadata) is written to disk. With --tar, a local
tarball is used/kept instead of streaming.

Source: https://github.com/IBM/Project_CodeNet (Apache-2.0)
Tarball: https://codait-cos-dax.s3.us.cloud-object-storage.appdomain.cloud/dax-project-codenet/1.0.0/Project_CodeNet.tar.gz
Expected result: 127210 Rust files, 2139 problems, 0 missing code.
"""
import argparse
import csv
import json
import os
import shutil
import subprocess
import sys
import tarfile
import time
from collections import Counter
from html.parser import HTMLParser
from pathlib import Path

URL = "https://codait-cos-dax.s3.us.cloud-object-storage.appdomain.cloud/dax-project-codenet/1.0.0/Project_CodeNet.tar.gz"
BASE = Path("Project_CodeNet")
OUT = Path("rust_package")
DATA = BASE / "data"
META = BASE / "metadata"
DESC = BASE / "problem_descriptions"
DERIVED = BASE / "derived"


# ---------- FETCH ----------

def open_tar_stream(url, tar_path):
    """Return a file object yielding the gzip tar stream."""
    if tar_path and Path(tar_path).exists():
        return open(tar_path, "rb"), None
    proc = subprocess.Popen(
        ["curl", "-L", "--retry", "3", "--retry-delay", "5", "--fail", url],
        stdout=subprocess.PIPE, bufsize=1024 * 1024,
    )
    return proc.stdout, proc


def fetch_tarball(url, dest):
    """Explicit fetch: download full tarball to dest (resumable via curl -C -)."""
    dest = Path(dest)
    dest.parent.mkdir(parents=True, exist_ok=True)
    cmd = ["curl", "-L", "-C", "-", "--retry", "3", "--retry-delay", "5",
           "-o", str(dest), url]
    print(f"fetching {url} -> {dest}", flush=True)
    subprocess.run(cmd, check=True)
    return dest


# ---------- EXTRACT ----------

def should_keep(name):
    if name in ("Project_CodeNet", "Project_CodeNet/",
                "Project_CodeNet/README.md"):
        return True
    for prefix in ("Project_CodeNet/metadata/",
                   "Project_CodeNet/problem_descriptions/",
                   "Project_CodeNet/derived/"):
        if name.startswith(prefix):
            return True
    parts = name.split("/")
    return (len(parts) >= 4 and parts[0] == "Project_CodeNet"
            and parts[1] == "data" and parts[3] == "Rust")


def extract_member(tar, member, dest):
    try:
        tar.extract(member, path=str(dest), filter="data")
    except TypeError:
        tar.extract(member, path=str(dest))


def log_progress(total, kept, rust, name, t0):
    if total % 50000 == 0 or (total < 50000 and total % 5000 == 0):
        print(f"[{(time.time()-t0)/60:.1f}m] scanned={total} "
              f"kept={kept} rust={rust} current={name}", flush=True)


def extract_rust_subset(url, dest, tar_path=None):
    """Stream-extract Rust subset; returns (scanned, kept, rust_files)."""
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    stream, proc = open_tar_stream(url, tar_path)
    total, kept, rust = 0, 0, 0
    t0 = time.time()
    tar = tarfile.open(fileobj=stream, mode="r|gz")
    try:
        for member in tar:
            total += 1
            if should_keep(member.name):
                extract_member(tar, member, dest)
                kept += 1
                if "/Rust/" in member.name and member.isfile():
                    rust += 1
            log_progress(total, kept, rust, member.name, t0)
    finally:
        tar.close()
        if proc is not None:
            if hasattr(stream, "close"):
                try:
                    stream.close()
                except Exception:
                    pass
            proc.wait()
            print(f"curl exit: {proc.returncode}", flush=True)
        else:
            stream.close()
    print(f"DONE in {(time.time()-t0)/60:.1f} min: "
          f"scanned={total} kept={kept} rust_files={rust}", flush=True)
    return total, kept, rust


# ---------- BUILD ----------

class TextExtractor(HTMLParser):
    def __init__(self):
        super().__init__()
        self.parts = []

    def handle_data(self, d):
        t = d.strip()
        if t:
            self.parts.append(t)

    def text(self):
        return " ".join(self.parts)


def html_to_text(html):
    p = TextExtractor()
    try:
        p.feed(html)
    except Exception:
        pass
    return p.text()[:8000]


def clean_row(row):
    return {k: (v[:-1] if isinstance(v, str) and v.endswith("\r") else v)
            for k, v in row.items()}


def is_rust_row(row):
    return row.get("language") == "Rust"


def load_problems():
    problems = {}
    path = META / "problem_list.csv"
    if not path.exists():
        print("WARNING: problem_list.csv missing", flush=True)
        return problems
    with open(path, newline="", encoding="utf-8", errors="replace") as f:
        reader = csv.DictReader(
            f, fieldnames=["id", "name", "dataset", "time_limit",
                           "memory_limit", "rating", "tags", "complexity"])
        for row in reader:
            if row["id"] == "id":
                continue
            problems[row["id"]] = row
    shutil.copy(path, OUT / "problem_list.csv")
    print(f"loaded {len(problems)} problems", flush=True)
    return problems


def collect_rust_rows():
    """Filter per-problem CSVs to Rust rows; write metadata_rust/ + consolidated CSV."""
    files = sorted(p for p in META.glob("p*.csv")
                   if p.name != "problem_list.csv") if META.exists() else []
    print(f"found {len(files)} per-problem csv files", flush=True)
    rust_rows, pids, counter = [], set(), Counter()
    for csv_path in files:
        kept = read_rust_rows(csv_path)
        if kept:
            pids.add(csv_path.stem)
            rust_rows.extend(kept)
            for row in kept:
                counter[row.get("status", "?")] += 1
            write_filtered_csv(csv_path.stem, kept)
    print(
        f"Rust submissions: {len(rust_rows)}, problems: {len(pids)}", flush=True)
    print(f"status: {dict(counter)}", flush=True)
    if rust_rows:
        with open(OUT / "rust_submissions.csv", "w", newline="",
                  encoding="utf-8") as f:
            writer = csv.DictWriter(f, fieldnames=list(rust_rows[0].keys()))
            writer.writeheader()
            writer.writerows(rust_rows)
    return rust_rows, pids, counter


def read_rust_rows(csv_path):
    try:
        with open(csv_path, newline="", encoding="utf-8",
                  errors="replace") as f:
            reader = csv.DictReader(f)
            if reader.fieldnames is None:
                return []
            return [clean_row(r) for r in reader if is_rust_row(r)]
    except Exception as e:
        print(f"warn {csv_path}: {e}", flush=True)
        return []


def write_filtered_csv(pid, kept):
    fields = list(kept[0].keys())
    with open(OUT / "metadata_rust" / f"{pid}.csv", "w", newline="",
              encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fields)
        writer.writeheader()
        writer.writerows(kept)


def copy_descriptions(pids):
    out = OUT / "problem_descriptions_rust"
    out.mkdir(exist_ok=True)
    n = 0
    for pid in pids:
        src = DESC / f"{pid}.html"
        if src.exists():
            shutil.copy(src, out / f"{pid}.html")
            n += 1
    print(f"copied {n} problem descriptions", flush=True)
    return n


def copy_derived_subset():
    """Copy Rust-relevant derived bits; skip 13GB C/C++/Java/Python duplicates."""
    if not DERIVED.exists():
        print("derived/ missing", flush=True)
        return
    out = OUT / "derived"
    out.mkdir(exist_ok=True)
    src = DERIVED / "input_output"
    if src.exists():
        dst = out / "input_output"
        if dst.exists():
            shutil.rmtree(dst)
        shutil.copytree(src, dst)
        print("copied derived/input_output/", flush=True)
    copy_identical_clusters(out)
    copy_top_readmes(out)
    with open(out / "NOTE.txt", "w") as f:
        f.write(
            "derived/duplicates bulk data (13GB, C/C++/Java/Python only) omitted.\n")
        f.write("Kept: input_output/, identical_problem_clusters.csv.\n")


def copy_identical_clusters(out):
    ipc = DERIVED / "duplicates" / "identical_problem_clusters"
    if ipc.is_file():
        shutil.copy(ipc, out / "identical_problem_clusters.csv")
        print("copied identical_problem_clusters.csv", flush=True)


def copy_top_readmes(out):
    for cand in [DERIVED / "benchmarks" / "README.md",
                 DERIVED / "duplicates" / "README"]:
        if cand.exists():
            shutil.copy(cand, out / f"{cand.parent.name}_{cand.name}")


def find_sample(pid):
    base = DERIVED / "input_output" / "data" / pid
    si, so = "", ""
    try:
        ip, op = base / "input.txt", base / "output.txt"
        if ip.exists():
            si = ip.read_text(encoding="utf-8", errors="replace")[:4000]
        if op.exists():
            so = op.read_text(encoding="utf-8", errors="replace")[:4000]
    except Exception:
        pass
    return si, so


def read_code(pid, sid, ext):
    path = DATA / pid / "Rust" / f"{sid}.{ext or 'rs'}"
    if not path.exists():
        path = DATA / pid / "Rust" / f"{sid}.rs"
    if not path.exists():
        return "", ext or "rs"
    try:
        return path.read_text(encoding="utf-8", errors="replace"), ext or "rs"
    except Exception:
        return "", ext or "rs"


def read_description(pid):
    path = DESC / f"{pid}.html"
    if not path.exists():
        return ""
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except Exception:
        return ""


def make_record(row, problems):
    sid = row.get("submission_id", "")
    pid = row.get("problem_id", "")
    code, ext = read_code(pid, sid, row.get("filename_ext"))
    if not code:
        return None
    html = read_description(pid)
    si, so = find_sample(pid)
    prob = problems.get(pid, {})
    return {
        "submission_id": sid, "problem_id": pid,
        "problem_name": prob.get("name", ""),
        "dataset": prob.get("dataset", ""),
        "time_limit_ms": prob.get("time_limit", ""),
        "memory_limit_kb": prob.get("memory_limit", ""),
        "user_id": row.get("user_id", ""), "date": row.get("date", ""),
        "language": row.get("language", ""),
        "original_language": row.get("original_language", ""),
        "status": row.get("status", ""),
        "cpu_time_ms": row.get("cpu_time", ""),
        "memory_kb": row.get("memory", ""),
        "code_size_bytes": row.get("code_size", ""),
        "accuracy": row.get("accuracy", ""),
        "problem_description_html": html[:20000],
        "problem_description_text": html_to_text(html) if html else "",
        "sample_input": si, "sample_output": so,
        "code": code, "code_path": f"data/{pid}/Rust/{sid}.{ext}",
    }


def write_jsonl(rust_rows, problems):
    n, missing = 0, 0
    with open(OUT / "rust_training.jsonl", "w", encoding="utf-8") as f:
        for row in rust_rows:
            rec = make_record(row, problems)
            if rec is None:
                missing += 1
                continue
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")
            n += 1
    print(f"wrote {n} jsonl records, missing_code={missing}", flush=True)
    return n, missing


def write_stats_readme(rust_rows, pids, counter, n_desc, n_written):
    rs_files = list(DATA.glob("*/Rust/*.rs")) if DATA.exists() else []
    stats = {
        "rust_submissions_total": len(rust_rows),
        "rust_records_with_code": n_written,
        "problems_with_rust": len(pids),
        "status_breakdown": dict(counter),
        "descriptions_copied": n_desc,
        "rs_files_on_disk": len(rs_files),
    }
    with open(OUT / "stats.json", "w") as f:
        json.dump(stats, f, indent=2)
    print(json.dumps(stats, indent=2), flush=True)
    readme = OUT / "README.md"
    readme.write_text(
        "# CodeNet Rust subset (training-ready)\n\n"
        "Recreated via `recreate_rust_package.py` from IBM Project CodeNet\n"
        f"({URL}).\n\n"
        f"- `rust_submissions.csv` — Rust rows only ({len(rust_rows)} rows)\n"
        "- `metadata_rust/pXXXXX.csv` — per-problem Rust rows\n"
        "- `problem_list.csv` — dataset-level problem list\n"
        f"- `problem_descriptions_rust/` — {n_desc} HTML files\n"
        "- `derived/` — input_output/ + identical_problem_clusters.csv\n"
        f"- `rust_training.jsonl` — {n_written} records (code + metadata)\n\n"
        "License: Apache-2.0 (IBM Project CodeNet). "
        "Paper: https://arxiv.org/abs/2105.12655.\n"
        f"\n```json\n{json.dumps(stats, indent=2)}\n```\n")
    print("build complete -> rust_package/", flush=True)


def build_package():
    OUT.mkdir(exist_ok=True)
    (OUT / "metadata_rust").mkdir(exist_ok=True)
    problems = load_problems()
    rust_rows, pids, counter = collect_rust_rows()
    n_desc = copy_descriptions(pids)
    copy_derived_subset()
    n_written, _ = write_jsonl(rust_rows, problems)
    write_stats_readme(rust_rows, pids, counter, n_desc, n_written)


# ---------- CLI ----------

def parse_args(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--url", default=URL)
    p.add_argument("--dest", default=".")
    p.add_argument("--tar", default=None,
                   help="local tarball path (use if present, or fetch target)")
    p.add_argument("--keep-tar", action="store_true")
    p.add_argument("--fetch-only", action="store_true")
    p.add_argument("--skip-extract", action="store_true")
    p.add_argument("--skip-build", action="store_true")
    return p.parse_args(argv)


def main(argv=None):
    args = parse_args(argv)
    if args.fetch_only:
        fetch_tarball(args.url, args.tar or "Project_CodeNet.tar.gz")
        return
    tar_path = args.tar if (args.tar and Path(args.tar).exists()) else None
    if not args.skip_extract:
        if args.tar and not Path(args.tar).exists():
            fetch_tarball(args.url, args.tar)
            tar_path = args.tar
        extract_rust_subset(args.url, args.dest, tar_path)
    if not args.skip_build:
        build_package()


if __name__ == "__main__":
    main()
