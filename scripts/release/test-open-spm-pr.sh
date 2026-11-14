#!/usr/bin/env bash
# Unit tests for scripts/release/open-spm-pr.sh,
# scripts/release/guard-release-refresh.sh, scripts/release/pin-order-gate.sh,
# and scripts/release/check-pin-order.sh, run entirely against stub
# git/gh binaries (no network, no repository mutation).
#
# Covered, among the rest of the suite: idempotent create/reuse of the
# checksum-pin PR with superseding of older pins and refusal on newer
# ones; rewind guards (semver precedence, build metadata excluded,
# deactivated-pin markers, mangled states); comment stripping
# (line/block, decoys, indent-tolerant); strict and ordering modes of
# the merge gate including the INCOMING_REF read and its failure paths;
# the gate script's diff-gated applicability, SHA256SUMS checksum
# binding, and strict-tag derivation; fatal origin refreshes
# (guard + gate); gh pr list failures failing closed with no push and
# no PR opened; semver.sh comparator units; fixture reset and
# no-mutation assertions.
# Runs without -e: failures are counted per case, not fatal.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UNDER_TEST="${SCRIPT_DIR}/open-spm-pr.sh"
TMP="$(mktemp -d)"
STUBS="${TMP}/stubs"
LOG="${TMP}/stub.log"
OUT="${TMP}/out.log"
trap 'rm -rf "${TMP}"' EXIT
mkdir -p "${STUBS}/repo/.git"
printf 'placeholder\n' > "${STUBS}/repo/Package.swift"
export LOG

command -v jq >/dev/null 2>&1 || { echo "jq is required by this test harness"; exit 1; }

pass=0
fail=0
ok() { pass=$((pass + 1)); echo "ok - $1"; }
# Sanitize log output: the script under test and stub args are
# PR-controlled on pull_request runs, so neutralize workflow-command
# forgery, control characters, DEL, and non-ASCII bytes before echoing.
sanitize() {
  # PR-controlled output: replace every '::' so no workflow command can
  # survive, per line. DEL and non-ASCII bytes are deleted outright
  # (esc() in pinlib.sh does the same); LF/CR are preserved — deleting
  # them could splice two benign fragments into a new command.
  printf '%s' "$1" | tr -d '\000-\010\011\013\014\016-\037\177\200-\377' | sed 's/::/[::]/g'
}
bad() { fail=$((fail + 1)); echo "NOT OK - $1: $(sanitize "$2")"; }

