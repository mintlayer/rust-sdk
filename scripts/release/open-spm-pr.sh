#!/usr/bin/env bash
# LC_ALL=C pins collation for the semver string comparisons below.
export LC_ALL=C

# Shared helpers (also used by check-pin-order.sh and
# guard-release-refresh.sh) and the semver precedence comparator (used
# by check-pin-order.sh).
# shellcheck disable=SC1091  # sourced next to this file, linted directly
source "${BASH_SOURCE[0]%/*}/pinlib.sh"
# shellcheck disable=SC1091  # sourced next to this file, linted directly
source "${BASH_SOURCE[0]%/*}/semver.sh"
# Open (or reuse) the SPM checksum-pin pull request for a release tag.
#
# Branch protection on the default branch rejects direct pushes (GH013),
# so the checksum pin lands via a pull request that a maintainer merges
# to activate SPM resolution for the release. Every step is idempotent so
# the workflow can be re-run after a transient failure: an open pin PR is
# updated, an already-merged pin (identical Package.swift on the default
# branch) short-circuits successfully.
#
# The branch namespace "pin-spm-checksum-*" is reserved for this script:
# it force-pushes the branch as disposable bot state on every run.
#
# Usage: open-spm-pr.sh <TAG> <VERSION> <XCSUM> <DEFAULT_BRANCH>
# Requires: GH_TOKEN, GITHUB_REPOSITORY (provided by Actions).
set -euo pipefail

TAG="${1:?TAG required}"
VERSION="${2:?VERSION required}"
XCSUM="${3:?XCSUM required}"
DEFAULT_BRANCH="${4:?DEFAULT_BRANCH required}"

