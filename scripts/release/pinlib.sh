#!/usr/bin/env bash
# Shared helpers for the release pin scripts (sourced, not executed).
# Self-contained: pins LC_ALL=C for raw-byte comparisons.
export LC_ALL=C

esc() {
  # Strip control and non-ASCII bytes (pin tags are ASCII-only, so this
  # is lossless for legitimate values and kills C0/C1 terminal escapes),
  # then %-escape for workflow-command safety.
  local s
  s="$(printf '%s' "$1" | tr -d '\000-\037\177\200-\377')"
  s="${s//%/%25}"
  printf '%s' "$s"
}

# Swift code-line extraction for pin verification: block comments first
# (perl -0777 so multi-line comments collapse), then line comments, then
# leading/trailing whitespace and blank lines — the emitted lines are
# exactly the assignment lines, normalized to column 0. A decoy literal
# inside any comment is invisible afterwards. Known limitation: a '//'-
# sequence inside a Swift string literal is still treated as a comment
# (fail-closed: counts drop and callers refuse), so keep pin lines free
# of '//' inside string literals.
strip_swift_comments() {
  printf '%s' "$1" \
    | perl -0777 -pe 's{/\*.*?\*/}{}gs' \
    | sed 's://.*$::' \
    | sed 's/^[[:space:]]*//; s/[[:space:]]*$//' \
    | grep -v '^$' \
    || true
}

# Count, in the normalized code view of <file>: (a) whole-line matches
# of the expected releaseTag assignment, (b) whole-line matches of the
# expected checksum assignment, (c) total checksum assignment lines.
# Prints "a b c" — fixed strings throughout (TAG may contain '+').
count_pin_lines() { # <file> <tag> <checksum>
  local file="$1" tag="$2" sum="$3"
  printf '%s\n' "$(strip_swift_comments "$(cat "$file")")" \
    | awk -v t="let releaseTag = \"${tag}\"" -v s="checksum: \"${sum}\"" \
        '{ if ($0 == t) a++; if ($0 == s) b++; if ($0 ~ /^checksum: "/) c++ } \
          END { printf "%d %d %d\n", a+0, b+0, c+0 }'
}
