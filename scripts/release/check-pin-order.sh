#!/usr/bin/env bash
# Merge-time pin-order gate for checksum-pin pull requests. Refuses a
# pin PR whose releaseTag would not advance the release currently
# pinned on the default branch (same-version repairs are manual: the
# rewind guard in open-spm-pr.sh refuses to create them).
#
# Usage: check-pin-order.sh <BASE_BRANCH> [EXPECTED_TAG] [INCOMING_REF]
# Run from the repo root in the PR checkout; LC_ALL=C is enforced
# internally. With EXPECTED_TAG (derived by ci.yml from the reserved
# bot branch name), strict mode requires the incoming checkout
# (INCOMING_REF, else the working tree) to carry exactly that
# assignment. With EXPECTED_TAG empty the script enforces pin
# structure and order only; the caller must then bind the incoming
# checksum to the release it names (ci.yml checks it against the
# release's published SHA256SUMS).
#
# Residual merge-time race: this gate decides against origin/<base> as
# of its own run. A pin PR merging AFTER the gate passes but BEFORE a
# rewind PR merges is invisible here; operators must enable "require
# branches to be up to date before merging" for this required check on
# the default branch to close that window.
set -euo pipefail
export LC_ALL=C

# Shared helpers (esc, comment stripping) and the semver comparator.
# shellcheck disable=SC1091  # sourced next to this file, linted directly
source "${BASH_SOURCE[0]%/*}/pinlib.sh"
# shellcheck disable=SC1091  # sourced next to this file, linted directly
source "${BASH_SOURCE[0]%/*}/semver.sh"

BASE_BRANCH="${1:?base branch required}"
EXPECTED_TAG="${2-}"
INCOMING_REF="${3:-}"
if [[ ! "${BASE_BRANCH}" =~ ^[A-Za-z0-9][A-Za-z0-9._/-]*$ ]] \
  || [[ "${BASE_BRANCH}" == pin-spm-checksum-* ]]; then
  echo "::error::unexpected base branch: $(esc "${BASE_BRANCH}")"
  exit 1
fi
if [ -n "${EXPECTED_TAG}" ] && [[ ! "${EXPECTED_TAG}" =~ ^wallet-bindings-v[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::unexpected expected tag: $(esc "${EXPECTED_TAG}")"
  exit 1
fi

# The incoming pin comes from the PR head SHA (workflow) or the working
# tree (legacy/harness) — always consumed as data by this trusted code.
if [ -n "${INCOMING_REF}" ]; then
  normalized_in="$(strip_swift_comments "$(git show "${INCOMING_REF}:Package.swift" 2>/dev/null)")" || {
    echo "::error::cannot read Package.swift from $(esc "${INCOMING_REF}")"
    exit 1
  }
else
  if [ ! -r Package.swift ]; then
    echo "::error::cannot read Package.swift"
    exit 1
  fi
  normalized_in="$(strip_swift_comments "$(cat Package.swift)")"
fi
# Strict mode (EXPECTED_TAG non-empty; ci.yml derives it from the
# reserved bot branch name): the normalized code must contain exactly
# that assignment and NO other releaseTag- or checksum-bearing line,
# so comment/nesting text tricks fail closed. Ordering-only mode
# (EXPECTED_TAG empty) skips the tag-equality count; the shared
# structural counts below still apply.
if [ -n "${EXPECTED_TAG}" ]; then
  expected_lines="$(printf '%s\n' "${normalized_in}" | grep -cF -x -- "let releaseTag = \"${EXPECTED_TAG}\"" || true)"
else
  expected_lines="1"
fi
# Anchored counts: the URL line interpolates \(releaseTag), so a
# substring 'releaseTag' count would double-count the real shape.
release_tag_lines="$(printf '%s\n' "${normalized_in}" | grep -cE '^let releaseTag = ' || true)"
checksum_lines="$(printf '%s\n' "${normalized_in}" | grep -cE '^checksum: "' || true)"
checksum_valid="$(printf '%s\n' "${normalized_in}" | grep -cE '^checksum: "[0-9a-f]{64}"$' || true)"
if [ "${expected_lines}" != "1" ] || [ "${release_tag_lines}" != "1" ] \
  || [ "${checksum_lines}" != "1" ] || [ "${checksum_valid}" != "1" ]; then
  if [ -n "${EXPECTED_TAG}" ]; then
    echo "::error::PR head does not carry exactly the expected pin assignment for $(esc "${EXPECTED_TAG}") (ambiguous, missing, or non-hex checksum)"
  else
    echo "::error::PR head must carry exactly one releaseTag assignment and one hex checksum pin"
  fi
  exit 1
fi
incoming_tag="$(printf '%s\n' "${normalized_in}" \
  | grep -E '^let releaseTag = ' | grep -oE 'wallet-bindings-v[^"]*' || true)"
if [ -z "${incoming_tag}" ]; then
  echo "::error::PR head carries no parseable wallet-bindings tag"
  exit 1
fi
incoming_version="${incoming_tag#wallet-bindings-v}"
if [[ ! "${incoming_version}" =~ ^[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::unparseable incoming pin version: $(esc "${incoming_version}")"
  exit 1
fi

# Refresh origin/<base> so the comparison reflects the branch as of
# now, not the checkout snapshot: a pin PR merged while this gate is
# in flight must be visible before ordering is decided (a stale view
# could pass a rewind). Fatal on failure.
git fetch origin "+refs/heads/${BASE_BRANCH}:refs/remotes/origin/${BASE_BRANCH}" \
  || { echo "::error::cannot refresh origin/${BASE_BRANCH}"; exit 1; }
origin_swift="$(git show "origin/${BASE_BRANCH}:Package.swift" 2>/dev/null)" \
  || { echo "::error::cannot read origin/${BASE_BRANCH}:Package.swift"; exit 1; }
origin_code="$(strip_swift_comments "${origin_swift}")"
# Mangle detection is not prefix-anchored: ANY releaseTag-bearing code
# line on the base that is not the canonical assignment (broken prefix,
# stray suffix, doubled spaces) must refuse — otherwise a corrupted
# base would read as "no pin" and let a rewind pass.
current_bearing="$(printf '%s\n' "${origin_code}" | grep -c 'releaseTag' || true)"
current_matches="$(printf '%s\n' "${origin_code}" \
  | grep -cE '^let releaseTag = "wallet-bindings-v[^"]*"$' || true)"
if [ "${current_matches}" = "0" ]; then
  if [ "${current_bearing}" != "0" ]; then
    echo "::error::origin/${BASE_BRANCH} carries ${current_bearing} mangled pin line(s) and no well-formed one; refusing (repair manually)"
    exit 1
  fi
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
  echo "::error::pin PR would not advance the release (incoming $(esc "${incoming_tag}"), current $(esc "${current_tag}"))"
  exit 1
fi
echo "pin order ok (incoming $(esc "${incoming_tag}"), current $(esc "${current_tag}"))"