# --- stub git/gh -------------------------------------------------------
# Each stub logs its arguments and returns scripted behavior controlled
# by fixture files in $TMP:
#   ancestry=<0|1>       git merge-base --is-ancestor result
#   staged_diff=<0|1>    git diff --cached --quiet result (0 = empty)
#   package_swift=file   content the checkout writes to Package.swift
#   existing_pr=<json>   gh pr list output
cat > "${STUBS}/git" << 'EOF'
#!/usr/bin/env bash
echo "git $*" >> "${LOG}"
case "$1 $2" in
  "merge-base --is-ancestor") exit "$(cat "${CASE_DIR}/ancestry" 2>/dev/null || echo 1)" ;;
  "diff --cached") [ "$(cat "${CASE_DIR}/staged_diff" 2>/dev/null || echo 1)" = "0" ] && exit 0 || exit 1 ;;
  "diff --quiet") [ "$(cat "${CASE_DIR}/default_diff" 2>/dev/null || echo 1)" = "0" ] && exit 0 || exit 1 ;;
  "rev-parse --is-shallow-repository") cat "${CASE_DIR}/is_shallow" ;;
  "fetch origin" | "fetch --unshallow" | "fetch --no-tags")
    [ "$(cat "${CASE_DIR}/fetch_fails" 2>/dev/null || echo 0)" = "1" ] && exit 1
    exit 0 ;;
  "diff --name-only")
    [ "$(cat "${CASE_DIR}/head_touched" 2>/dev/null || echo 0)" = "1" ] && echo "Package.swift"
    exit 0 ;;
  "rev-parse HEAD") cat "${CASE_DIR}/head_commit" ;;
  "show origin/"*)
    if [ "$(cat "${CASE_DIR}/show_fails" 2>/dev/null || echo 0)" = "1" ]; then
      exit 1
    fi
    cat "${CASE_DIR}/origin_swift" ;;
  show\ *:*)
    if [ "$(cat "${CASE_DIR}/incoming_show_fails" 2>/dev/null || echo 0)" = "1" ]; then
      exit 1
    fi
    cat "${CASE_DIR}/incoming_swift" ;;
  rev-parse\ refs/tags/*) cat "${CASE_DIR}/tag_commit" ;;
  "checkout -B")
    cp "${CASE_DIR}/package_swift" "$(dirname "$0")/repo/Package.swift"
    exit 0 ;;
esac
exit 0
EOF
cat > "${STUBS}/gh" << 'EOF'
#!/usr/bin/env bash
echo "gh $*" >> "${LOG}"
if [ "$1" = "release" ] && [ "$2" = "download" ]; then
  [ "$(cat "${CASE_DIR}/release_download_fails" 2>/dev/null || echo 0)" = "1" ] && exit 1
  cat "${CASE_DIR}/release_sums"
  exit 0
fi
if [ "$1" = "pr" ] && [ "$2" = "list" ]; then
  mode="$(cat "${CASE_DIR}/pr_list_fails" 2>/dev/null || echo "")"
  case "${mode}" in
    any) exit 1 ;;
    existing)
      for a in "$@"; do [ "${a}" = "--head" ] && exit 1; done ;;
    supersede)
      for a in "$@"; do [ "${a}" = "--head" ] && return_failed=1; done
      [ "${return_failed:-}" = "1" ] || exit 1 ;;
  esac
  payload="$(cat "${CASE_DIR}/existing_pr" 2>/dev/null || echo "[]")"
  prog=""
  prev=""
  for a in "$@"; do
    [ "${prev}" = "--jq" ] && prog="$a"
    prev="$a"
  done
  printf '%s' "${payload}" | jq -r "${prog}"
  exit $?
fi
# pr close/create must reference a numeric PR (real gh rejects garbage).
if [ "$1" = "pr" ] && { [ "$2" = "close" ] || [ "$2" = "create" ]; }; then
  for a in "$@"; do
    if [[ "${a}" =~ ^[0-9]+$ ]]; then
      exit 0
    fi
    if [[ "${a}" == --head* || "${a}" == --base* || "${a}" == --title* || "${a}" == --body* ]]; then
      exit 0
    fi
  done
  echo "gh: refusing non-numeric PR argument" >&2
  exit 1
fi
exit 0
EOF
chmod +x "${STUBS}/git" "${STUBS}/gh"

XCSUM="aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"
FULLY_PINNED='let releaseTag = "wallet-bindings-v1.2.3"
            checksum: "'"${XCSUM}"'"'
PARTIAL_DRIFT='let releaseTag = "wallet-bindings-v1.2.2"
            checksum: "'"${XCSUM}"'"'
NOT_PINNED='let releaseTag = "wallet-bindings-v9.9.9"
            checksum: "deadbeef"'

# PR fixtures; baseRefName is main unless stated.
echo '[]' > "${TMP}/pr.none"
echo '[{"number": 7, "headRefName": "pin-spm-checksum-1.2.3", "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": false, "baseRefName": "main"}]' > "${TMP}/pr.own"
echo '[{"number": 9, "headRefName": "pin-spm-checksum-1.2.3", "headRepositoryOwner": {"login": "evil"}, "isCrossRepository": false, "baseRefName": "main"}]' > "${TMP}/pr.evillow"
echo '[{"number": 9, "headRefName": "pin-spm-checksum-1.2.3", "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": true, "baseRefName": "main"}]' > "${TMP}/pr.fork"
echo '[{"number": 9, "headRefName": "pin-spm-checksum-1.2.3", "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": false, "baseRefName": "develop"}]' > "${TMP}/pr.wrongbase"
echo '[{"number": 5, "headRefName": "pin-spm-checksum-0.0.9", "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": false, "baseRefName": "main"}]' > "${TMP}/pr.stale_older"
echo '[{"number": 6, "headRefName": "pin-spm-checksum-9.9.9", "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": false, "baseRefName": "main"}]' > "${TMP}/pr.stale_newer"
echo '[{"number": 12, "headRefName": "pin-spm-checksum-1.2.3+meta", "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": false, "baseRefName": "main"}]' > "${TMP}/pr.stale_meta"
echo '[{"number": 13, "headRefName": "pin-spm-checksum-v0.1.1", "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": false, "baseRefName": "main"}]' > "${TMP}/pr.stale_legacy"

# set_fixtures <ancestry> <staged_diff> <package_swift> <existing_pr> [default_diff]
# default_diff: 0 = Package.swift identical to the default branch (pin merged).
set_fixtures() {
  printf '%s' "$1" > "${TMP}/ancestry"
  printf '%s' "$2" > "${TMP}/staged_diff"
  printf '%s' "$3" > "${TMP}/package_swift"
  cp "${TMP}/$4" "${TMP}/existing_pr"
  printf '%s' "${5:-1}" > "${TMP}/default_diff"
  printf 'tagcommitsha' > "${TMP}/head_commit"
  printf 'tagcommitsha' > "${TMP}/tag_commit"
  printf 'false' > "${TMP}/is_shallow"
  # Realistic shape: the file header comment also carries a
  # wallet-bindings-vX.Y.Z literal ahead of the pin line.
  # shellcheck disable=SC2016  # \n are printf escapes, no expansion
  printf '// The binary target is attached to the `wallet-bindings-vX.Y.Z`\n// GitHub release; its checksum is updated by the release workflow.\n' > "${TMP}/origin_swift"
  printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' >> "${TMP}/origin_swift"
}

# assert_no_mutations: fail-closed cases must not push or open PRs.
reset_origin() {
  printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"
}

assert_no_mutations() {
  if grep -q "git push" <<< "${LOG_CONTENTS}" || grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
    bad "no mutations on failure" "${LOG_CONTENTS}"
  else
    ok "no mutations on failure"
  fi
}

# run_case <name> <expected_exit> [repository] [default_branch] [xcsum]
#   [version] [tag]
run_case() {
  local name="$1" expected="$2" repo="${3:-mintlayer/rust-sdk}"
  local branch="${4:-main}" xcsum="${5:-${XCSUM}}" actual=0
  local version="${6:-1.2.3}" tag="${7:-wallet-bindings-v1.2.3}"
  : > "${LOG}"
  ( cd "${STUBS}/repo" && \
    PATH="${STUBS}:${PATH}" \
    GITHUB_REPOSITORY="${repo}" \
    CASE_DIR="${TMP}" \
    bash "${UNDER_TEST}" "${tag}" "${version}" "${xcsum}" "${branch}" \
      > "${OUT}" 2>&1 ) || actual=$?
  LOG_CONTENTS="$(cat "${LOG}")"
  if [ "${actual}" = "${expected}" ]; then
    ok "${name}"
  else
    bad "${name}" "exit ${actual}, want ${expected}; output: $(cat "${OUT}")"
  fi
}

# 1. Tag not an ancestor of the default branch -> fail loudly.
set_fixtures 1 1 "${NOT_PINNED}" pr.none
run_case "non-ancestor tag fails" 1
assert_no_mutations

# 2. Ancestor + real diff -> success; push issued before create.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "0" ]; then ok "case2 exit 0"; else bad "case2 exit" "rc=${rc}"; fi
if grep -q -- "push --force origin pin-spm-checksum-1.2.3" <<< "${LOG_CONTENTS}"; then
  ok "force-push issued"
else
  bad "force-push issued" "${LOG_CONTENTS}"
fi
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "pr create called"
else
  bad "pr create called" "${LOG_CONTENTS}"
fi
push_line="$(grep -n -- '--force' <<< "${LOG_CONTENTS}" | head -1 | cut -d: -f1)"
create_line="$(grep -n 'gh pr create' <<< "${LOG_CONTENTS}" | head -1 | cut -d: -f1)"
if [ -n "${push_line}" ] && [ -n "${create_line}" ] && [ "${push_line}" -lt "${create_line}" ]; then
  ok "push precedes create"
else
  bad "push precedes create" "${LOG_CONTENTS}"
fi
if grep -q -- "--base main" <<< "${LOG_CONTENTS}" \
  && grep -q -- "--head pin-spm-checksum-1.2.3" <<< "${LOG_CONTENTS}"; then
  ok "create pins base and head explicitly"
else
  bad "create pins base and head explicitly" "${LOG_CONTENTS}"
fi

# 3. Empty diff + pin present but main DIFFERS -> the pin on main was
# reverted; fail closed instead of claiming success.
set_fixtures 0 0 "${FULLY_PINNED}" pr.none 1
run_case "empty diff with reverted main fails" 1
assert_no_mutations

# 4. Empty diff + pin absent -> fail closed.
set_fixtures 0 0 "${NOT_PINNED}" pr.none
run_case "empty diff without pin fails" 1
assert_no_mutations

# 5. Partial drift: new checksum but OLD tag -> fail closed (the sed for
# the tag no-op'd; publishing would break the binaryTarget URL).
set_fixtures 0 1 "${PARTIAL_DRIFT}" pr.none
run_case "partial drift (checksum new, tag old) fails" 1
assert_no_mutations

# 6. Mirror drift: new tag but STALE checksum -> fail closed (the sed
# for the checksum no-op'd; SPM consumers would resolve a stale artifact).
OLD_XCSUM="1111111111111111111111111111111111111111111111111111111111111111"
STALE_CKSUM='let releaseTag = "wallet-bindings-v1.2.3"
            checksum: "'"${OLD_XCSUM}"'"'
set_fixtures 0 1 "${STALE_CKSUM}" pr.none
run_case "mirror drift (tag new, checksum stale) fails" 1
assert_no_mutations

# 7. Pin already merged: Package.swift identical to the default branch
# -> short-circuit success even though the checkout diff is non-empty.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none 0
run_case "already-merged pin exits 0" 0
assert_no_mutations

# 8. Open same-repo PR against main -> detected, no duplicate create.
set_fixtures 0 1 "${FULLY_PINNED}" pr.own
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "0" ]; then ok "own open PR detected"; else bad "own open PR detected" "exit ${rc}"; fi
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  bad "no duplicate create" "${LOG_CONTENTS}"
else
  ok "no duplicate create"
fi

# 9. Open different-owner PR with colliding head branch -> create proceeds.
set_fixtures 0 1 "${FULLY_PINNED}" pr.evillow
run_case "different-owner PR ignored, create proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create despite different-owner PR"
else
  bad "create despite different-owner PR" "${LOG_CONTENTS}"
fi

# 10. Open fork PR with colliding head branch -> create proceeds.
set_fixtures 0 1 "${FULLY_PINNED}" pr.fork
run_case "fork PR ignored, create proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create despite fork PR"
else
  bad "create despite fork PR" "${LOG_CONTENTS}"
fi

# 11. Open same-owner PR against a non-default base -> create proceeds.
set_fixtures 0 1 "${FULLY_PINNED}" pr.wrongbase
run_case "wrong-base PR ignored, create proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create despite wrong-base PR"
else
  bad "create despite wrong-base PR" "${LOG_CONTENTS}"
fi

# 12. Malformed repository owner -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
run_case "invalid owner fails" 1 "bad..owner/rust-sdk"
assert_no_mutations

# 13. Malformed DEFAULT_BRANCH -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid default branch fails" 1 "mintlayer/rust-sdk" $'ma\tin'
assert_no_mutations

# 14. Malformed XCSUM -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid checksum fails" 1 "mintlayer/rust-sdk" main "xyz"
assert_no_mutations

# 15. HEAD is not the tagged commit -> fail closed, no push.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'othercommitsha' > "${TMP}/head_commit"
run_case "HEAD differs from tag fails" 1
assert_no_mutations
printf 'tagcommitsha' > "${TMP}/head_commit"

# 16. All-zero checksum -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "all-zero checksum fails" 1 "mintlayer/rust-sdk" main \
  "0000000000000000000000000000000000000000000000000000000000000000"
assert_no_mutations

# 17. VERSION that is not semver -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid version fails" 1 "mintlayer/rust-sdk" main "${XCSUM}" 1.2
assert_no_mutations

# 18. TAG not matching the release pattern -> fail closed before fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid tag (no patch) fails" 1 "mintlayer/rust-sdk" main "${XCSUM}" 1.2 "wallet-bindings-v1.2"
assert_no_mutations
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid tag (wrong prefix) fails" 1 "mintlayer/rust-sdk" main "${XCSUM}" 1.2.3 "evil-wallet-bindings-v1.2.3"
assert_no_mutations

# 19. Default branch inside the reserved namespace -> fail closed.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "reserved-namespace default branch fails" 1 "mintlayer/rust-sdk" "pin-spm-checksum-1.2.3"
assert_no_mutations

# 20. Shallow clone -> the unshallow fetch is used.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'true' > "${TMP}/is_shallow"
run_case "shallow clone unshallows" 0
if grep -q -- "fetch --unshallow origin" <<< "${LOG_CONTENTS}"; then
  ok "unshallow fetch issued"
else
  bad "unshallow fetch issued" "${LOG_CONTENTS}"
fi
printf 'false' > "${TMP}/is_shallow"

# 21. Complete clone -> plain fetch with the explicit full refspec.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
run_case "complete clone plain fetch" 0
if grep -q -- "fetch origin +refs/heads/main:refs/remotes/origin/main" <<< "${LOG_CONTENTS}"; then
  ok "plain fetch carries the full refspec"
else
  bad "plain fetch carries the full refspec" "${LOG_CONTENTS}"
fi

# 22. Default branch already pins a DIFFERENT release -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v9.9.9"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
run_case "stale tag against a newer pin fails" 1
assert_no_mutations

# 23. Forward release over an OLDER active pin -> allowed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v0.1.0"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
run_case "forward release over older pin proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create proceeds over older pin"
else
  bad "create proceeds over older pin" "${LOG_CONTENTS}"
fi

# 23b. gh pr list failure on the existing-pin lookup -> fail closed
# with no push and no PR opened.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'existing' > "${TMP}/pr_list_fails"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
printf '' > "${TMP}/pr_list_fails"
LOG_CONTENTS="$(cat "${LOG}")"
PUSHED="$(grep -c "git push" <<< "${LOG_CONTENTS}" || true)"
CREATED="$(grep -c "pr create" <<< "${LOG_CONTENTS}" || true)"
if [ "${rc}" = "1" ] && [ "${PUSHED}" = "0" ] && [ "${CREATED}" = "0" ]; then
  ok "existing-pin lookup failure fails closed"
else
  bad "existing-pin lookup failure fails closed" "exit ${rc}, pushes=${PUSHED}, creates=${CREATED}"
fi
assert_no_mutations

# 23c. gh pr list failure on the supersede lookup -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'supersede' > "${TMP}/pr_list_fails"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
printf '' > "${TMP}/pr_list_fails"
LOG_CONTENTS="$(cat "${LOG}")"
PUSHED="$(grep -c "git push" <<< "${LOG_CONTENTS}" || true)"
CREATED="$(grep -c "pr create" <<< "${LOG_CONTENTS}" || true)"
if [ "${rc}" = "1" ] && [ "${PUSHED}" = "0" ] && [ "${CREATED}" = "0" ]; then
  ok "supersede lookup failure fails closed"
else
  bad "supersede lookup failure fails closed" "exit ${rc}, pushes=${PUSHED}, creates=${CREATED}"
fi
assert_no_mutations
reset_origin

# 24. Same-version active pin with differing content -> rewind guard.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
run_case "same-version conflicting pin fails" 1
assert_no_mutations
reset_origin

# 25. merge-base git error (rc > 1) -> fail closed, distinct from rc 1.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf '128' > "${TMP}/ancestry"
run_case "merge-base git error fails" 1
assert_no_mutations
printf '0' > "${TMP}/ancestry"

# 26. Unparseable origin pin tag -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-vx"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
run_case "unparseable origin pin tag fails" 1
assert_no_mutations
reset_origin

# 27. Active checksum with missing tag line -> fail closed (one-sided).
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'checksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
run_case "one-sided active checksum fails" 1
assert_no_mutations
reset_origin

# 28. TAG/VERSION mismatch -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "mismatched tag and version fails" 1 "mintlayer/rust-sdk" main "${XCSUM}" 9.9.9
assert_no_mutations

# 29. Mangled checksum line (valid tag, non-hex checksum) -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "deadbeef"\n' > "${TMP}/origin_swift"
run_case "mangled origin checksum fails" 1
assert_no_mutations
reset_origin

# 30. Prerelease over final (1.2.3-rc.1 after 1.2.3) -> rewind, refused.
set_fixtures 0 1 'let releaseTag = "wallet-bindings-v1.2.3-rc.1" // ...
checksum: "'"${XCSUM}"'"' pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3-rc.1 1.2.3-rc.1 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ]; then ok "prerelease after final refused"; else bad "prerelease after final refused" "exit ${rc}: $(cat "${OUT}")"; fi
assert_no_mutations
reset_origin

# 31. Final over prerelease (1.2.3 after 1.2.3-rc1) -> allowed forward.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3-rc1"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
if [ "${rc}" = "0" ] && grep -q "gh pr create" "${LOG}"; then
  ok "final over prerelease proceeds"
else
  bad "final over prerelease proceeds" "exit ${rc}: $(cat "${OUT}")"
fi
reset_origin

# 32. Origin pin 1.2.3+meta.b vs incoming 1.2.3+meta.a compares EQUAL -> refuse.
set_fixtures 0 1 'let releaseTag = "wallet-bindings-v1.2.3+meta.a" // ...
checksum: "'"${XCSUM}"'"' pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3+meta.b"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3+meta.a 1.2.3+meta.a "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ]; then ok "build-metadata equal precedence refused"; else bad "build-metadata equal precedence refused" "exit ${rc}: $(cat "${OUT}")"; fi
assert_no_mutations

# 33. Origin pin without metadata vs incoming with metadata: equal
# precedence -> refuse.
set_fixtures 0 1 'let releaseTag = "wallet-bindings-v1.2.3+meta" // ...
checksum: "'"${XCSUM}"'"' pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3+meta 1.2.3+meta "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ]; then ok "metadata-suffix equal precedence refused"; else bad "metadata-suffix equal precedence refused" "exit ${rc}: $(cat "${OUT}")"; fi
assert_no_mutations

# 34. Guard: mangled checksum (65+ hex) on origin -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b9700"\n' > "${TMP}/origin_swift"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ]; then ok "guard refuses 65-hex checksum"; else bad "guard refuses 65-hex checksum" "exit ${rc}"; fi
assert_no_mutations

# 35. Huge numeric prerelease identifiers compare by (digit length,
# bytes) — no arithmetic — so an origin pin of rc<20 digits> still
# outranks incoming rc.2 -> refused, no arithmetic wrap.
set_fixtures 0 1 'let releaseTag = "wallet-bindings-v1.2.3-2" // ...
checksum: "'"${XCSUM}"'"' pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3-18446744073709551617"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3-2 1.2.3-2 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ]; then ok "huge numeric prerelease refused without wrap"; else bad "huge numeric prerelease refused without wrap" "exit ${rc}: $(cat "${OUT}")"; fi
assert_no_mutations
reset_origin

# 36. Guard script: same tag + differing checksum -> refuse.
GUARD="${SCRIPT_DIR}/guard-release-refresh.sh"
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${GUARD}" wallet-bindings-v1.2.3 "1111111111111111111111111111111111111111111111111111111111111111" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard refuses mismatched rebuild"; else bad "guard refuses mismatched rebuild" "exit ${rc}"; fi

# 37. Guard script: same tag + identical checksum (with a header
# comment whose version literal differs) -> allow.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
# shellcheck disable=SC2016  # \n are printf escapes, no expansion
printf '// The binary target is attached to the `wallet-bindings-v9.9.9`\n// GitHub release; its checksum is updated by the release workflow.\n' > "${TMP}/origin_swift"
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "%s"\n' "${XCSUM}" >> "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${GUARD}" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "guard allows identical rebuild"; else bad "guard allows identical rebuild" "exit ${rc}"; fi

# 38. Guard script: older pin on main (forward release) -> allow.
printf 'let releaseTag = "wallet-bindings-v0.1.0"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${GUARD}" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "guard allows forward release"; else bad "guard allows forward release" "exit ${rc}"; fi
reset_origin

# 39. git show fails -> open-spm-pr.sh fails closed (no silent skip).
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf '1' > "${TMP}/show_fails"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ]; then ok "git show failure fails closed"; else bad "git show failure fails closed" "exit ${rc}: $(cat "${OUT}")"; fi
assert_no_mutations
printf '0' > "${TMP}/show_fails"
reset_origin

# 40. Comment decoys inside an unterminated block comment; the real
# code pin (v1.0.0) follows the block. Correct stripping sees 1.2.3 >
# 1.0.0 and allows; a stripping regression leaks the decoys (mangled
# multi-pin state, or >= the incoming version) and refuses.
printf '/*\nchecksum: "1111111111111111111111111111111111111111111111111111111111111111"\nlet releaseTag = "wallet-bindings-v1.2.3"\nlet releaseTag = "wallet-bindings-v9.9.9"\n*/\nlet releaseTag = "wallet-bindings-v1.0.0"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "script ignores comment decoys"; else bad "script ignores comment decoys" "exit ${rc}"; fi
reset_origin

