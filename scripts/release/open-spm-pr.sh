#!/usr/bin/env bash
# Open (or reuse) the SPM checksum-pin pull request for a release tag.
#
# Branch protection on the default branch rejects direct pushes (GH013),
# so the checksum pin lands via a pull request that a maintainer merges
# to activate SPM resolution for the release. Every step is idempotent so
# the workflow can be re-run after a transient failure.
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
  esc() { printf '%s' "$1" | sed -e 's/%/%25/g' -e 's/\r/%0D/g' -e 's/\n/%0A/g'; }
  echo "::error::tag $(esc "${TAG}") is not an ancestor of $(esc "${DEFAULT_BRANCH}"); cut the tag from the default branch tip"
  exit 1
fi

git checkout -B "${BRANCH}"
git add Package.swift
if git diff --cached --quiet; then
  # The tag is an ancestor of the default branch, so an empty diff means
  # the pin is already merged; publishing can proceed without a PR.
  echo "Package.swift already pinned to this checksum; nothing to do"
  exit 0
fi
git commit -m "chore: pin the SPM checksum to ${TAG}"

# Only an OPEN pull request from this repository's own branch counts.
# A fork PR with a colliding head-branch name must not suppress the
# genuine bot PR (checksum-spoofing guard).
owner="${GITHUB_REPOSITORY%%/*}"
existing_pr="$(gh pr list --head "${BRANCH}" --state open --json number,headRepositoryOwner \
  --jq "[.[] | select(.headRepositoryOwner.login == \"${owner}\")][0].number // empty")"

# The branch is disposable bot state; force-push keeps re-runs working
# when the remote branch already exists. It happens after the existence
# decision so a spoofed PR never references the bot commit.
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
