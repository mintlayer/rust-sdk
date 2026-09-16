#!/usr/bin/env bash
# Verifies that every Rust file carries the license header.
set -euo pipefail

missing=0
while IFS= read -r file; do
    if ! grep -q "Mintlayer Institutional FZCO" "$file"; then
        echo "missing header: $file"
        missing=$((missing + 1))
    fi
done < <(find src examples tests -name '*.rs' -type f 2>/dev/null)

if [ "$missing" -gt 0 ]; then
    echo "$missing file(s) missing the license header"
    exit 1
fi
echo "all Rust files carry the license header"