# 41. Same decoy shape for the guard: real v1.0.0 pin follows the
# block, so the refresh (1.2.3 > 1.0.0) must be allowed.
printf '/*\nchecksum: "1111111111111111111111111111111111111111111111111111111111111111"\nlet releaseTag = "wallet-bindings-v1.2.3"\nlet releaseTag = "wallet-bindings-v9.9.9"\n*/\nlet releaseTag = "wallet-bindings-v1.0.0"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "guard ignores comment decoys"; else bad "guard ignores comment decoys" "exit ${rc}"; fi

# 42. Guard: a failed origin refresh must fail closed (no clobber
# decision can be made against a stale pin view).
printf '1' > "${TMP}/fetch_fails"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
printf '0' > "${TMP}/fetch_fails"
if [ "${rc}" = "1" ]; then ok "guard origin refresh failure fails closed"; else bad "guard origin refresh failure fails closed" "exit ${rc}"; fi
reset_origin

# 43. Guard: zero releaseTag assignments -> refuse.
printf 'checksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard refuses zero assignments"; else bad "guard refuses zero assignments" "exit ${rc}"; fi

# 44. Guard: two releaseTag assignments -> refuse.
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nlet releaseTag = "wallet-bindings-v9.9.9"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard refuses duplicate assignments"; else bad "guard refuses duplicate assignments" "exit ${rc}"; fi

