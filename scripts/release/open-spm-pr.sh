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
owner="${GITHUB_REPOSITORY%%/*}"
if [[ ! "${owner}" =~ ^[A-Za-z0-9][A-Za-z0-9-]*$ ]]; then
  echo "::error::unexpected repository owner format: $(esc "${owner}")"
  exit 1
fi

git config user.name "mintlayer-release-bot"
git config user.email "hello@mintlayer.org"

# The tag must sit on the default branch's history; otherwise the pin PR
# would carry unrelated commits. Fail loudly before any push. Fetch with
# an explicit full-history refspec so the ancestry check sees the real
# graph (the release checkout is shallow by default).
git fetch --unshallow origin "+refs/heads/${DEFAULT_BRANCH}:refs/remotes/origin/${DEFAULT_BRANCH}" 2>/dev/null \
  || git fetch origin "+refs/heads/${DEFAULT_BRANCH}:refs/remotes/origin/${DEFAULT_BRANCH}"
if ! git merge-base --is-ancestor "refs/tags/${TAG}" "origin/${DEFAULT_BRANCH}"; then
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
# Empty staged diff = the tag commit itself carries the pin. The tag is
# an ancestor of the default branch, so that pin already shipped; any
# pin PR for it was merged previously. Nothing to open.
if git diff --cached --quiet; then
  echo "the tag already carries the pin; nothing to do"
  exit 0
fi
# An identical Package.swift on the default branch means the pin PR was
# already merged; short-circuit instead of attempting a create that
# GitHub would reject with "No commits between".
if git diff --quiet "origin/${DEFAULT_BRANCH}" -- Package.swift; then
  echo "checksum pin is already merged into ${DEFAULT_BRANCH}; nothing to do"
  exit 0
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

Merge to activate."
fi
