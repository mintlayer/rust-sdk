#!/usr/bin/env bash
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

# Workflow-command values need %-/CR-/LF-escaping to stay one annotation.
esc() {
  local s="$1"
  s="${s//%/%25}"
  s="${s//$'\r'/%0D}"
  s="${s//$'\n'/%0A}"
  printf '%s' "$s"
}

# Mirror the workflow's validation so direct/manual invocations get the
# same fail-closed guarantees as the release run.
if [[ ! "${TAG}" =~ ^wallet-bindings-v[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::tag does not match wallet-bindings-v<semver>"
  exit 1
fi
if [[ ! "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::version does not match <semver>"
  exit 1
fi
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
owner="${GITHUB_REPOSITORY%%/*}"
if [[ ! "${owner}" =~ ^[A-Za-z0-9][A-Za-z0-9-]*$ ]]; then
  echo "::error::unexpected repository owner format: $(esc "${owner}")"
  exit 1
fi

# The remaining git/grep logic assumes the repository root; enforce it
# before touching the checkout's git config.
if [ ! -d .git ] || [ ! -f Package.swift ]; then
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
  echo "::error::failed to compare ${TAG} against origin/${DEFAULT_BRANCH} (git exit ${merge_rc}); check the fetch and the tag"
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
  echo "::error::HEAD is not the tagged commit; run from a checkout of ${TAG}"
  exit 1
fi

git checkout -B "${BRANCH}"
git add Package.swift
# Both substitutions must be present, on every path: anything else means
# the sed no-op'd or corrupted the file; fail closed.
if ! grep -qF "${XCSUM}" Package.swift \
  || ! grep -qF "let releaseTag = \"${TAG}\"" Package.swift; then
  echo "::error::Package.swift does not carry the expected release tag and checksum after substitution"
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
origin_pin_tag="$(git show "origin/${DEFAULT_BRANCH}:Package.swift" \
  | grep -oE 'let releaseTag = "wallet-bindings-v[^"]*"' \
  | grep -oE 'v[0-9][^"]*' || true)"
origin_pin_sum="$(git show "origin/${DEFAULT_BRANCH}:Package.swift" \
  | grep -oE 'checksum: "[0-9a-f]{64}"' | grep -oE '[0-9a-f]{64}' || true)"
origin_pin_sum_raw="$(git show "origin/${DEFAULT_BRANCH}:Package.swift" \
  | grep -oE 'checksum: "[^"]*"' | grep -oE '"[^"]*"' | tr -d '"' | head -1 || true)"
origin_pin_active=0
if [ -n "${origin_pin_sum}" ] && [ "${origin_pin_sum}" != "0000000000000000000000000000000000000000000000000000000000000000" ]; then
  origin_pin_active=1
fi
if [ "${origin_pin_active}" = "1" ]; then
  # An active (non-placeholder) pin requires a parsable tag line; a
  # missing/mangled tag line is itself a fail-closed state.
  origin_version="${origin_pin_tag#v}"
  if [ -z "${origin_pin_tag}" ] || [[ ! "${origin_version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
    echo "::error::origin/${DEFAULT_BRANCH} carries an unparseable pin tag: $(esc "${origin_pin_tag}")"
    exit 1
  fi
  # Semver precedence: numeric triple, then the pre-release rule (a
  # pre-release is less than the final release).
  semver_cmp() { # <0 if $1 < $2, 0 if equal, >0 if $1 > $2
    local a="${1%%-*}" b="${2%%-*}"
    local ap="" bp=""
    [[ "$1" == *-* ]] && ap="${1#*-}"
    [[ "$2" == *-* ]] && bp="${2#*-}"
    local -a A B
    read -ra A <<< "${a//[^0-9]/ }"
    read -ra B <<< "${b//[^0-9]/ }"
    local i
    for i in 0 1 2; do
      [ "${A[i]:-0}" -lt "${B[i]:-0}" ] && { echo -1; return; }
      [ "${A[i]:-0}" -gt "${B[i]:-0}" ] && { echo 1; return; }
    done
    if [ -z "${ap}" ] && [ -n "${bp}" ]; then echo 1; return; fi
    if [ -n "${ap}" ] && [ -z "${bp}" ]; then echo -1; return; fi
    echo 0
  }
  if [ "$(semver_cmp "${VERSION}" "${origin_version}")" -le 0 ]; then
    echo "::error::origin/${DEFAULT_BRANCH} already pins $(esc "${origin_pin_tag}") (>= ${VERSION}); refusing to rewind the pin"
    exit 1
  fi
elif [ -n "${origin_pin_sum_raw}" ] && [ -z "${origin_pin_sum}" ]; then
  # A checksum line exists but is not 64 lowercase hex: mangled pin.
  echo "::error::origin/${DEFAULT_BRANCH} carries a mangled checksum line: $(esc "${origin_pin_sum_raw}")"
  exit 1
elif [ -z "${origin_pin_sum_raw}" ] && [ -n "${origin_pin_tag}" ]; then
  # A tag line without any checksum line: mangled pin.
  echo "::error::origin/${DEFAULT_BRANCH} carries a releaseTag without a checksum line: $(esc "${origin_pin_tag}")"
  exit 1
fi
git commit -m "chore: pin the SPM checksum to ${TAG}"

# Only an OPEN pull request against the default branch from this
# repository's own branch counts. Fork PRs, same-owner cross-repo PRs,
# and PRs targeting a non-default base must not suppress the genuine bot
# PR. The owner name is validated before it is interpolated into the jq
# program; --paginate keeps the check independent of list windows.
existing_pr="$(gh pr list --head "${BRANCH}" --state open --paginate \
  --json number,headRepositoryOwner,isCrossRepository,baseRefName \
  --jq "[.[] | select(.headRepositoryOwner.login == \"${owner}\" and .isCrossRepository == false and .baseRefName == \"${DEFAULT_BRANCH}\")][0].number // empty")"

# The branch is disposable bot state; force-push keeps re-runs working
# when the remote branch already exists. It happens after the existence
# decision so a spoofed PR never references the bot commit.
git push --force origin "${BRANCH}"

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