# 45. Guard: invalid checksum -> refuse.
reset_origin
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "nothex" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard rejects invalid checksum"; else bad "guard rejects invalid checksum" "exit ${rc}"; fi

# 46. Guard: git show failure -> fail closed.
printf '1' > "${TMP}/show_fails"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard fails closed on git show failure"; else bad "guard fails closed on git show failure" "exit ${rc}"; fi
printf '0' > "${TMP}/show_fails"
reset_origin

# 47. Pin-order script: upgrade allows, downgrade refuses, same refuses.
ORDER="${SCRIPT_DIR}/check-pin-order.sh"
printf 'let releaseTag = "wallet-bindings-v1.2.3+meta"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "pin order allows upgrade"; else bad "pin order allows upgrade" "exit ${rc}"; fi

printf 'let releaseTag = "wallet-bindings-v0.9.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v0.9.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order refuses downgrade"; else bad "pin order refuses downgrade" "exit ${rc}"; fi

printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.2.3" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order refuses same version"; else bad "pin order refuses same version" "exit ${rc}"; fi

# 48. Production form: the incoming pin read from an INCOMING_REF
# (the PR head SHA) rather than the working tree.
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/incoming_swift"
printf 'placeholder\n' > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" 0123456789abcdef0123456789abcdef01234567 >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "pin order reads the incoming pin from a ref"; else bad "pin order reads the incoming pin from a ref" "exit ${rc}"; fi

