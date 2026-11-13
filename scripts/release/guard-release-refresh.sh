#!/usr/bin/env bash
# LC_ALL=C pins collation for the ASCII-range validations below.
export LC_ALL=C

# Shared helpers (esc, comment stripping).
# shellcheck disable=SC1091  # sourced next to this file, linted directly
source "${BASH_SOURCE[0]%/*}/pinlib.sh"
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

# Only real code lines constitute the pin: block comments, line
# comments, and their decoy literals are all ignored (shared stripper).
origin_swift="$(git show "origin/${DEFAULT_BRANCH}:Package.swift" 2>/dev/null)" \
  || { echo "::error::cannot read origin/${DEFAULT_BRANCH}:Package.swift"; exit 1; }
origin_code="$(strip_swift_comments "${origin_swift}")"
if [ -z "${origin_code}" ]; then
  echo "::error::origin/${DEFAULT_BRANCH}:Package.swift carries no code lines"
  exit 1
fi

# Exactly one releaseTag and one checksum assignment must exist (any
# version); anything else is a mangled pin state -> fail closed.
tag_assignments="$(printf '%s\n' "${origin_code}" | grep -cE '^let releaseTag = "wallet-bindings-v' || true)"
checksum_assignments="$(printf '%s\n' "${origin_code}" | grep -cE '^checksum: "' || true)"
if [ "${tag_assignments}" != "1" ] || [ "${checksum_assignments}" != "1" ]; then
  echo "::error::origin/${DEFAULT_BRANCH} carries ${tag_assignments} releaseTag and ${checksum_assignments} checksum assignments; expected exactly one of each"
  exit 1
fi
# Fixed-string whole-line comparison (TAG may contain ERE metacharacters
# such as '+').
pin_tag="$(printf '%s\n' "${origin_code}" \
  | grep -E '^let releaseTag = ' | grep -oE 'wallet-bindings-v[^"]*')"
pin_sum_raw="$(printf '%s\n' "${origin_code}" \
  | grep -E '^checksum: ' | head -1 | grep -oE '"[^"]*"' | tr -d '"' || true)"
pin_sum="$(printf '%s\n' "${pin_sum_raw}" | grep -oE '^[0-9a-f]{64}$' || true)"
if [ "${pin_tag}" = "${TAG}" ]; then
  if [ -z "${pin_sum}" ]; then
    echo "::error::origin/${DEFAULT_BRANCH} carries a mangled checksum line: $(esc "${pin_sum_raw}")"
    exit 1
  fi
  if [ "${pin_sum}" = "${XCSUM}" ]; then
    echo "rebuilt artifacts match the merged pin; asset refresh proceeds safely"
    exit 0
  fi
  echo "::error::rebuilt artifacts (checksum $(esc "${XCSUM}")) do not match the pin merged on $(esc "${DEFAULT_BRANCH}") ($(esc "${pin_sum}")); release assets left untouched - repair manually"
  exit 1
fi
# A pin for a DIFFERENT release is the normal forward-release state:
# open-spm-pr.sh refuses to create rewind pin PRs, so no pin constrains
# this tag and the asset refresh is safe.
echo "no pin for ${TAG} on ${DEFAULT_BRANCH}; asset refresh allowed"
