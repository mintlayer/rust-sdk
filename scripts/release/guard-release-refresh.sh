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

if [[ ! "${DEFAULT_BRANCH}" =~ ^[A-Za-z0-9][A-Za-z0-9._/-]*$ ]] \
  || [[ "${DEFAULT_BRANCH}" == pin-spm-checksum-* ]]; then
  echo "::error::unexpected default branch: ${DEFAULT_BRANCH}"
  exit 1
fi

pin_sum="$(git show "origin/${DEFAULT_BRANCH}:Package.swift" 2>/dev/null \
  | grep -oE 'checksum: "[0-9a-f]{64}"' | head -1 | grep -oE '[0-9a-f]{64}' || true)"
pin_tag="$(git show "origin/${DEFAULT_BRANCH}:Package.swift" 2>/dev/null \
  | grep -oE 'wallet-bindings-v[^"]*' | head -1 || true)"

# Only a pin for THIS release constrains the refresh; identical
# checksums make the refresh a harmless no-op.
if [ "${pin_tag}" = "${TAG}" ] && [ -n "${pin_sum}" ] && [ "${pin_sum}" != "${XCSUM}" ]; then
  echo "::error::rebuilt artifacts (checksum ${XCSUM}) do not match the pin merged on ${DEFAULT_BRANCH} (${pin_sum}); release assets left untouched - repair manually"
  exit 1
fi