# 49. Same, but the ref read fails -> fail closed (esc'd annotation).
printf '1' > "${TMP}/incoming_show_fails"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" 0123456789abcdef0123456789abcdef01234567 >/dev/null 2>&1 )
rc=$?
printf '0' > "${TMP}/incoming_show_fails"
if [ "${rc}" = "1" ]; then ok "pin order fails closed on an unreadable incoming ref"; else bad "pin order fails closed on an unreadable incoming ref" "exit ${rc}"; fi

# Ordering-only mode (empty EXPECTED_TAG): non-namespace Package.swift
# edits are order-checked without a tag-equality requirement. The
# caller (ci.yml) binds the checksum to the published release.

# 45f-2. Base branch with no pin at all -> early-accept.
printf 'checksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "pin order accepts any pin over an unpinned base"; else bad "pin order accepts any pin over an unpinned base" "exit ${rc}"; fi

# 45f-3. A prefix-matching but malformed base pin line (stray suffix) ->
# refuse: the base is not "unpinned".
printf 'let releaseTag = "wallet-bindings-v1.0.0"x\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "" > "${OUT}" 2>&1 )
rc=$?
if [ "${rc}" = "1" ] && grep -q "mangled pin line" <<< "$(cat "${OUT}")"; then ok "pin order refuses a prefix-mangled base pin"; else bad "pin order refuses a prefix-mangled base pin" "exit ${rc}: $(cat "${OUT}")"; fi

# Mixed state: one canonical assignment plus an extra pin-bearing line
# that is not anchored (releaseTagX) -> refuse (extras beyond the
# canonical assignment are mangled state, even with a well-formed pin
# present).
printf 'let releaseTag = "wallet-bindings-v1.0.0"\nlet releaseTagX = "wallet-bindings-v0.1.0"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "" > "${OUT}" 2>&1 )
rc=$?
if [ "${rc}" = "1" ] && grep -q "pin-bearing occurrence" <<< "$(cat "${OUT}")"; then ok "pin order refuses extra pin-bearing lines beside the canonical one"; else bad "pin order refuses extra pin-bearing lines beside the canonical one" "exit ${rc}: $(cat "${OUT}")"; fi

# Comment-only base -> refuse (mirrors the guard; an unpinned verdict
# here would make any pin mergeable over a corrupted base).
printf '// nothing but comments\n' > "${TMP}/origin_swift"
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "" > "${OUT}" 2>&1 )
rc=$?
if [ "${rc}" = "1" ] && grep -q "carries no code lines" <<< "$(cat "${OUT}")"; then ok "pin order refuses a comment-only base"; else bad "pin order refuses a comment-only base" "exit ${rc}: $(cat "${OUT}")"; fi

