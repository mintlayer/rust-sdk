#!/usr/bin/env bash
# Merge-time pin-order gate for checksum-pin pull requests. Refuses a
# pin PR whose releaseTag would not advance the release currently
# pinned on the default branch (same-version repairs are manual: the
# rewind guard in open-spm-pr.sh refuses to create them).
#
# Usage: check-pin-order.sh <BASE_BRANCH>   (run from the repo root, in
# the PR checkout; LC_ALL=C is enforced internally).
set -euo pipefail
export LC_ALL=C

# Shared helpers (esc, comment stripping) and the semver comparator.
# shellcheck disable=SC1091  # sourced next to this file, linted directly
source "${BASH_SOURCE[0]%/*}/pinlib.sh"
# shellcheck disable=SC1091  # sourced next to this file, linted directly
source "${BASH_SOURCE[0]%/*}/semver.sh"

BASE_BRANCH="${1:?base branch required}"
EXPECTED_TAG="${2:-}"
if [[ ! "${BASE_BRANCH}" =~ ^[A-Za-z0-9][A-Za-z0-9._/-]*$ ]] \
  || [[ "${BASE_BRANCH}" == pin-spm-checksum-* ]]; then
  echo "::error::unexpected base branch: $(esc "${BASE_BRANCH}")"
  exit 1
fi
if [ -n "${EXPECTED_TAG}" ] && [[ ! "${EXPECTED_TAG}" =~ ^wallet-bindings-v[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::unexpected expected tag: $(esc "${EXPECTED_TAG}")"
  exit 1
fi

normalized_in="$(strip_swift_comments "$(cat Package.swift)")"
# Strict mode when the expected tag is known (ci.yml derives it from
# the reserved bot branch name): the normalized code must contain
# exactly that assignment and NO other releaseTag- or checksum-bearing
# line, so comment/nesting text tricks fail closed.
if [ -n "${EXPECTED_TAG}" ]; then
  expected_lines="$(printf '%s\n' "${normalized_in}" | grep -cF -x -- "let releaseTag = \"${EXPECTED_TAG}\"" || true)"
  release_tag_lines="$(printf '%s\n' "${normalized_in}" | grep -cE 'releaseTag' || true)"
  checksum_lines="$(printf '%s\n' "${normalized_in}" | grep -cE 'checksum:' || true)"
  if [ "${expected_lines}" != "1" ] || [ "${release_tag_lines}" != "1" ] || [ "${checksum_lines}" != "1" ]; then
    echo "::error::PR head does not carry exactly the expected pin assignment for $(esc "${EXPECTED_TAG}") (ambiguous or missing)"
    exit 1
  fi
fi
incoming_matches="$(printf '%s\n' "${normalized_in}" \
  | grep -cE '^let releaseTag = "wallet-bindings-v[^"]*"$' || true)"
if [ "${incoming_matches}" != "1" ]; then
  echo "::error::expected exactly one releaseTag assignment on the PR head, found ${incoming_matches}"
  exit 1
fi
incoming_tag="$(printf '%s\n' "${normalized_in}" \
  | grep -E '^let releaseTag = ' | grep -oE 'wallet-bindings-v[^"]*')"
incoming_version="${incoming_tag#wallet-bindings-v}"
if [[ ! "${incoming_version}" =~ ^[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::unparseable incoming pin version: $(esc "${incoming_version}")"
  exit 1
fi

origin_swift="$(git show "origin/${BASE_BRANCH}:Package.swift" 2>/dev/null)" \
  || { echo "::error::cannot read origin/${BASE_BRANCH}:Package.swift"; exit 1; }
origin_code="$(strip_swift_comments "${origin_swift}")"
current_matches="$(printf '%s\n' "${origin_code}" \
  | grep -cE '^let releaseTag = "wallet-bindings-v[^"]*"$' || true)"
if [ "${current_matches}" = "0" ]; then
  echo "no pin on ${BASE_BRANCH}; any pin order is acceptable"
  exit 0
fi
if [ "${current_matches}" != "1" ]; then
  echo "::error::expected at most one releaseTag assignment on ${BASE_BRANCH}, found ${current_matches}"
  exit 1
fi
current_tag="$(printf '%s\n' "${origin_code}" \
  | grep -E '^let releaseTag = ' | grep -oE 'wallet-bindings-v[^"]*')"
current_version="${current_tag#wallet-bindings-v}"
if [[ ! "${current_version}" =~ ^[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::unparseable current pin version: $(esc "${current_version}")"
  exit 1
fi

cmp="$(semver_cmp "${incoming_version}" "${current_version}")"
if [[ ! "${cmp}" =~ ^-?[012]$ ]]; then
  echo "::error::semver comparison failed for ${incoming_version} against ${current_version}"
  exit 1
fi
# Only a strictly newer release advances the pin; equal or lower
# (rewind or same-version repair) is refused at merge time.
if [ "${cmp}" -le 0 ]; then
  echo "::error::pin PR would not advance the release (incoming ${incoming_tag}, current ${current_tag})"
  exit 1
fi
echo "pin order ok (incoming ${incoming_tag}, current ${current_tag})"
