# All Available Commands

## 🎯 Main Commands (You Only Need These)

```bash
./list_cases.sh                     # Show all available test cases
./run_case.sh cases/<case>.json     # Run test and analyze results
```

## 📊 Optional Utilities 

```bash  
./summary.sh                        # Show status of all cases
./summary.sh simple_text             # Detailed analysis of specific case
./cleanup.sh                        # Manual cleanup (rarely needed)
```

## 📁 Clean Structure

### Bash Scripts (What You Run)
- `list_cases.sh` - List available cases
- `run_case.sh` - Run and analyze (main command)
- `summary.sh` - Show case status
- `cleanup.sh` - Clean old snapshots

### Python Scripts (Called Automatically)
- `tools/runner.py` - API calls (used by run_case.sh)
- `tools/analyzer.py` - Structure analysis (used by run_case.sh)  
- `tools/summary.py` - Status report (used by summary.sh)
- `tools/cleanup.py` - Snapshot cleanup (used by cleanup.sh)

### Test Cases
- `cases/*.json` - Test definitions you can run

### Results
- `snapshots/*.json` - Latest results (auto-managed)

## ✨ What Changed

**Removed:**
- ❌ `quick_analyze.py` (redundant)
- ❌ Direct Python script calls in docs

**Added:**
- ✅ `summary.sh` - Bash wrapper for summary
- ✅ `cleanup.sh` - Bash wrapper for cleanup  
- ✅ Bash-only interface

**Result:** You only need to remember 2 main commands!
- `./list_cases.sh` - See what's available
- `./run_case.sh cases/<case>.json` - Run anything