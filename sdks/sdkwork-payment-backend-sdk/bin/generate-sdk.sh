#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FAMILY_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
GENERATOR_PATH="${FAMILY_ROOT}/../../sdkwork-sdk-generator/bin/sdkgen.js"
INPUT_PATH="${FAMILY_ROOT}/openapi/sdkwork-payment-backend-api.sdkgen.yaml"
SDK_NAME="sdkwork-payment-backend-sdk"
BASE_URL="${BASE_URL:-http://localhost:8080}"
SDK_VERSION="${SDK_VERSION:-1.0.0}"
API_PREFIX="/backend/v3/api"
LANGUAGES="${LANGUAGES:-typescript}"

if [[ ! -f "${GENERATOR_PATH}" ]]; then
  echo "Canonical SDK generator not found: ${GENERATOR_PATH}" >&2
  echo "Materialize the sdkwork-sdk-generator workspace (sibling of sdkwork-payment) before running this script." >&2
  echo "Per SDK_WORKSPACE_GENERATION_SPEC.md the generator MUST be present; the script refuses to fall back to a missing or partial generator." >&2
  exit 1
fi

if [[ ! -f "${INPUT_PATH}" ]]; then
  echo "OpenAPI sdkgen input not found: ${INPUT_PATH}" >&2
  echo "The authority spec must be mirrored into the family openapi/ directory before generation." >&2
  exit 1
fi

package_name() {
  case "$1" in
    typescript) echo "@sdkwork/payment-backend-sdk" ;;
    *) echo "sdkwork-payment-backend-sdk-$1" ;;
  esac
}

IFS=',' read -r -a language_array <<< "${LANGUAGES}"
for language in "${language_array[@]}"; do
  language="$(echo "${language}" | xargs)"
  [[ -z "${language}" ]] && continue
  output_path="${FAMILY_ROOT}/${SDK_NAME}-${language}"
  node "${GENERATOR_PATH}" generate \
    -i "${INPUT_PATH}" \
    -o "${output_path}" \
    -n "${SDK_NAME}" \
    -t backend \
    -l "${language}" \
    --fixed-sdk-version "${SDK_VERSION}" \
    --base-url "${BASE_URL}" \
    --api-prefix "${API_PREFIX}" \
    --package-name "$(package_name "${language}")" \
    --standard-profile sdkwork-v3 \
    --sdk-root "${FAMILY_ROOT}" \
    --sdk-name "${SDK_NAME}" \
    --no-sync-published-version
done
