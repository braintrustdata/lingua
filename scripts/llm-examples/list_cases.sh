#!/bin/bash

# List available test cases

echo "📋 Available Test Cases:"
echo "========================"

for case_file in cases/*.json; do
    if [ -f "$case_file" ]; then
        case_name=$(basename "$case_file" .json)
        description=$(jq -r '.description' "$case_file" 2>/dev/null || echo "No description")
        models=$(jq -r '.models | keys | join(", ")' "$case_file" 2>/dev/null || echo "Unknown models")
        
        echo "📄 $case_name"
        echo "   $description"
        echo "   Models: $models"
        echo "   Usage: ./run_case.sh $case_file"
        echo ""
    fi
done

echo "💡 Run any case with: ./run_case.sh cases/<case_name>.json"