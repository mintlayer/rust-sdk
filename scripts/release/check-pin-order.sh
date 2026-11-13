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

BASE_BRANCH="${1:?base branch required}"
if [[ ! "${BASE_BRANCH}" =~ ^[A-Za-z0-9][A-Za-z0-9._/-]*$ ]]; then
  echo "::error::unexpected base branch: ${BASE_BRANCH}"
  exit 1
fi

esc() {
  # Strip control and non-ASCII bytes (pin tags are ASCII), then
  # %-escape for workflow-command safety.
  local s
  s="$(printf '%s' "$1" | tr -d '\000-\037\177\200-\377')"
  s="${s//%/%25}"
  printf '%s' "$s"
}

# shellcheck disable=SC1091  # sourced next to this file, linted directly
source "${BASH_SOURCE[0]%/*}/semver.sh"

# Anchored, comment-stripped, exactly-one extraction — the same shape
# the release scripts use on the default branch.
strip_comments() { sed 's#//.*$##'; }

incoming_matches="$(strip_comments < Package.swift \
  | grep -cE '^let releaseTag = "wallet-bindings-v[^"]*"$' || true)"
if [ "${incoming_matches}" != "1" ]; then
  echo "::error::expected exactly one releaseTag assignment on the PR head, found ${incoming_matches}"
  exit 1
fi
incoming_tag="$(strip_comments < Package.swift \
  | grep -E '^let releaseTag = ' | grep -oE 'wallet-bindings-v[^"]*')"
incoming_version="${incoming_tag#wallet-bindings-v}"
if [[ ! "${incoming_version}" =~ ^[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::unparseable incoming pin version: $(esc "${incoming_version}")"
  exit 1
fi

origin_swift="$(git show "origin/${BASE_BRANCH}:Package.swift" 2>/dev/null)" \
  || { echo "::error::cannot read origin/${BASE_BRANCH}:Package.swift"; exit 1; }
origin_code="$(printf '%s\n' "${origin_swift}" | strip_comments)"
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
