#!/usr/bin/env python3

import json
import os
from pathlib import Path

def main():
    base_dir = Path(__file__).parent.parent
    cases_dir = base_dir / "cases"
    snapshots_dir = base_dir / "snapshots"
    
    print("=" * 80)
    print("📋 LLM EXAMPLES SUMMARY")
    print("=" * 80)
    
    # List all available cases
    cases = list(cases_dir.glob("*.json"))
    cases.sort()
    
    print(f"\n🎯 Available Test Cases ({len(cases)}):")
    print("-" * 40)
    
    for case_file in cases:
        with open(case_file, 'r') as f:
            case_data = json.load(f)
        
        print(f"📄 {case_data['name']}")
        print(f"   {case_data['description']}")
        print(f"   Models: {', '.join(case_data['models'].keys())}")
        
        # Find snapshot for this case (should be only one after cleanup)
        snapshots = list(snapshots_dir.glob(f"{case_data['name']}_*.json"))
        if snapshots:
            # Should be only one, but use latest if multiple exist
            latest = max(snapshots, key=os.path.getctime) if len(snapshots) > 1 else snapshots[0]
            timestamp = latest.stem.split('_', 1)[1]
            print(f"   Latest run: {timestamp}")
            
            # Quick analysis of snapshot
            with open(latest, 'r') as f:
                snapshot_data = json.load(f)
            
            results = snapshot_data["results"]
            success_count = sum(1 for result in results.values() if result.get("status_code") == 200)
            print(f"   Success: {success_count}/{len(results)} providers")
        else:
            print(f"   ⚠️  Never run")
        
        print()
    
    # Summary of all snapshots
    all_snapshots = list(snapshots_dir.glob("*.json"))
    print(f"📸 Total Snapshots: {len(all_snapshots)}")
    
    if all_snapshots:
        latest_overall = max(all_snapshots, key=os.path.getctime)
        print(f"🕐 Most recent: {latest_overall.name}")
    
    print()
    print("=" * 80)
    print("🚀 Usage:")
    print("  ./list_cases.sh                    # List available cases")
    print("  ./run_case.sh cases/<case>.json    # Run and analyze")  
    print("  ./summary.sh                       # Show this summary")
    print("  ./summary.sh <case_name>           # Detailed analysis")
    print("=" * 80)

if __name__ == "__main__":
    main()