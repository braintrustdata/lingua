#!/bin/bash

# Run test case and automatically analyze results

if [ $# -lt 1 ]; then
    echo "Usage: ./run_case.sh <case_file>"
    echo "Example: ./run_case.sh cases/web_search_simple.json"
    exit 1
fi

case_file="$1"
case_name=$(basename "$case_file" .json)

echo "🚀 Running case: $case_file"
cd tools

# Run the case
uv run runner.py "../$case_file"

# Check if it succeeded by looking for the snapshot
snapshot_file="../snapshots/${case_name}_*.json"
if ls $snapshot_file 1> /dev/null 2>&1; then
    # Get the actual filename (in case of multiple matches, take the newest)
    actual_snapshot=$(ls -t $snapshot_file | head -1)
    echo ""
    echo "📊 Analyzing results..."
    echo "==============================================="
    uv run analyzer.py "$actual_snapshot"
else
    echo "❌ No snapshot found, analysis skipped"
fi