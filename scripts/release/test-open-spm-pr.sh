#!/usr/bin/env bash
# Unit tests for scripts/release/open-spm-pr.sh using stub git/gh on PATH.
# Covers: tag-ancestry guard, empty-diff idempotency (pin present vs
# drifted file), owner/cross-repo PR detection, force-push ordering, and
# the create path. Run by lint.yml and locally before releases.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UNDER_TEST="${SCRIPT_DIR}/open-spm-pr.sh"
TMP="$(mktemp -d)"
STUBS="${TMP}/stubs"
LOG="${TMP}/stub.log"
trap 'rm -rf "${TMP}"' EXIT
mkdir -p "${STUBS}/repo"
export LOG

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
  "checkout -B")
    # The script runs git checkout inside the repo dir; copy the fixture
    # next to it so `git add Package.swift` / `grep` see controlled data.
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
  if [ -n "${prog}" ] && command -v jq >/dev/null 2>&1; then
    printf '%s' "${payload}" | jq -r "${prog}"
  else
    printf '%s' "${payload}"
  fi
fi
exit 0
EOF
chmod +x "${STUBS}/git" "${STUBS}/gh"

XCSUM="aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"
PINNED="let releaseTag = \"wallet-bindings-v1.2.3\" // ...
checksum: \"${XCSUM}\""
DRIFTED="let releaseTag = \"wallet-bindings-v9.9.9\" // ...
checksum: \"deadbeef\""

LOG_CONTENTS=""
run_case() { # <name> <expected_exit>
  local name="$1" expected="$2" actual=0
  ( cd "${STUBS}/repo" && \
    PATH="${STUBS}:${PATH}" \
    GITHUB_REPOSITORY="mintlayer/rust-sdk" \
    CASE_DIR="${TMP}" \
    bash "${UNDER_TEST}" wallet-bindings-v1.2.3 1.2.3 "${XCSUM}" main \
      >/dev/null 2>&1 ) || actual=$?
  LOG_CONTENTS="$(cat "${LOG}" 2>/dev/null || true)"
  if [ "${actual}" = "${expected}" ]; then ok "${name}"; else bad "${name}" "exit ${actual}, want ${expected}"; fi
  : > "${LOG}"
}

printf '%s' "${PINNED}" > "${TMP}/package_swift.pinned"
printf '%s' "${DRIFTED}" > "${TMP}/package_swift.drifted"
echo '[]' > "${TMP}/existing_pr.empty"
echo '[{"number": 9, "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": false}]' > "${TMP}/existing_pr.own"
echo '[{"number": 9, "headRepositoryOwner": {"login": "mintlayer"}, "isCrossRepository": true}]' > "${TMP}/existing_pr.fork"

# 1. Tag not an ancestor of the default branch -> fail loudly.
printf '1' > "${TMP}/ancestry"; printf '1' > "${TMP}/staged_diff"
cp "${TMP}/package_swift.drifted" "${TMP}/package_swift"
run_case "non-ancestor tag fails" 1

# 2. Ancestor + real diff -> success; push issued before create.
printf '0' > "${TMP}/ancestry"; printf '1' > "${TMP}/staged_diff"
cp "${TMP}/package_swift.drifted" "${TMP}/package_swift"
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

# 3. Empty diff + pin genuinely present -> idempotent success, no create.
printf '0' > "${TMP}/ancestry"; printf '0' > "${TMP}/staged_diff"
cp "${TMP}/package_swift.pinned" "${TMP}/package_swift"
run_case "empty diff with pin exits 0" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  bad "no create when pinned" "${LOG_CONTENTS}"
else
  ok "no create when pinned"
fi

# 4. Empty diff + pin ABSENT (sed no-op'd on drifted file) -> fail closed.
printf '0' > "${TMP}/ancestry"; printf '0' > "${TMP}/staged_diff"
cp "${TMP}/package_swift.drifted" "${TMP}/package_swift"
run_case "empty diff without pin fails" 1

# 5. Open same-repo PR exists -> detected, no duplicate create.
printf '0' > "${TMP}/ancestry"; printf '1' > "${TMP}/staged_diff"
cp "${TMP}/package_swift.drifted" "${TMP}/package_swift"
cp "${TMP}/existing_pr.own" "${TMP}/existing_pr"
run_case "own open PR detected" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  bad "no duplicate create" "${LOG_CONTENTS}"
else
  ok "no duplicate create"
fi

# 6. Open fork PR with colliding head branch -> ignored, create proceeds.
cp "${TMP}/existing_pr.fork" "${TMP}/existing_pr"
run_case "fork PR ignored, create proceeds" 0
if grep -q "gh pr create" <<< "${LOG_CONTENTS}"; then
  ok "create despite fork PR"
else
  bad "create despite fork PR" "${LOG_CONTENTS}"
fi

echo "passed: ${pass}, failed: ${fail}"
[ "${fail}" = "0" ]
