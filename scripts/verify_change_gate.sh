#!/usr/bin/env bash
# Puerta local pre-push (patrón de producción JamePrompt): contrato de
# commitlint más espejo de CI. Solo orquesta herramientas oficiales;
# no valida nada por sí mismo.
set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"
cd "$repository_root"

npm run test:commitlint
bash scripts/check-workspace.sh
