#!/usr/bin/env bash
# Unit tests for scripts/release/open-spm-pr.sh and
# scripts/release/guard-release-refresh.sh using stub git/gh on PATH.
# Covers: tag-ancestry guard (shallow-aware fetch both ways, git-error
# disambiguation), HEAD==tag guard, both substitution verifications (pin
# fully present / either half drifted / absent), already-merged,
# pre-pinned and reverted-main short-circuits, semver rewind guard
# (forward allowed, stale/same-version refused, one-sided and mangled
# origin pins refused), owner/cross-repo/base-branch PR detection,
# force-push ordering and its absence on failure paths,
# owner/branch/checksum/tag/version validation, and the byte-identical
# TAG regex across all copies. Run by lint.yml and locally before
# releases.
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
# forgery and control characters before echoing.
sanitize() {
  # PR-controlled output: replace every '::' so no workflow command can
  # survive, per line. LF/CR are intentionally preserved — deleting them
  # could splice two benign fragments into a new command — and each
  # resulting line is safe because '::' no longer occurs in it.
  printf '%s' "$1" | tr -d '\000-\010\011\013\014\016-\037' | sed 's/::/[::]/g'
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
  "fetch origin")
    [ "$(cat "${CASE_DIR}/fetch_fails" 2>/dev/null || echo 0)" = "1" ] && exit 1
    exit 0 ;;
  "rev-parse HEAD") cat "${CASE_DIR}/head_commit" ;;
  "show origin/"*)
    if [ "$(cat "${CASE_DIR}/show_fails" 2>/dev/null || echo 0)" = "1" ]; then
      exit 1
    fi
    cat "${CASE_DIR}/origin_swift" ;;
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
if [ "$1" = "pr" ] && [ "$2" = "list" ]; then
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

# 5c. Pin already merged: Package.swift identical to the default branch
# -> short-circuit success even though the checkout diff is non-empty.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none 0
run_case "already-merged pin exits 0" 0
assert_no_mutations

# 7. Open same-repo PR against main -> detected, no duplicate create.
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

# 8. Open different-owner PR with colliding head branch -> create proceeds.
set_fixtures 0 1 "${FULLY_PINNED}" pr.evillow
run_case "different-owner PR ignored, create proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create despite different-owner PR"
else
  bad "create despite different-owner PR" "${LOG_CONTENTS}"
fi

# 9. Open fork PR with colliding head branch -> create proceeds.
set_fixtures 0 1 "${FULLY_PINNED}" pr.fork
run_case "fork PR ignored, create proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create despite fork PR"
else
  bad "create despite fork PR" "${LOG_CONTENTS}"
fi

# 10. Open same-owner PR against a non-default base -> create proceeds.
set_fixtures 0 1 "${FULLY_PINNED}" pr.wrongbase
run_case "wrong-base PR ignored, create proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create despite wrong-base PR"
else
  bad "create despite wrong-base PR" "${LOG_CONTENTS}"
fi

# 11. Malformed repository owner -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
run_case "invalid owner fails" 1 "bad..owner/rust-sdk"
assert_no_mutations

# 12. Malformed DEFAULT_BRANCH -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid default branch fails" 1 "mintlayer/rust-sdk" $'ma\tin'
assert_no_mutations

# 13. Malformed XCSUM -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid checksum fails" 1 "mintlayer/rust-sdk" main "xyz"
assert_no_mutations

# 14. HEAD is not the tagged commit -> fail closed, no push.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'othercommitsha' > "${TMP}/head_commit"
run_case "HEAD differs from tag fails" 1
assert_no_mutations
printf 'tagcommitsha' > "${TMP}/head_commit"

# 15. All-zero checksum -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "all-zero checksum fails" 1 "mintlayer/rust-sdk" main \
  "0000000000000000000000000000000000000000000000000000000000000000"
assert_no_mutations

# 16. VERSION that is not semver -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid version fails" 1 "mintlayer/rust-sdk" main "${XCSUM}" 1.2
assert_no_mutations

# 17. TAG not matching the release pattern -> fail closed before fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid tag (no patch) fails" 1 "mintlayer/rust-sdk" main "${XCSUM}" 1.2 "wallet-bindings-v1.2"
assert_no_mutations
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid tag (wrong prefix) fails" 1 "mintlayer/rust-sdk" main "${XCSUM}" 1.2.3 "evil-wallet-bindings-v1.2.3"
assert_no_mutations

# 18. Default branch inside the reserved namespace -> fail closed.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "reserved-namespace default branch fails" 1 "mintlayer/rust-sdk" "pin-spm-checksum-1.2.3"
assert_no_mutations

# 19. Shallow clone -> the unshallow fetch is used.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'true' > "${TMP}/is_shallow"
run_case "shallow clone unshallows" 0
if grep -q -- "fetch --unshallow origin" <<< "${LOG_CONTENTS}"; then
  ok "unshallow fetch issued"
