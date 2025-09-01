#!/bin/bash

# Show summary of all test cases, or detailed analysis of a specific case

if [ $# -eq 0 ]; then
    # No arguments - show general summary
    echo "📊 Running summary..."
    uv run tools/summary.py
else
    # Case name provided - show detailed analysis
    case_name="$1"
    
    # Check if case exists
    if [ ! -f "cases/${case_name}.json" ]; then
        echo "❌ Case '${case_name}' not found"
        echo "Available cases:"
        ls cases/*.json 2>/dev/null | sed 's/cases\///g' | sed 's/\.json//g' | sed 's/^/  - /'
        exit 1
    fi
    
    # Find the snapshot for this case
    snapshot_file="snapshots/${case_name}_*.json"
    if ls $snapshot_file 1> /dev/null 2>&1; then
        actual_snapshot=$(ls -t $snapshot_file | head -1)
        echo "📊 Detailed analysis for: $case_name"
        echo "📸 Using snapshot: $(basename "$actual_snapshot")"
        echo "==============================================="
        cd tools
        uv run analyzer.py "../$actual_snapshot"
    else
        echo "❌ No snapshot found for case '$case_name'"
        echo "💡 Run it first: ./run_case.sh cases/${case_name}.json"
    fi
fi