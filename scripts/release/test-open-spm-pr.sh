#!/usr/bin/env bash
# Unit tests for scripts/release/open-spm-pr.sh using stub git/gh on PATH.
# Covers: tag-ancestry guard, both substitution verifications (pin fully
# present / partially drifted / absent), owner/cross-repo/base-branch PR
# detection, force-push ordering, and owner validation. Run by lint.yml
# and locally before releases.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UNDER_TEST="${SCRIPT_DIR}/open-spm-pr.sh"
TMP="$(mktemp -d)"
STUBS="${TMP}/stubs"
LOG="${TMP}/stub.log"
OUT="${TMP}/out.log"
trap 'rm -rf "${TMP}"' EXIT
mkdir -p "${STUBS}/repo"
export LOG

command -v jq >/dev/null 2>&1 || { echo "jq is required by this test harness"; exit 1; }

pass=0
fail=0
ok() { pass=$((pass + 1)); echo "ok - $1"; }
bad() { fail=$((fail + 1)); echo "NOT OK - $1: $2"; }

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
fi
exit 0
EOF
chmod +x "${STUBS}/git" "${STUBS}/gh"

XCSUM="aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"
FULLY_PINNED='let releaseTag = "wallet-bindings-v1.2.3" // ...
checksum: "'"${XCSUM}"'"'
PARTIAL_DRIFT='let releaseTag = "wallet-bindings-v1.2.2" // ...
checksum: "'"${XCSUM}"'"'
NOT_PINNED='let releaseTag = "wallet-bindings-v9.9.9" // ...
checksum: "deadbeef"'

# PR fixtures; baseRefName is main unless stated.
echo '[]' > "${TMP}/pr.none"
echo '[{"number": 9, "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": false, "baseRefName": "main"}]' > "${TMP}/pr.own"
echo '[{"number": 9, "headRepositoryOwner": {"login": "evil"}, "isCrossRepository": false, "baseRefName": "main"}]' > "${TMP}/pr.evillow"
echo '[{"number": 9, "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": true, "baseRefName": "main"}]' > "${TMP}/pr.fork"
echo '[{"number": 9, "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": false, "baseRefName": "develop"}]' > "${TMP}/pr.wrongbase"

# set_fixtures <ancestry> <staged_diff> <package_swift> <existing_pr> [default_diff]
# default_diff: 0 = Package.swift identical to the default branch (pin merged).
set_fixtures() {
  printf '%s' "$1" > "${TMP}/ancestry"
  printf '%s' "$2" > "${TMP}/staged_diff"
  printf '%s' "$3" > "${TMP}/package_swift"
  cp "${TMP}/$4" "${TMP}/existing_pr"
  printf '%s' "${5:-1}" > "${TMP}/default_diff"
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
run_case() {
  local name="$1" expected="$2" repo="${3:-mintlayer/rust-sdk}"
  local branch="${4:-main}" xcsum="${5:-${XCSUM}}" actual=0
  : > "${LOG}"
  ( cd "${STUBS}/repo" && \
    PATH="${STUBS}:${PATH}" \
    GITHUB_REPOSITORY="${repo}" \
    CASE_DIR="${TMP}" \
    bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${xcsum}" "${branch}" \
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
run_case "diff present creates PR" 0
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

# 3. Empty diff + both substitutions present -> idempotent success.
set_fixtures 0 0 "${FULLY_PINNED}" pr.none 1
run_case "empty diff with pin exits 0" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  bad "no create when pinned" "${LOG_CONTENTS}"
else
  ok "no create when pinned"
fi

# 4. Empty diff + pin absent -> fail closed.
set_fixtures 0 0 "${NOT_PINNED}" pr.none
run_case "empty diff without pin fails" 1
assert_no_mutations

# 5. Partial drift: new checksum but OLD tag -> fail closed (the sed for
# the tag no-op'd; publishing would break the binaryTarget URL).
set_fixtures 0 1 "${PARTIAL_DRIFT}" pr.none
run_case "partial drift (checksum new, tag old) fails" 1
assert_no_mutations

# 5b. Mirror drift: new tag but STALE checksum -> fail closed (the sed
# for the checksum no-op'd; SPM consumers would resolve a stale artifact).
OLD_XCSUM="1111111111111111111111111111111111111111111111111111111111111111"
STALE_CKSUM='let releaseTag = "wallet-bindings-v1.2.3" // ...
checksum: "'"${OLD_XCSUM}"'"'
set_fixtures 0 1 "${STALE_CKSUM}" pr.none
run_case "mirror drift (tag new, checksum stale) fails" 1
assert_no_mutations

# 5c. Pin already merged: Package.swift identical to the default branch
# -> short-circuit success even though the checkout diff is non-empty.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none 0
run_case "already-merged pin exits 0" 0

# 6. Open same-repo PR against main -> detected, no duplicate create.
set_fixtures 0 1 "${FULLY_PINNED}" pr.own
run_case "own open PR detected" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  bad "no duplicate create" "${LOG_CONTENTS}"
else
  ok "no duplicate create"
fi

# 7. Open different-owner PR with colliding head branch -> create proceeds.
set_fixtures 0 1 "${FULLY_PINNED}" pr.evillow
run_case "different-owner PR ignored, create proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create despite different-owner PR"
else
  bad "create despite different-owner PR" "${LOG_CONTENTS}"
fi

# 8. Open fork PR with colliding head branch -> create proceeds.
set_fixtures 0 1 "${FULLY_PINNED}" pr.fork
run_case "fork PR ignored, create proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create despite fork PR"
else
  bad "create despite fork PR" "${LOG_CONTENTS}"
fi

# 9. Open same-owner PR against a non-default base -> create proceeds.
set_fixtures 0 1 "${FULLY_PINNED}" pr.wrongbase
run_case "wrong-base PR ignored, create proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create despite wrong-base PR"
else
  bad "create despite wrong-base PR" "${LOG_CONTENTS}"
fi

# 10. Malformed repository owner -> fail closed.
set_fixtures 0 1 "${FULLY_PINNED}" pr.none
run_case "invalid owner fails" 1 "bad..owner/rust-sdk"
assert_no_mutations

# 11. Malformed DEFAULT_BRANCH -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid default branch fails" 1 "mintlayer/rust-sdk" $'ma\tin'
assert_no_mutations

# 12. Malformed XCSUM -> fail closed before any fetch.
set_fixtures 0 1 "${NOT_PINNED}" pr.none
run_case "invalid checksum fails" 1 "mintlayer/rust-sdk" main "xyz"
assert_no_mutations

# Static check: the TAG regex must be byte-identical in every copy
# (three occurrences in release.yml, one in the script under test).
TAG_REGEX='^wallet-bindings-v[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$'
wf="$(grep -cF -- "${TAG_REGEX}" "${SCRIPT_DIR}/../../.github/workflows/release.yml" || true)"
sc="$(grep -cF -- "${TAG_REGEX}" "${UNDER_TEST}" || true)"
if [ "${wf}" = "3" ] && [ "${sc}" = "1" ]; then
  ok "tag regex identical across all four copies"
else
  bad "tag regex identical across all four copies" "release.yml=${wf}, script=${sc}"
fi

echo "passed: ${pass}, failed: ${fail}"
[ "${fail}" = "0" ]