# Broken-prefix variant: a double space before '=' breaks the anchored
# prefix itself — detection must not be prefix-anchored.
printf 'let releaseTag  = "wallet-bindings-v1.0.0"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "" > "${OUT}" 2>&1 )
rc=$?
if [ "${rc}" = "1" ] && grep -q "mangled pin line" <<< "$(cat "${OUT}")"; then ok "pin order refuses a broken-prefix base pin"; else bad "pin order refuses a broken-prefix base pin" "exit ${rc}: $(cat "${OUT}")"; fi
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "ordering mode allows a forward pin"; else bad "ordering mode allows a forward pin" "exit ${rc}"; fi

printf 'let releaseTag = "wallet-bindings-v0.9.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "ordering mode refuses a rewind"; else bad "ordering mode refuses a rewind" "exit ${rc}"; fi

# The gate's origin refresh is fatal: a failed fetch must not decide
# against a stale pin view.
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
printf '1' > "${TMP}/fetch_fails"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
printf '0' > "${TMP}/fetch_fails"
if [ "${rc}" = "1" ]; then ok "pin order origin refresh failure fails closed"; else bad "pin order origin refresh failure fails closed" "exit ${rc}"; fi

# A base branch carrying two pin assignments is a mangled state.
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\nlet releaseTag = "wallet-bindings-v1.0.0"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order refuses an ambiguous base pin"; else bad "pin order refuses an ambiguous base pin" "exit ${rc}"; fi
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"

# GATE. The ci.yml gate script: diff-gated applicability, checksum
# binding against the release's SHA256SUMS, then ordering.
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
GATE="${SCRIPT_DIR}/pin-order-gate.sh"
printf 'SHA256SUMS for the release\n%s  MintlayerWalletBindings.xcframework.zip\n' "${XCSUM}" > "${TMP}/release_sums"
printf '0' > "${TMP}/release_download_fails"

# Untouched Package.swift -> skip without reading anything.
printf '0' > "${TMP}/head_touched"
run_gate() {
  ( cd "${STUBS}/repo" && \
    PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
    BASE_REF="main" HEAD_SHA="0123456789abcdef0123456789abcdef01234567" \
    HEAD_REF="$1" EVENT_NAME="pull_request" PR_NUMBER="7" \
    bash "${GATE}" > "${OUT}" 2>&1 )
  rc=$?
}
# Push-event variant: no PR-head fetch branch; the pushed SHA is
# already local.
run_gate_push() {
  ( cd "${STUBS}/repo" && \
    PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
    BASE_REF="main" HEAD_SHA="0123456789abcdef0123456789abcdef01234567" \
    HEAD_REF="$1" EVENT_NAME="push" PR_NUMBER="" \
    bash "${GATE}" > "${OUT}" 2>&1 )
  rc=$?
}
run_gate "feature"
if [ "${rc}" = "0" ] && grep -q "untouched" "${OUT}"; then
  ok "gate skips a PR that does not touch Package.swift"
else
  bad "gate skips a PR that does not touch Package.swift" "exit ${rc}: $(cat "${OUT}")"
fi

# Touched + forward pin bound to the published sums -> allow.
printf '1' > "${TMP}/head_touched"
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/incoming_swift"
run_gate "feature"
if [ "${rc}" = "0" ]; then ok "gate allows a bound forward pin"; else bad "gate allows a bound forward pin" "exit ${rc}: $(cat "${OUT}")"; fi

# Checksum not matching the published SHA256SUMS -> refuse.
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/incoming_swift"
run_gate "feature"
if [ "${rc}" = "1" ] && grep -q "does not match the published" "${OUT}"; then
  ok "gate refuses an unbound checksum"
else
  bad "gate refuses an unbound checksum" "exit ${rc}: $(cat "${OUT}")"
fi

# Unreadable release manifest -> fail closed.
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/incoming_swift"
printf '1' > "${TMP}/release_download_fails"
run_gate "feature"
printf '0' > "${TMP}/release_download_fails"
if [ "${rc}" = "1" ] && grep -q "unverifiable pin" "${OUT}"; then
  ok "gate fails closed on a missing release manifest"
else
  bad "gate fails closed on a missing release manifest" "exit ${rc}: $(cat "${OUT}")"
fi

# Mangled head (two releaseTag lines) -> refuse.
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\nlet releaseTag = "wallet-bindings-v1.2.3"\n' "${XCSUM}" > "${TMP}/incoming_swift"
run_gate "feature"
if [ "${rc}" = "1" ] && grep -q "exactly one releaseTag" "${OUT}"; then
  ok "gate refuses a mangled head"
else
  bad "gate refuses a mangled head" "exit ${rc}: $(cat "${OUT}")"
fi

# Push event: same binding and ordering without the PR-head fetch.
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/incoming_swift"
printf '1' > "${TMP}/head_touched"
run_gate_push "feature"
if [ "${rc}" = "0" ]; then ok "gate push event allows a bound forward pin"; else bad "gate push event allows a bound forward pin" "exit ${rc}: $(cat "${OUT}")"; fi

printf '0' > "${TMP}/head_touched"
run_gate_push "feature"
if [ "${rc}" = "0" ] && grep -q "untouched" "${OUT}"; then
  ok "gate push event skips an untouched Package.swift"
else
  bad "gate push event skips an untouched Package.swift" "exit ${rc}: $(cat "${OUT}")"
fi
printf '1' > "${TMP}/head_touched"

# Reserved namespace: strict tag equality against the branch name.
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/incoming_swift"
run_gate "pin-spm-checksum-1.3.0"
if [ "${rc}" = "0" ]; then ok "gate strict mode accepts the declared release"; else bad "gate strict mode accepts the declared release" "exit ${rc}: $(cat "${OUT}")"; fi

printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/incoming_swift"
run_gate "pin-spm-checksum-1.3.0"
if [ "${rc}" = "1" ] && grep -q "does not carry exactly the expected pin assignment" "${OUT}"; then
  ok "gate strict mode refuses a tag mismatch"
