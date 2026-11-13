#!/usr/bin/env bash
# Open (or reuse) the SPM checksum-pin pull request for a release tag.
#
# Branch protection on the default branch rejects direct pushes (GH013),
# so the checksum pin lands via a pull request that a maintainer merges
# to activate SPM resolution for the release. Every step is idempotent so
# the workflow can be re-run after a transient failure.
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

git config user.name "mintlayer-release-bot"
git config user.email "hello@mintlayer.org"

# The tag must sit on the default branch's history; otherwise the pin PR
# would carry unrelated commits. Fail loudly before any push. Fetch with
# an explicit refspec so the remote-tracking ref exists regardless of the
# checkout action's fetch configuration.
git fetch origin "+refs/heads/${DEFAULT_BRANCH}:refs/remotes/origin/${DEFAULT_BRANCH}"
if ! git merge-base --is-ancestor "refs/tags/${TAG}" "origin/${DEFAULT_BRANCH}"; then
  # Workflow-command values need %-/CR-/LF-escaping to stay one annotation.
  esc() {
    local s="$1"
    s="${s//%/%25}"
    s="${s//$'\r'/%0D}"
    s="${s//$'\n'/%0A}"
    printf '%s' "$s"
  }
  echo "::error::tag $(esc "${TAG}") is not an ancestor of $(esc "${DEFAULT_BRANCH}"); cut the tag from the default branch tip"
  exit 1
fi

git checkout -B "${BRANCH}"
git add Package.swift
if git diff --cached --quiet; then
  # An empty diff is only success if the pin is genuinely present; an
  # unchanged file could equally mean the substitutions no-op'd on a
  # drifted Package.swift. Fail closed instead of skipping the pin.
  if grep -qF "${XCSUM}" Package.swift && grep -qF "${TAG}" Package.swift; then
    echo "Package.swift already pinned to this checksum; nothing to do"
    exit 0
  fi
  echo "::error::Package.swift did not change and does not carry the expected release tag or checksum"
  exit 1
fi
git commit -m "chore: pin the SPM checksum to ${TAG}"

# Only an OPEN pull request from this repository's own branch counts.
# Fork PRs with a colliding head-branch name (same owner via an org repo
# or otherwise) must not suppress the genuine bot PR. The owner name is
# validated before it is interpolated into the jq program; repo names
# cannot contain quotes or backslashes, and this fails closed otherwise.
owner="${GITHUB_REPOSITORY%%/*}"
if [[ ! "${owner}" =~ ^[A-Za-z0-9][A-Za-z0-9-]*$ ]]; then
  echo "::error::unexpected repository owner format: ${owner}"
  exit 1
fi
existing_pr="$(gh pr list --head "${BRANCH}" --state open --limit 200 \
  --json number,headRepositoryOwner,isCrossRepository \
  --jq "[.[] | select(.headRepositoryOwner.login == \"${owner}\" and .isCrossRepository == false)][0].number // empty")"

# The branch is disposable bot state; force-push keeps re-runs working
# when the remote branch already exists. It happens after the existence
# decision so a spoofed fork PR never references the bot commit.
git push --force origin "${BRANCH}"

if [ -n "${existing_pr}" ]; then
  echo "checksum-pin PR already exists (#${existing_pr}); updated its head branch"
else
  gh pr create \
    --head "${BRANCH}" \
    --title "chore: pin the SPM checksum to ${TAG}" \
    --body "Attaches the released XCFramework checksum to the root Package.swift so SPM resolves the ${TAG} artifact.

Checksum: \`${XCSUM}\`

Merge to activate."
fi
