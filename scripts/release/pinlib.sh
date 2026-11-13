#!/usr/bin/env bash
# Shared helpers for the release pin scripts (sourced, not executed).
# Callers export LC_ALL=C for raw-byte comparisons.

esc() {
  # Strip control and non-ASCII bytes (pin tags are ASCII-only, so this
  # is lossless for legitimate values and kills C0/C1 terminal escapes),
  # then %-escape for workflow-command safety.
  local s
  s="$(printf '%s' "$1" | tr -d '\000-\037\177\200-\377')"
  s="${s//%/%25}"
  printf '%s' "$s"
}

# Swift comment removal for pin verification: block comments first
# (perl -0777 so multi-line comments collapse), then whole line
# comments. A decoy literal inside any comment is invisible afterwards.
strip_swift_comments() {
  printf '%s' "$1" \
    | perl -0777 -pe 's{/\*.*?\*/}{}gs' \
    | grep -v '^[[:space:]]*//' \
    || true
}

# Count whole-line assignments after comment stripping and trailing
# whitespace removal. Usage: count_line <file-content-var-name> <line>
count_exact_line() {
  local -n _content="$1"
  printf '%s\n' "${_content}" | sed 's:[[:space:]]*//.*$::' | sed 's:[[:space:]]*$://' \
    | grep -cF -x -- "$2" || true
}
