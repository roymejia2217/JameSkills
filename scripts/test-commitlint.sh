#!/usr/bin/env bash
# Contrato de commitlint portado de JamePrompt (producción): casos válidos e
# inválidos contra las versiones fijadas en package.json. Ejemplos adaptados
# al dominio JameSkills; las reglas son las oficiales de config-conventional.
set -euo pipefail

commitlint() {
  npm exec --no -- commitlint --verbose
}

assert_rejected() {
  local name="$1"
  local message="$2"

  if printf '%s\n' "$message" | commitlint; then
    echo "Expected Commitlint to reject: $name" >&2
    exit 1
  fi
}

valid_message=$'feat(desktop): add shell with routes\n\nWire the Sidebar catalog primitives to the route state without IO in render.'
valid_body_and_footer=$'fix(infra): isolate Windows data and cache paths\n\nKeep the existing overlap rejection across case variants.\n\nRefs: #123'

printf '%s\n' "$valid_message" | commitlint
printf '%s\n' "$valid_body_and_footer" | commitlint
printf '%s\n' 'feat(desktop): add shell with routes' | npm exec --no -- commitlint --config commitlint.title.config.cjs --verbose

assert_rejected 'missing body' 'feat(desktop): add shell with routes'
assert_rejected 'missing type' $'Update shell routes\n\nDescribe the navigation behavior for the supported user workflow.'
assert_rejected 'unknown type' $'unknown(desktop): add shell with routes\n\nDescribe the navigation behavior for the supported user workflow.'
assert_rejected 'bracket scope' $'fix[infra]: isolate Windows paths\n\nKeep the existing overlap rejection across case variants.'
assert_rejected 'uppercase subject' $'feat(desktop): Add shell with routes\n\nDescribe the navigation behavior for the supported user workflow.'
assert_rejected 'subject with period' $'feat(desktop): add shell with routes.\n\nDescribe the navigation behavior for the supported user workflow.'
assert_rejected 'header over 100 characters' $'feat(desktop): add a shell navigation flow with a header that is intentionally longer than one hundred characters to verify\n\nDescribe the navigation behavior for the supported user workflow.'
assert_rejected 'body without leading blank' $'fix(infra): isolate Windows paths\nKeep the existing overlap rejection across case variants.'
assert_rejected 'footer without leading blank' $'fix(infra): isolate Windows paths\n\nKeep the existing overlap rejection across case variants.\nRefs: #123'

echo "commit message contract: ok"