else
  bad "gate strict mode refuses a tag mismatch" "exit ${rc}: $(cat "${OUT}")"
fi

# Legacy v-suffix branch names (pre-automation) derive the correct
# expected tag: pin-spm-checksum-v1.3.0 -> wallet-bindings-v1.3.0.
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/incoming_swift"
run_gate "pin-spm-checksum-v1.3.0"
if [ "${rc}" = "0" ]; then ok "gate tolerates a legacy v-suffix branch name"; else bad "gate tolerates a legacy v-suffix branch name" "exit ${rc}: $(cat "${OUT}")"; fi

# A reserved-namespace branch whose suffix is not a semver -> refuse
# at the derivation check.
run_gate "pin-spm-checksum-oops"
if [ "${rc}" = "1" ] && grep -q "does not match wallet-bindings-v<semver>" "${OUT}"; then
  ok "gate refuses an unparseable namespace branch name"
else
  bad "gate refuses an unparseable namespace branch name" "exit ${rc}: $(cat "${OUT}")"
fi

# An unreadable head commit -> fail closed at the gate itself.
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/incoming_swift"
printf '1' > "${TMP}/incoming_show_fails"
run_gate "feature"
printf '0' > "${TMP}/incoming_show_fails"
if [ "${rc}" = "1" ] && grep -q "cannot read Package.swift from the PR head" "${OUT}"; then
  ok "gate fails closed on an unreadable head"
else
  bad "gate fails closed on an unreadable head" "exit ${rc}: $(cat "${OUT}")"
fi
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 50. Reserved-namespace base branch -> fail closed.
# shellcheck disable=SC2016  # fixture line content is not expanded
printf 'let releaseTag = "wallet-bindings-v0.9.0"\nchecksum: "${XCSUM}"\n' > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" "pin-spm-checksum-1.2.3" "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order rejects reserved namespace"; else bad "pin order rejects reserved namespace" "exit ${rc}"; fi

# 51. Ambiguous PR head (a second releaseTag line) -> strict mode
# refuses: the normalized head must carry exactly the expected
# assignment and no other releaseTag- or checksum-bearing line.
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "%s"\nlet releaseTag = "wallet-bindings-v1.2.3"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order refuses ambiguous PR head"; else bad "pin order refuses ambiguous PR head" "exit ${rc}"; fi

# 52. Single-defect counterpart: exactly the expected head but a
# non-hex checksum -> strict mode refuses.
# shellcheck disable=SC2016  # fixture line content is not expanded
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "${XCSUM}"\n' > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order refuses a non-hex checksum"; else bad "pin order refuses a non-hex checksum" "exit ${rc}"; fi

# 53. git show failure on the base -> fail closed.
printf '1' > "${TMP}/show_fails"
# shellcheck disable=SC2016  # fixture line content is not expanded
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "${XCSUM}"\n' > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order fails closed on git show error"; else bad "pin order fails closed on git show error" "exit ${rc}"; fi
printf '0' > "${TMP}/show_fails"
printf 'placeholder\n' > "${STUBS}/repo/Package.swift"

# 54. Strict mode PASS path over a representative root Package.swift shape:
# header comment, the download URL interpolating \(releaseTag) (which a
# substring 'releaseTag' count would double-count), releaseTag at
# column 0, checksum indented 12 spaces.
# shellcheck disable=SC2016  # \(releaseTag) is Swift interpolation, literal here
printf '// SPM entry point for iOS consumers. The binary target is attached to the\n// `wallet-bindings-vX.Y.Z` GitHub release; its checksum is updated by the\n// release workflow.\n\nimport PackageDescription\n\nlet releaseTag = "wallet-bindings-v1.3.0"\nlet releaseBaseUrl = "https://github.com/mintlayer/rust-sdk/releases/download/\(releaseTag)"\nlet package = Package(\n    binaryTarget(\n            checksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${STUBS}/repo/Package.swift"
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "strict mode passes the real Package.swift shape"; else bad "strict mode passes the real Package.swift shape" "exit ${rc}"; fi

# 55. Supersede: an older stale pin PR is closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.stale_older
reset_origin
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
if [ "${rc}" = "0" ] && grep -q -- "pr close 5" "${LOG}"; then
  ok "supersede closes the older stale pin PR"
else
  bad "supersede closes the older stale pin PR" "exit ${rc}: $(cat "${OUT}")"
fi

# 56. Supersede: a newer open pin PR refuses the run.
cp "${TMP}/pr.stale_newer" "${TMP}/existing_pr"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
if [ "${rc}" = "1" ] && grep -q "9.9.9" <<< "$(cat "${OUT}")"; then
  ok "supersede refuses a newer open pin PR"
else
  bad "supersede refuses a newer open pin PR" "exit ${rc}: $(cat "${OUT}")"
fi

# 57. Guard over a deactivated newer-or-equal marker: refresh allowed (rewind refusal enforced upstream by open-spm-pr.sh).
set_fixtures 0 1 'let releaseTag = "wallet-bindings-v9.9.9"
            checksum: "0000000000000000000000000000000000000000000000000000000000000000"' pr.none
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "0" ]; then ok "guard allows refresh over deactivated newer marker (enforced upstream)"; else bad "guard allows refresh over deactivated newer marker" "exit ${rc}: $(cat "${OUT}")"; fi
assert_no_mutations

# 58. Guard over a deactivated equal-version marker: refresh allowed (enforced upstream).
set_fixtures 0 1 'let releaseTag = "wallet-bindings-v1.2.3"
            checksum: "0000000000000000000000000000000000000000000000000000000000000000"' pr.none
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "0" ]; then ok "guard allows refresh over deactivated equal marker (enforced upstream)"; else bad "guard allows refresh over deactivated equal marker" "exit ${rc}: $(cat "${OUT}")"; fi
assert_no_mutations

# 59. Guard: deactivated OLDER pin -> allow (forward release).
set_fixtures 0 1 'let releaseTag = "wallet-bindings-v0.1.0"
            checksum: "0000000000000000000000000000000000000000000000000000000000000000"' pr.none
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "0" ]; then ok "guard allows deactivated older pin"; else bad "guard allows deactivated older pin" "exit ${rc}"; fi

