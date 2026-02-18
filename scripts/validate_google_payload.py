#!/usr/bin/env python3
"""Send a Google Gemini payload to the real API and report whether it's accepted.

Usage:
    # Validate the minimal fuzz case:
    python scripts/validate_google_payload.py '{"contents": [{}]}'

    # Validate from a snapshot file:
    python scripts/validate_google_payload.py payloads/fuzz-snapshots/google-roundtrip/case-XYZ.request.json

    # Validate all snapshots in a directory:
    python scripts/validate_google_payload.py payloads/fuzz-snapshots/google-roundtrip/

Requires GEMINI_API_KEY (or GOOGLE_API_KEY) in the environment.
"""

import json
import os
import sys
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

MODEL = os.environ.get("GEMINI_MODEL", "gemini-2.0-flash")
API_KEY = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
BASE_URL = "https://generativelanguage.googleapis.com/v1beta"


def validate_payload(payload: dict, label: str = "") -> bool:
    url = f"{BASE_URL}/models/{MODEL}:generateContent?key={API_KEY}"
    data = json.dumps(payload).encode()
    req = Request(url, data=data, headers={"Content-Type": "application/json"})

    prefix = f"[{label}] " if label else ""
    try:
        resp = urlopen(req)
        body = json.loads(resp.read())
        print(f"{prefix}ACCEPTED (200) -- API processed the payload")
        if "candidates" in body:
            text = (
                body["candidates"][0]
                .get("content", {})
                .get("parts", [{}])[0]
                .get("text", "")[:80]
            )
            if text:
                print(f"  response: {text!r}")
        return True
    except HTTPError as e:
        body = e.read().decode()
        try:
            err = json.loads(body)
            msg = err.get("error", {}).get("message", body[:200])
            status = err.get("error", {}).get("status", e.code)
        except json.JSONDecodeError:
            msg = body[:200]
            status = e.code
        print(f"{prefix}REJECTED ({status}) -- {msg}")
        return False


def load_payload(arg: str) -> list[tuple[str, dict]]:
    """Return list of (label, payload) from a JSON string, file, or directory."""
    path = Path(arg)

    # Directory: load all .request.json files
    if path.is_dir():
        results = []
        for f in sorted(path.glob("*.request.json")):
            with open(f) as fh:
                results.append((f.name, json.load(fh)))
        return results

    # File
    if path.is_file():
        with open(path) as fh:
            return [(path.name, json.load(fh))]

    # Inline JSON string
    try:
        return [("inline", json.loads(arg))]
    except json.JSONDecodeError:
        print(f"Error: not a valid JSON string, file, or directory: {arg}", file=sys.stderr)
        sys.exit(1)


def main():
    if not API_KEY:
        print("Set GEMINI_API_KEY or GOOGLE_API_KEY environment variable.", file=sys.stderr)
        sys.exit(1)

    if len(sys.argv) < 2:
        print(__doc__.strip())
        sys.exit(1)

    payloads = load_payload(sys.argv[1])
    if not payloads:
        print("No payloads found.", file=sys.stderr)
        sys.exit(1)

    accepted = 0
    rejected = 0
    for label, payload in payloads:
        if validate_payload(payload, label):
            accepted += 1
        else:
            rejected += 1

    if len(payloads) > 1:
        print(f"\n--- {accepted} accepted, {rejected} rejected (of {len(payloads)}) ---")

    sys.exit(0 if rejected == 0 else 1)


if __name__ == "__main__":
    main()