else
  bad "unshallow fetch issued" "${LOG_CONTENTS}"
fi
printf 'false' > "${TMP}/is_shallow"

# 20. Complete clone -> plain fetch with the explicit full refspec.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
run_case "complete clone plain fetch" 0
if grep -q -- "fetch origin +refs/heads/main:refs/remotes/origin/main" <<< "${LOG_CONTENTS}"; then
  ok "plain fetch carries the full refspec"
else
  bad "plain fetch carries the full refspec" "${LOG_CONTENTS}"
fi

# 21. Default branch already pins a DIFFERENT release -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v9.9.9"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
run_case "stale tag against a newer pin fails" 1
assert_no_mutations

# 22. Forward release over an OLDER active pin -> allowed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v0.1.0"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
run_case "forward release over older pin proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create proceeds over older pin"
else
  bad "create proceeds over older pin" "${LOG_CONTENTS}"
fi
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 23. Same-version active pin with differing content -> rewind guard.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
run_case "same-version conflicting pin fails" 1
assert_no_mutations
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 24. merge-base git error (rc > 1) -> fail closed, distinct from rc 1.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf '128' > "${TMP}/ancestry"
run_case "merge-base git error fails" 1
assert_no_mutations
printf '0' > "${TMP}/ancestry"

# 25. Unparseable origin pin tag -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-vx"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
run_case "unparseable origin pin tag fails" 1
assert_no_mutations
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 26. Active checksum with missing tag line -> fail closed (one-sided).
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'checksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
run_case "one-sided active checksum fails" 1
assert_no_mutations
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 27. TAG/VERSION mismatch -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "mismatched tag and version fails" 1 "mintlayer/rust-sdk" main "${XCSUM}" 9.9.9
assert_no_mutations

# 28. Mangled checksum line (valid tag, non-hex checksum) -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "deadbeef"\n' > "${TMP}/origin_swift"
run_case "mangled origin checksum fails" 1
assert_no_mutations
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 29. Prerelease over final (1.2.3-rc.1 after 1.2.3) -> rewind, refused.
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
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 30. Final over prerelease (1.2.3 after 1.2.3-rc1) -> allowed forward.
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
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 31. Origin pin 1.2.3+meta.b vs incoming 1.2.3+meta.a compares EQUAL -> refuse.
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

# 29c. Origin pin without metadata vs incoming with metadata: equal
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

# 33. Guard: mangled checksum (65+ hex) on origin -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b9700"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" GITHUB_REPOSITORY="mintlayer/rust-sdk" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard refuses 65-hex checksum"; else bad "guard refuses 65-hex checksum" "exit ${rc}"; fi
assert_no_mutations

# 34. Huge numeric prerelease identifiers compare by (digit length,
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
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 35. Guard script: same tag + differing checksum -> refuse.
GUARD="${SCRIPT_DIR}/guard-release-refresh.sh"
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${GUARD}" wallet-bindings-v1.2.3 "1111111111111111111111111111111111111111111111111111111111111111" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard refuses mismatched rebuild"; else bad "guard refuses mismatched rebuild" "exit ${rc}"; fi

# 36. Guard script: same tag + identical checksum (with a header
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

# 37. Guard script: older pin on main (forward release) -> allow.
printf 'let releaseTag = "wallet-bindings-v0.1.0"\nchecksum: "cad323bf1d59c87ba0c252952933ea467f72afc0eda8e17845da4fe78d7b0654"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${GUARD}" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "guard allows forward release"; else bad "guard allows forward release" "exit ${rc}"; fi
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 38. git show fails -> open-spm-pr.sh fails closed (no silent skip).
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
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 39. Comment decoys inside an unterminated block comment; the real
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
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 40. Same decoy shape for the guard: real v1.0.0 pin follows the
# block, so the refresh (1.2.3 > 1.0.0) must be allowed.
printf '/*\nchecksum: "1111111111111111111111111111111111111111111111111111111111111111"\nlet releaseTag = "wallet-bindings-v1.2.3"\nlet releaseTag = "wallet-bindings-v9.9.9"\n*/\nlet releaseTag = "wallet-bindings-v1.0.0"\nchecksum: "%s"\n' "${XCSUM}" > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "0" ]; then ok "guard ignores comment decoys"; else bad "guard ignores comment decoys" "exit ${rc}"; fi

# 40b. Guard: a failed origin refresh must fail closed (no clobber
# decision can be made against a stale pin view).
printf '1' > "${TMP}/fetch_fails"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
printf '0' > "${TMP}/fetch_fails"
if [ "${rc}" = "1" ]; then ok "guard origin refresh failure fails closed"; else bad "guard origin refresh failure fails closed" "exit ${rc}"; fi
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 41. Guard: zero releaseTag assignments -> refuse.
printf 'checksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard refuses zero assignments"; else bad "guard refuses zero assignments" "exit ${rc}"; fi

