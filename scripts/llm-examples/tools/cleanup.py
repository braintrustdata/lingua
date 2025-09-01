#!/usr/bin/env python3

"""
Clean up old snapshots, keeping only the latest for each test case.
"""

import os
from pathlib import Path
from collections import defaultdict

def main():
    snapshots_dir = Path(__file__).parent.parent / "snapshots"
    
    if not snapshots_dir.exists():
        print("📂 No snapshots directory found")
        return
    
    # Group snapshots by case name
    case_snapshots = defaultdict(list)
    
    for snapshot_file in snapshots_dir.glob("*.json"):
        # Extract case name from filename (everything before the last underscore + timestamp)
        name_parts = snapshot_file.stem.split('_')
        if len(name_parts) >= 3:  # case_name_YYYYMMDD_HHMMSS
            case_name = '_'.join(name_parts[:-2])  # Everything except last 2 parts
            case_snapshots[case_name].append(snapshot_file)
    
    print(f"🧹 Cleaning up snapshots...")
    
    total_removed = 0
    for case_name, snapshots in case_snapshots.items():
        if len(snapshots) > 1:
            # Sort by modification time, keep the newest
            snapshots.sort(key=os.path.getctime, reverse=True)
            latest = snapshots[0]
            old_snapshots = snapshots[1:]
            
            print(f"\n📋 {case_name}:")
            print(f"  ✅ Keeping: {latest.name}")
            
            for old_snapshot in old_snapshots:
                try:
                    old_snapshot.unlink()
                    print(f"  🗑️  Removed: {old_snapshot.name}")
                    total_removed += 1
                except OSError as e:
                    print(f"  ❌ Failed to remove {old_snapshot.name}: {e}")
        else:
            print(f"📋 {case_name}: Only 1 snapshot, keeping it")
    
    print(f"\n✨ Cleanup complete! Removed {total_removed} old snapshots")

if __name__ == "__main__":
    main()