#!/usr/bin/env bash
# Refuse to clobber published release assets when the default branch
# already pins THIS release and the rebuilt artifacts are not
# bit-reproducible: publishing mismatched artifacts would desync SPM
# resolution from the merged checksum on the default branch. Forward
# releases are unaffected (the default branch pins an older release, or
# nothing at all).
#
# Usage: guard-release-refresh.sh <TAG> <XCSUM> <DEFAULT_BRANCH>
set -euo pipefail

TAG="${1:?TAG required}"
XCSUM="${2:?XCSUM required}"
DEFAULT_BRANCH="${3:?DEFAULT_BRANCH required}"

esc() {
  local s="$1"
  s="${s//%/%25}"
  s="${s//$'\r'/%0D}"
  s="${s//$'\n'/%0A}"
  printf '%s' "$s"
}

# Mirror the workflow's validation so direct invocations fail closed.
if [[ ! "${TAG}" =~ ^wallet-bindings-v[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::tag does not match wallet-bindings-v<semver>"
  exit 1
fi
if [[ ! "${XCSUM}" =~ ^[0-9a-f]{64}$ ]] || [ "${XCSUM//0/}" = "" ]; then
  echo "::error::checksum is not 64 lowercase hex characters (all-zero refused)"
  exit 1
fi
if [[ ! "${DEFAULT_BRANCH}" =~ ^[A-Za-z0-9][A-Za-z0-9._/-]*$ ]] \
  || [[ "${DEFAULT_BRANCH}" == pin-spm-checksum-* ]]; then
  echo "::error::unexpected default branch: $(esc "${DEFAULT_BRANCH}")"
  exit 1
fi

origin_swift="$(git show "origin/${DEFAULT_BRANCH}:Package.swift" 2>/dev/null)" \
  || { echo "::error::cannot read origin/${DEFAULT_BRANCH}:Package.swift"; exit 1; }
# Only real code lines constitute the pin: comment lines are ignored so
# a decoy literal in a comment cannot flip the guard's verdict.
origin_code="$(printf '%s\n' "${origin_swift}" | grep -v '^[[:space:]]*//')"

pin_tag_matches="$(printf '%s\n' "${origin_code}" \
  | grep -cE '^[[:space:]]*let releaseTag = "wallet-bindings-v[^"]*"$' || true)"
if [ "${pin_tag_matches}" != "1" ]; then
  echo "::error::expected exactly one releaseTag assignment on ${DEFAULT_BRANCH}, found ${pin_tag_matches}"
  exit 1
fi
pin_sum_matches="$(printf '%s\n' "${origin_code}" \
  | grep -cE '^[[:space:]]*checksum: "[0-9a-f]{64}"$' || true)"
if [ "${pin_sum_matches}" != "1" ]; then
  echo "::error::expected exactly one checksum assignment on ${DEFAULT_BRANCH}, found ${pin_sum_matches}"
  exit 1
fi
pin_tag="$(printf '%s\n' "${origin_code}" \
  | grep -E '^[[:space:]]*let releaseTag = ' | grep -oE 'wallet-bindings-v[^"]*')"
pin_sum="$(printf '%s\n' "${origin_code}" \
  | grep -E '^[[:space:]]*checksum: ' | grep -oE '[0-9a-f]{64}')"

# Only a pin for THIS release constrains the refresh; identical
# checksums make the refresh a harmless no-op.
if [ "${pin_tag}" = "${TAG}" ] && [ "${pin_sum}" != "${XCSUM}" ]; then
  echo "::error::rebuilt artifacts (checksum $(esc "${XCSUM}")) do not match the pin merged on $(esc "${DEFAULT_BRANCH}") ($(esc "${pin_sum}")); release assets left untouched - repair manually"
  exit 1
fi