# 42. Guard: two releaseTag assignments -> refuse.
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nlet releaseTag = "wallet-bindings-v9.9.9"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard refuses duplicate assignments"; else bad "guard refuses duplicate assignments" "exit ${rc}"; fi

# 43. Guard: invalid checksum -> refuse.
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "nothex" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard rejects invalid checksum"; else bad "guard rejects invalid checksum" "exit ${rc}"; fi

# 44. Guard: git show failure -> fail closed.
printf '1' > "${TMP}/show_fails"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${SCRIPT_DIR}/guard-release-refresh.sh" wallet-bindings-v1.2.3 "${XCSUM}" main \
    >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "guard fails closed on git show failure"; else bad "guard fails closed on git show failure" "exit ${rc}"; fi
printf '0' > "${TMP}/show_fails"
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"

# 45. Pin-order script: upgrade allows, downgrade refuses, same refuses.
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
  bash "${ORDER}" main "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order refuses downgrade"; else bad "pin order refuses downgrade" "exit ${rc}"; fi

printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "%s"\n' "${XCSUM}" > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order refuses same version"; else bad "pin order refuses same version" "exit ${rc}"; fi

# 46. Reserved-namespace base branch -> fail closed.
# shellcheck disable=SC2016  # fixture line content is not expanded
printf 'let releaseTag = "wallet-bindings-v0.9.0"\nchecksum: "${XCSUM}"\n' > "${STUBS}/repo/Package.swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" "pin-spm-checksum-1.2.3" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order rejects reserved namespace"; else bad "pin order rejects reserved namespace" "exit ${rc}"; fi

# 38c. Ambiguous PR head (a second releaseTag line) -> strict mode
# refuses: the normalized head must carry exactly the expected
# assignment and no other releaseTag- or checksum-bearing line.
# shellcheck disable=SC2016  # fixture line content is not expanded
printf 'let releaseTag = "wallet-bindings-v1.3.0"\nchecksum: "${XCSUM}"\nlet releaseTag = "wallet-bindings-v1.2.3"\n' > "${STUBS}/repo/Package.swift"
printf 'let releaseTag = "wallet-bindings-v1.2.3"\nchecksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"\n' > "${TMP}/origin_swift"
( cd "${STUBS}/repo" && \
  PATH="${STUBS}:${PATH}" CASE_DIR="${TMP}" \
  bash "${ORDER}" main "wallet-bindings-v1.3.0" >/dev/null 2>&1 )
rc=$?
if [ "${rc}" = "1" ]; then ok "pin order refuses ambiguous PR head"; else bad "pin order refuses ambiguous PR head" "exit ${rc}"; fi

# 47. git show failure on the base -> fail closed.
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

# 48a. Strict mode PASS path over the REAL root Package.swift shape:
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

# 48b. Supersede: an older stale pin PR is closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.stale_older
printf 'let releaseTag = "wallet-bindings-v0.0.0" // placeholder\nchecksum: "0000000000000000000000000000000000000000000000000000000000000000"\n' > "${TMP}/origin_swift"
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

# 48c. Supersede: a newer open pin PR refuses the run.
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

# 48b-2. Guard over a deactivated newer-or-equal marker: refresh allowed (rewind refusal enforced upstream by open-spm-pr.sh).
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

# 48b-2b. Guard over a deactivated equal-version marker: refresh allowed (enforced upstream).
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

# 48b-2c. Guard: deactivated OLDER pin -> allow (forward release).
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

# 48b-2d. Supersede: meta-suffix stale marker (>= precedence) -> refuse.
set_fixtures 0 1 "${FULLY_PINNED}" pr.stale_newer
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
printf 'placeholder\n' > "${STUBS}/repo/Package.swift"

# 48. semver.sh unit checks (sourced directly).
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
# (three occurrences in release.yml, one in open-spm-pr.sh, one in
# guard-release-refresh.sh).
TAG_REGEX='^wallet-bindings-v[0-9]{1,18}\.[0-9]{1,18}\.[0-9]{1,18}([-+][0-9A-Za-z.-]+)?$'
wf="$(grep -cF -- "${TAG_REGEX}" "${SCRIPT_DIR}/../../.github/workflows/release.yml" || true)"
sc="$(grep -cF -- "${TAG_REGEX}" "${UNDER_TEST}" || true)"
gd="$(grep -cF -- "${TAG_REGEX}" "${SCRIPT_DIR}/guard-release-refresh.sh" || true)"
cy="$(grep -cF -- "${TAG_REGEX}" "${SCRIPT_DIR}/../../.github/workflows/ci.yml" || true)"
if [ "${wf}" = "3" ] && [ "${sc}" = "1" ] && [ "${gd}" = "2" ] && [ "${cy}" = "1" ]; then
  ok "tag regex identical across all seven copies"
else
  bad "tag regex identical across all seven copies" "release.yml=${wf}, script=${sc}, guard=${gd}, ci=${cy}"
fi

echo "passed: ${pass}, failed: ${fail}"
[ "${fail}" = "0" ]
