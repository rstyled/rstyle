#!/usr/bin/env python3
"""Pipe existing plain CPG documents to a receiver using bounded byte buffers.

Real frontends can write their JSON directly between the same record separators.
Emit the terminal null only after all document generation succeeds.
"""
import shutil
import sys


def main():
    if len(sys.argv) < 2:
        raise SystemExit("usage: pipe.py DOCUMENT.cpg.json ...")
    output = sys.stdout.buffer
    for path in sys.argv[1:]:
        with open(path, "rb") as document:
            output.write(b"\x1e")
            shutil.copyfileobj(document, output, length=64 * 1024)
            output.write(b"\n")
    output.write(b"\x1enull\n")
    output.flush()


if __name__ == "__main__":
    main()
