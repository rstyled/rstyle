#!/bin/sh
# Points git at the tracked hooks in .githooks/ (no copying, so they stay current).
set -e
cd "$(dirname "$0")/.."
chmod +x .githooks/*
git config core.hooksPath .githooks
echo "hooks installed: core.hooksPath=.githooks"
