#!/usr/bin/env bash
# Adds the license header to any Rust file that is missing it.
set -euo pipefail

header='// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.'

found=0
while IFS= read -r file; do
    found=1
    if ! grep -q "Mintlayer Institutional FZCO" "$file"; then
        printf '%s\n\n' "$header" | cat - "$file" > "$file.tmp"
        mv "$file.tmp" "$file"
        echo "added header: $file"
    fi
done < <(find src examples tests -name '*.rs' -type f 2>/dev/null)

if [ "$found" -eq 0 ]; then
    echo "no Rust files found"
fi