# 58b. Guard: unparseable origin pin tag (no version) -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ] && grep -q "unparseable pin tag" <<< "$(cat "${OUT}")"; then ok "guard refuses an unparseable origin pin tag"; else bad "guard refuses an unparseable origin pin tag" "exit ${rc}: $(cat "${OUT}")"; fi
assert_no_mutations

# 58c. Guard: an all-comments origin carries no code lines -> fail
# closed.
printf '// nothing but comments here\n/* still comments */\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ] && grep -q "carries no code lines" <<< "$(cat "${OUT}")"; then ok "guard refuses a comment-only origin"; else bad "guard refuses a comment-only origin" "exit ${rc}: $(cat "${OUT}")"; fi
assert_no_mutations

# 59b. Deactivated pin (zeroed checksum) whose marker is newer-or-equal
# -> refuse with the out-of-band comparator validation.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v9.9.9"\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ] && grep -q "carries a deactivated pin" <<< "$(cat "${OUT}")"; then
  ok "deactivated newer-or-equal marker refused"
else
  bad "deactivated newer-or-equal marker refused" "exit ${rc}: $(cat "${OUT}")"
fi
assert_no_mutations

# 59c. A releaseTag line without any checksum line -> refuse as
# mangled.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v9.9.9"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
# The upstream counts guard refuses this shape first (1 tag, 0
# checksums); the inner tag-without-checksum branch below it is
# unreachable for this fixture and stays as defense in depth.
if [ "${rc}" = "1" ] && grep -q "0 checksum assignments" <<< "$(cat "${OUT}")"; then ok "tag-without-checksum origin refused"; else bad "tag-without-checksum origin refused" "exit ${rc}: $(cat "${OUT}")"; fi
assert_no_mutations

# 60. Supersede: build-metadata suffix on the open pin branch compares
# EQUAL (metadata excluded from precedence) -> refuse like any >=
# marker.
set_fixtures 0 1 "${FULLY_PINNED}" pr.stale_meta
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ] && grep -q "close or merge it before releasing" <<< "$(cat "${OUT}")"; then
  ok "supersede refuses meta-suffix stale pin"
else
  bad "supersede refuses meta-suffix stale pin" "exit ${rc}: $(cat "${OUT}")"
fi
assert_no_mutations

# 60b. Supersede: a legacy v-suffix stale branch (pin-spm-checksum-
# v0.1.1 -> version 0.1.1 < 1.2.3) is closed like any older pin.
set_fixtures 0 1 "${FULLY_PINNED}" pr.stale_legacy
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "0" ] && grep -q -- "pr close 13" "${LOG}"; then
  ok "supersede closes a legacy v-suffix stale pin PR"
else
  bad "supersede closes a legacy v-suffix stale pin PR" "exit ${rc}: $(cat "${OUT}")"
fi

# 61. Supersede: plain newer stale marker (9.9.9, cmp_old = -1) ->
# refuse.
set_fixtures 0 1 "${FULLY_PINNED}" pr.stale_newer
: > "${LOG}"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
    > "${OUT}" 2>&1 )
rc=$?
LOG_CONTENTS="$(cat "${LOG}")"
if [ "${rc}" = "1" ] && grep -q "close or merge it before releasing" <<< "$(cat "${OUT}")"; then
  ok "supersede refuses a newer open pin PR"
else
  bad "supersede refuses a newer open pin PR" "exit ${rc}: $(cat "${OUT}")"
fi
assert_no_mutations
printf 'placeholder\n' > "${STUBS}/repo/Package.swift"

# 62. semver.sh unit checks (sourced directly).
# shellcheck source=/dev/null
semver_tests="${SCRIPT_DIR}/semver.sh"
semver_cmp_check() {
  # shellcheck disable=SC1090  # semver.sh is linted directly
  ( source "${semver_tests}"; semver_cmp "$1" "$2" )
}
if [ "$(semver_cmp_check "1.2.3" "1.2.3-rc.1")" = "1" ] \
  && [ "$(semver_cmp_check "1.2.3-rc.1" "1.2.3")" = "-1" ] \
  && [ "$(semver_cmp_check "1.2.3+meta.a" "1.2.3+meta.b")" = "0" ] \
  && [ "$(semver_cmp_check "1.2.3" "1.2.3+meta")" = "0" ] \
  && [ "$(semver_cmp_check "1.2.3-2" "1.2.3-18446744073709551617")" = "-1" ] \
  && [ "$(semver_cmp_check "1.02.3" "1.2.3")" = "0" ]; then
  ok "semver comparator unit checks"
else
  bad "semver comparator unit checks" "unexpected comparator output"
fi

# Static check: the TAG regex must be byte-identical in every copy
# Byte-identical TAG regex everywhere it appears: four occurrences in
# release.yml, one in open-spm-pr.sh, two in guard-release-refresh.sh,
# two in pin-order-gate.sh, one in check-pin-order.sh, and an explicit
# zero expectation for ci.yml (the gate moved into the tested script).
TAG_REGEX='^wallet-bindings-v[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$'
wf="$(grep -cF -- "${TAG_REGEX}" "${SCRIPT_DIR}/../../.github/workflows/release.yml" || true)"
sc="$(grep -cF -- "${TAG_REGEX}" "${UNDER_TEST}" || true)"
gd="$(grep -cF -- "${TAG_REGEX}" "${SCRIPT_DIR}/guard-release-refresh.sh" || true)"
cy="$(grep -cF -- "${TAG_REGEX}" "${SCRIPT_DIR}/../../.github/workflows/ci.yml" || true)"
gate="$(grep -cF -- "${TAG_REGEX}" "${SCRIPT_DIR}/pin-order-gate.sh" || true)"
cpo="$(grep -cF -- "${TAG_REGEX}" "${SCRIPT_DIR}/check-pin-order.sh" || true)"
if [ "${wf}" = "4" ] && [ "${sc}" = "1" ] && [ "${gd}" = "2" ] && [ "${cy}" = "0" ] && [ "${gate}" = "2" ] && [ "${cpo}" = "1" ]; then
  ok "tag regex identical across all ten copies"
else
  bad "tag regex identical across all ten copies" "release.yml=${wf}, script=${sc}, guard=${gd}, ci=${cy}, gate=${gate}, pin-order=${cpo}"
fi

echo "passed: ${pass}, failed: ${fail}"
[ "${fail}" = "0" ]