BRANCH="pin-spm-checksum-${VERSION}"
# Mirror the workflow's validation so direct/manual invocations get the
# same fail-closed guarantees as the release run. This runs before
# check-ref-format so git's raw multi-line stderr (which may contain
# unescaped, attacker-influenced bytes) can never reach the log.
if [[ ! "${TAG}" =~ ^wallet-bindings-v[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::tag does not match wallet-bindings-v<semver>"
  exit 1
fi
if [[ ! "${VERSION}" =~ ^[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::version does not match <semver>"
  exit 1
fi

git check-ref-format --branch "${BRANCH}"

if [[ ! "${XCSUM}" =~ ^[0-9a-f]{64}$ ]] || [ "${XCSUM//0/}" = "" ]; then
  echo "::error::checksum is not 64 lowercase hex characters (all-zero refused)"
  exit 1
fi
if [[ ! "${DEFAULT_BRANCH}" =~ ^[A-Za-z0-9][A-Za-z0-9._/-]*$ ]]; then
  echo "::error::unexpected default branch format: $(esc "${DEFAULT_BRANCH}")"
  exit 1
fi
# The default branch must never live in the reserved bot namespace: a
# force-push there would rewind main instead of a disposable branch.
if [[ "${DEFAULT_BRANCH}" == pin-spm-checksum-* ]]; then
  echo "::error::default branch must not use the reserved pin-spm-checksum-* namespace"
  exit 1
fi
if [ -z "${GITHUB_REPOSITORY:-}" ]; then
  echo "::error::GITHUB_REPOSITORY is not set"
  exit 1
fi
owner="${GITHUB_REPOSITORY%%/*}"
if [[ ! "${owner}" =~ ^[A-Za-z0-9][A-Za-z0-9-]*$ ]]; then
  echo "::error::unexpected repository owner format: $(esc "${owner}")"
  exit 1
fi

# The remaining git/grep logic assumes the repository root; enforce it
# before touching the checkout's git config.
if [ ! -e .git ] || [ ! -f Package.swift ]; then
  echo "::error::run this script from the repository root"
  exit 1
fi

git config user.name "mintlayer-release-bot"
git config user.email "hello@mintlayer.org"

# TAG and VERSION must describe the same release.
if [ "${TAG}" != "wallet-bindings-v${VERSION}" ]; then
  echo "::error::TAG (${TAG}) and VERSION (${VERSION}) do not describe the same release"
  exit 1
fi

# The tag must sit on the default branch's history; otherwise the pin PR
# would carry unrelated commits. Fail loudly before any push. Unshallow
# only when needed, and keep stderr visible for genuine fetch failures.
if [ "$(git rev-parse --is-shallow-repository)" = "true" ]; then
  git fetch --unshallow origin "+refs/heads/${DEFAULT_BRANCH}:refs/remotes/origin/${DEFAULT_BRANCH}"
else
  git fetch origin "+refs/heads/${DEFAULT_BRANCH}:refs/remotes/origin/${DEFAULT_BRANCH}"
fi
merge_rc=0
git merge-base --is-ancestor "refs/tags/${TAG}" "origin/${DEFAULT_BRANCH}" || merge_rc=$?
if [ "${merge_rc}" -gt 1 ]; then
  echo "::error::failed to compare $(esc "${TAG}") against origin/${DEFAULT_BRANCH} (git exit ${merge_rc}); check the fetch and the tag"
  exit 1
fi
if [ "${merge_rc}" -eq 1 ]; then
  echo "::error::tag $(esc "${TAG}") is not an ancestor of $(esc "${DEFAULT_BRANCH}"); cut the tag from the default branch tip"
  exit 1
fi

# Manual invocations must run from the tagged commit, or the pin commit
# would be built (and force-pushed) on the wrong base.
tag_commit="$(git rev-parse "refs/tags/${TAG}^{commit}")"
if [ "$(git rev-parse HEAD)" != "${tag_commit}" ]; then
  echo "::error::HEAD is not the tagged commit; run from a checkout of $(esc "${TAG}")"
  exit 1
fi

git checkout -B "${BRANCH}"
git add Package.swift
# Both substitutions must be present, on every path: anything else means
# the sed no-op'd or corrupted the file; fail closed.
# Anchored, exactly-once verification with fixed strings (TAG may
# contain ERE metacharacters such as '+'): strip comments and trailing
# whitespace, then count whole-line matches.
read -r pin_tag_lines pin_sum_lines checksum_lines <<< "$(count_pin_lines Package.swift "${TAG}" "${XCSUM}")"
if [ "${pin_tag_lines}" != "1" ] || [ "${pin_sum_lines}" != "1" ] || [ "${checksum_lines}" != "1" ]; then
  echo "::error::Package.swift does not carry exactly one expected release tag and checksum assignment after substitution"
  exit 1
fi
# An identical Package.swift on the default branch means the pin PR was
# already merged; short-circuit instead of attempting a create that
# GitHub would reject with "No commits between".
if git diff --quiet "origin/${DEFAULT_BRANCH}" -- Package.swift; then
  echo "checksum pin is already merged into ${DEFAULT_BRANCH}; nothing to do"
  exit 0
fi
# Empty staged diff = the tag commit itself carries the pin. That is
# only benign when the default branch agrees (handled above); if the
# default branch differs, the pin on main was reverted and the release
# is effectively unpinned. Fail closed rather than claim success.
if git diff --cached --quiet; then
  echo "::error::the tag commit carries the pin but origin/${DEFAULT_BRANCH} does not; the pin was reverted on the default branch and needs manual repair"
  exit 1
fi
# The default branch carrying an ACTIVE pin for a NEWER-or-equal
# release means this invocation would rewind that pin; refuse. Older
# active pins are the normal forward-release state and proceed.
origin_swift="$(git show "origin/${DEFAULT_BRANCH}:Package.swift" 2>/dev/null)" \
  || { echo "::error::cannot read origin/${DEFAULT_BRANCH}:Package.swift"; exit 1; }
# Only real code lines constitute the pin: block comments, line
# comments, and their decoy literals are all ignored.
origin_code="$(strip_swift_comments "${origin_swift}")"
# Canonical count uses the same whole-line anchor as the merge gate,
# so trailing garbage on the canonical line itself (e.g. a stray 'x'
# after the closing quote) refuses here exactly like it refuses at
# merge time.
tag_assignments="$(printf '%s\n' "${origin_code}" | grep -cE '^let releaseTag = "wallet-bindings-v[^"]*"$' || true)"
sum_assignments="$(printf '%s\n' "${origin_code}" | grep -cE '^checksum: "' || true)"
# Pin-bearing occurrence count, mirroring the merge gate: extras beside
# the canonical assignment (broken prefix, releaseTagX variants) would
# open a pin PR the gate is guaranteed to reject. The download-URL line
# never contains the substring 'let releaseTag'.
tag_bearing="$(printf '%s\n' "${origin_code}" | { grep -o 'let releaseTag' || true; } | wc -l | tr -d ' ')"
if [ "${tag_assignments}" != "1" ] || [ "${sum_assignments}" != "1" ] || [ "${tag_bearing}" != "1" ]; then
  echo "::error::origin/${DEFAULT_BRANCH} carries ${tag_assignments} canonical releaseTag assignments, ${tag_bearing} pin-bearing line(s) and ${sum_assignments} checksum assignments; expected exactly one canonical of each"
  exit 1
fi
# Same extraction/validation the merge gate uses: the whole tag token,
# then the full version regex — a hand-mangled 'wallet-bindings-vv1.2.3'
# must refuse here exactly like it refuses at merge time.
origin_pin_tag="$(printf '%s\n' "${origin_code}" \
  | grep -E '^let releaseTag = ' | grep -oE 'wallet-bindings-v[^"]*')"
origin_pin_sum_raw="$(printf '%s\n' "${origin_code}" \
  | grep -E '^checksum: "' | head -1 | grep -oE '"[^"]*"' | tr -d '"' || true)"
origin_pin_sum="$(printf '%s\n' "${origin_pin_sum_raw}" \
  | grep -oE '^[0-9a-f]{64}$' || true)"
origin_pin_active=0
origin_version=""
if [ -n "${origin_pin_sum}" ] && [ "${origin_pin_sum}" != "0000000000000000000000000000000000000000000000000000000000000000" ]; then
  origin_pin_active=1
fi
# The release marker must be parsable whenever the assignment exists,
# active or deactivated: both gates rely on the version for ordering.
# An empty extraction (degenerate 'wallet-bindings-v' line) is itself
# unparseable.
if [ -n "${origin_pin_tag}" ]; then
  origin_version="${origin_pin_tag#wallet-bindings-v}"
fi
if [[ ! "${origin_version}" =~ ^[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::origin/${DEFAULT_BRANCH} carries an unparseable pin tag: $(esc "${origin_pin_tag}")"
  exit 1
fi
if [ "${origin_pin_active}" = "1" ]; then
  cmp="$(semver_cmp "${VERSION}" "${origin_version}")"
  if [[ ! "${cmp}" =~ ^-?[012]$ ]]; then
    echo "::error::semver comparison failed for ${VERSION} against ${origin_version}"
    exit 1
  fi
  if [ "${cmp}" -le 0 ]; then
    echo "::error::origin/${DEFAULT_BRANCH} already pins $(esc "${origin_pin_tag}") (>= ${VERSION}); refusing to rewind the pin"
    exit 1
  fi
elif [ -n "${origin_pin_sum_raw}" ] && [ -z "${origin_pin_sum}" ]; then
  # A checksum line exists but is not 64 lowercase hex: mangled pin.
  echo "::error::origin/${DEFAULT_BRANCH} carries a mangled checksum line: $(esc "${origin_pin_sum_raw}")"
  exit 1
elif [ -n "${origin_pin_tag}" ]; then
  # Not active, not checksum-mangled: either a deactivated
  # (zeroed-checksum) pin or a tag line without any checksum line.
  # Both refuse; the deactivated path orders with the comparator
  # validated out-of-band, so a malformed result refuses instead of
  # slipping past the guard.
  if [ -z "${origin_pin_sum_raw}" ]; then
    echo "::error::origin/${DEFAULT_BRANCH} carries a releaseTag without a checksum line: $(esc "${origin_pin_tag}")"
    exit 1
  fi
  cmp="$(semver_cmp "${VERSION}" "${origin_version}")"
  if [[ ! "${cmp}" =~ ^-?[012]$ ]]; then
    echo "::error::semver comparison failed for ${VERSION} against ${origin_version}"
    exit 1
  fi
  if [ "${cmp}" -le 0 ]; then
    echo "::error::origin/${DEFAULT_BRANCH} carries a deactivated pin for ${origin_version} (>= ${VERSION}); refusing to pin - repair the default branch manually"
    exit 1
  fi
fi
git commit -m "chore: pin the SPM checksum to ${TAG}"

# Preflight canary: fail fast when the PR-state lookups are broken
# before anything is mutated. The value itself is discarded (hence the
# deliberate shellcheck disable below) — the reuse-vs-create decision
# is made by the post-push re-lookup.
# Only an OPEN pull request against the default branch from this
# repository's own branch counts. Fork PRs, same-owner cross-repo PRs,
# and PRs targeting a non-default base must not suppress the genuine bot
# PR. The owner name is validated before it is interpolated into the jq
# program; --paginate keeps the check independent of list windows.
# shellcheck disable=SC2034  # canary: value intentionally discarded
preflight_pr="$(gh pr list --head "${BRANCH}" --state open --paginate \
  --json number,headRepositoryOwner,isCrossRepository,baseRefName \
  --jq "[.[] | select(.headRepositoryOwner.login == \"${owner}\" and .isCrossRepository == false and .baseRefName == \"${DEFAULT_BRANCH}\")][0].number // empty")" \
  || { echo "::error::failed to list existing pin PRs"; exit 1; }

# Supersede strictly-older open pin PRs and refuse when a strictly
# NEWER release's pin PR is still open (merging it after this one would
# rewind the pin; the concurrency group serializes runs, not queue
# order). Runs on both the create and reuse paths. Close failures fail
# the run - a stale open pin PR is a rewind hazard. Operator note:
# closes happen BEFORE the push, so a push failure leaves a transient
# zero-open-pin-PR window; the direction is safe (main stays pinned to
# the older release) and the next run is idempotent.
superseded=""
# Same trust filters as the existing_pr lookup: fork PRs, same-owner
# cross-repo PRs, and PRs against a non-default base never constrain
# this run even when their branch names collide with the namespace.
old_prs="$(gh pr list --state open --paginate \
  --json number,headRefName,headRepositoryOwner,isCrossRepository,baseRefName \
  --jq ".[] | select((.headRefName | startswith(\"pin-spm-checksum-\")) and .headRefName != \"${BRANCH}\" and .headRepositoryOwner.login == \"${owner}\" and .isCrossRepository == false and .baseRefName == \"${DEFAULT_BRANCH}\") | \"\(.number)\t\(.headRefName)\"")" \
  || { echo "::error::failed to list open pin PRs for superseding"; exit 1; }
while IFS=$'\t' read -r old_pr old_branch; do
  [ -n "${old_pr}" ] || continue
  [ "${old_branch}" = "${BRANCH}" ] && continue
  old_version="${old_branch#pin-spm-checksum-}"
  # A pre-automation branch carried a bare v (pin-spm-checksum-v0.1.0);
  # tolerate the suffix so a stale legacy PR cannot hard-block releases.
  old_version="${old_version#v}"
  # Defensive: the jq filter already excludes BRANCH itself, so a
  # same-version row should be unreachable; skip rather than mis-handle.
  if [ "${old_version}" = "${VERSION}" ]; then continue; fi
  if [[ ! "${old_version}" =~ ^[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$ ]]; then
    echo "::error::open pin PR #${old_pr} carries an unparseable version: $(esc "${old_version}")"
    exit 1
  fi
  cmp_old="$(semver_cmp "${VERSION}" "${old_version}")"
  if [[ ! "${cmp_old}" =~ ^-?[012]$ ]]; then
    echo "::error::could not compare ${VERSION} against open pin PR #${old_pr} ($(esc "${old_version}"))"
    exit 1
  fi
  if [ "${cmp_old}" -le 0 ]; then
    echo "::error::open pin PR #${old_pr} pins $(esc "${old_version}") (>= ${VERSION}); close or merge it before releasing ${VERSION}"
    exit 1
  fi
  gh pr close "${old_pr}" \
    --comment "Superseded by the ${TAG} checksum-pin PR: an older pin must not merge after a newer one."
  superseded="${superseded} #${old_pr}"
done << OLD_PRS
${old_prs}
OLD_PRS
[ -z "${superseded}" ] || echo "superseded older open pin PR(s):${superseded}"

# The branch is disposable bot state; force-with-lease-like semantics
# are unnecessary because the list above is authoritative: the push
# happens after the existence decision so a spoofed PR never references
# the bot commit.
git push --force origin "${BRANCH}"

# Re-run the open-PR lookup: the PR found before the supersede loop may
# have been merged by a maintainer while this run was in flight. In
# that window an "updated its head branch" verdict would end the run
# green with NO open pin PR for the release, so the post-push state is
# authoritative for the reuse-vs-create decision.
existing_pr="$(gh pr list --head "${BRANCH}" --state open --paginate \
  --json number,headRepositoryOwner,isCrossRepository,baseRefName \
  --jq "[.[] | select(.headRepositoryOwner.login == \"${owner}\" and .isCrossRepository == false and .baseRefName == \"${DEFAULT_BRANCH}\")][0].number // empty")" \
  || { echo "::error::failed to re-list existing pin PRs after the push"; exit 1; }

if [ -n "${existing_pr}" ]; then
  echo "checksum-pin PR already exists (#${existing_pr}); updated its head branch"
else
  gh pr create \
    --base "${DEFAULT_BRANCH}" \
    --head "${BRANCH}" \
    --title "chore: pin the SPM checksum to ${TAG}" \
    --body "Attaches the released XCFramework checksum to the root Package.swift so SPM resolves the ${TAG} artifact.

Checksum: \`${XCSUM}\`

The GitHub release for this tag is published before this PR is opened.
Merge to activate."
fi