#!/bin/bash

# Clean up old snapshots (rarely needed since run_case.sh auto-cleans)

echo "🧹 Cleaning up old snapshots..."
uv run tools/cleanup.py