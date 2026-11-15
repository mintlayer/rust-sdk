#!/usr/bin/env bash
# Shared semver precedence comparator. Sourced by open-spm-pr.sh and
# check-pin-order.sh so every pin-order verdict uses one implementation.
# Self-contained: pins LC_ALL=C for the raw-byte identifier comparisons.
export LC_ALL=C

# semver_prerelease_cmp <a> <b>: identifier-wise compare of the
# pre-release parts (without the leading '-'). A final release outranks
# any pre-release of the same triple (the empty part is the final).
semver_prerelease_cmp() { # <0 if $1 < $2, 0 equal, >0 if $1 > $2
  if [ -z "$1" ] && [ -n "$2" ]; then echo 1; return; fi
  if [ -n "$1" ] && [ -z "$2" ]; then echo -1; return; fi
  if [ -z "$1" ] && [ -z "$2" ]; then echo 0; return; fi
  local -a A B
  IFS='.' read -ra A <<< "${1}"
  IFS='.' read -ra B <<< "${2}"
  local i n
  n=$(( ${#A[@]} > ${#B[@]} ? ${#A[@]} : ${#B[@]} ))
  for ((i = 0; i < n; i++)); do
    local x="${A[i]:-}" y="${B[i]:-}"
    if [ -z "${x}" ] && [ -z "${y}" ]; then continue; fi
    if [ -z "${x}" ]; then echo -1; return; fi
    if [ -z "${y}" ]; then echo 1; return; fi
    # Numeric identifiers are lower than alphanumeric ones. Two numeric
    # identifiers compare by digit length then bytes (no arithmetic, no
    # overflow). The caller regexes admit leading zeros, which semver
    # forbids; with this comparator '01' > '1' deterministically, while
    # the numeric-triple path treats 10#'01' == 10#'1' as equal — a
    # documented deviation between the two sites, not a bug. The
    # caller regexes also admit empty prerelease identifiers
    # (1.2.3-a..b); the comparator orders the empty identifier lowest,
    # deterministically - same deviation class as leading zeros.
    if [[ "${x}" =~ ^[0-9]+$ ]] && [[ "${y}" =~ ^[0-9]+$ ]]; then
      if [ "${#x}" -ne "${#y}" ]; then
        [ "${#x}" -lt "${#y}" ] && { echo -1; return; }
        [ "${#x}" -gt "${#y}" ] && { echo 1; return; }
      fi
      [[ "${x}" < "${y}" ]] && { echo -1; return; }
      [[ "${x}" > "${y}" ]] && { echo 1; return; }
      continue
    elif [[ "${x}" =~ ^[0-9]+$ ]]; then
      echo -1; return
    elif [[ "${y}" =~ ^[0-9]+$ ]]; then
      echo 1; return
    elif [[ "${x}" < "${y}" ]]; then
      echo -1; return
    elif [[ "${x}" > "${y}" ]]; then
      echo 1; return
    fi
  done
  echo 0
}

# semver_cmp <a> <b>: echoes -1|0|1 per semver precedence. Build
# metadata (+suffix) is excluded from precedence. Accepted deviation:
# the callers' validation regexes permit leading zeros in numeric
# components; the 10# arithmetic below compares them decimally, so
# '1.02.3' equals '1.2.3' rather than being rejected.
semver_cmp() { # <0 if $1 < $2, 0 if equal, >0 if $1 > $2
  # Build metadata is excluded from precedence: strip it FIRST, then
  # take the pre-release from the stripped half (a '+' build suffix may
  # itself contain '-').
  local a="${1%%+*}" b="${2%%+*}"
  local ap="" bp=""
  [[ "$a" == *-* ]] && ap="${a#*-}"
  [[ "$b" == *-* ]] && bp="${b#*-}"
  local -a A B
  read -ra A <<< "${a//[^0-9]/ }"
  read -ra B <<< "${b//[^0-9]/ }"
  local i
  for i in 0 1 2; do
    [ "$((10#${A[i]:-0}))" -lt "$((10#${B[i]:-0}))" ] && { echo -1; return; }
    [ "$((10#${A[i]:-0}))" -gt "$((10#${B[i]:-0}))" ] && { echo 1; return; }
  done
  semver_prerelease_cmp "${ap}" "${bp}"
}
