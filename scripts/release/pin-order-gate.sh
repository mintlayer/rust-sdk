#!/usr/bin/env bash
# CI-side pin-order gate (invoked by .github/workflows/ci.yml).
#
# Applicability is diff-driven: every pull request or push whose diff
# touches Package.swift is checked; anything else exits 0 ("nothing to
# check"). On pull_request events the PR-head fetch below necessarily
# precedes the diff, so "no network beyond that fetch" is the accurate
# claim for skipped diffs. For an applicable diff:
#   1. The head's single releaseTag + hex checksum pin is extracted
#      with the shared stripper (pinlib.sh).
#   2. The checksum is bound to the release the pin names: the
#      release's published SHA256SUMS must list exactly that hash for
#      the XCFramework zip (SPM verifies at resolve time, so an
#      unverifiable pin would strand every consumer).
#   3. Ordering is delegated to check-pin-order.sh: reserved-namespace
#      heads (pin-spm-checksum-*) must pin exactly the release their
#      branch name declares; any other head is order-checked without a
#      tag-equality requirement.
#
# Environment (set by ci.yml): BASE_REF, HEAD_SHA, HEAD_REF,
# EVENT_NAME, PR_NUMBER, GH_TOKEN. LC_ALL=C is enforced internally.
set -euo pipefail
export LC_ALL=C

# Shared helpers (esc, comment stripping).
# shellcheck disable=SC1091  # sourced next to this file, linted directly
source "${BASH_SOURCE[0]%/*}/pinlib.sh"

BASE_REF="${BASE_REF:?base branch required}"
HEAD_SHA="${HEAD_SHA:?head sha required}"
HEAD_REF="${HEAD_REF:?head ref required}"
EVENT_NAME="${EVENT_NAME:-}"
PR_NUMBER="${PR_NUMBER:-}"

if [[ ! "${BASE_REF}" =~ ^[A-Za-z0-9][A-Za-z0-9._/-]*$ ]] \
  || [[ "${BASE_REF}" == pin-spm-checksum-* ]]; then
  echo "::error::unexpected base branch: $(esc "${BASE_REF}")"
  exit 1
fi

# The checkout is the base branch; pull the PR head commit so the diff
# and the data-only read of the head's Package.swift can see it (push
# events already have their SHA locally). Anonymous on public repos;
# private repositories fail closed below rather than skipping.
if [ "${EVENT_NAME}" = "pull_request" ]; then
  git fetch --no-tags origin "+refs/pull/${PR_NUMBER}/head" \
    || { echo "::error::cannot fetch the PR head"; exit 1; }
fi

# Fail closed on diff errors: an unreadable head (e.g. a force-push
# landing between event emission and the fetch) must not masquerade as
# "Package.swift untouched".
if ! head_files="$(git diff --name-only "origin/${BASE_REF}...${HEAD_SHA}")"; then
  echo "::error::cannot diff the incoming pin against origin/${BASE_REF}"
  exit 1
fi
if ! printf '%s\n' "${head_files}" | grep -q '^Package.swift$'; then
  echo "Package.swift untouched; nothing to check"
  exit 0
fi

# Strict tag equality only for the reserved bot namespace; other
# heads are order-checked without a tag-equality requirement.
case "${HEAD_REF}" in
  pin-spm-checksum-*)
    # A pre-automation branch carried a bare v in the suffix; strip it
    # so legacy branches still derive the correct expected tag.
    gate_suffix="${HEAD_REF#pin-spm-checksum-}"
    expected_tag="wallet-bindings-v${gate_suffix#v}"
    if [[ ! "${expected_tag}" =~ ^wallet-bindings-v[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
      echo "::error::derived tag does not match wallet-bindings-v<semver>: $(esc "${expected_tag}")"
      exit 1
    fi
    ;;
  *)
    expected_tag=""
    ;;
esac

# The head's Package.swift is consumed purely as data via its SHA.
head_swift="$(git show "${HEAD_SHA}:Package.swift" 2>/dev/null)" \
  || { echo "::error::cannot read Package.swift from the PR head"; exit 1; }
head_norm="$(strip_swift_comments "${head_swift}")"
# Anchored counts: the URL line interpolates \(releaseTag), so a
# substring 'releaseTag' count would double-count the real shape.
head_tag="$(printf '%s\n' "${head_norm}" | grep -E '^let releaseTag = ' | grep -oE 'wallet-bindings-v[^"]*' || true)"
head_checksum="$(printf '%s\n' "${head_norm}" | grep -E '^checksum: "' | grep -oE '[0-9a-f]{64}' || true)"
if [ "$(printf '%s\n' "${head_norm}" | grep -cE '^let releaseTag = ' || true)" != "1" ] \
  || [ "$(printf '%s\n' "${head_norm}" | grep -cE '^checksum: "[0-9a-f]{64}"$' || true)" != "1" ] \
  || [[ ! "${head_tag}" =~ ^wallet-bindings-v[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::PR head does not carry exactly one releaseTag and one hex checksum pin"
  exit 1
fi

# Bind the checksum to the published release; fail closed on a
# missing or unreadable manifest.
sums="$(gh release download "${head_tag}" --pattern "SHA256SUMS" --output - 2>/dev/null)" \
  || { echo "::error::release $(esc "${head_tag}") is missing or carries no readable SHA256SUMS; refusing an unverifiable pin"; exit 1; }
published="$(printf '%s\n' "${sums}" | grep 'MintlayerWalletBindings.xcframework.zip' | awk '{print $1}' || true)"
if [ "${published}" != "${head_checksum}" ]; then
  echo "::error::pinned checksum does not match the published SHA256SUMS for $(esc "${head_tag}")"
  exit 1
fi

exec "${BASH_SOURCE[0]%/*}/check-pin-order.sh" "${BASE_REF}" "${expected_tag}" "${HEAD_SHA}"
