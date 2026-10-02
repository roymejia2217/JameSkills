# Spec: policy-engine y guía de requisitos

El motor verifica y explica requisitos; ejecuta drivers de herramientas estándar registrados en JameSkills. Una suite declara el resultado exigido y los puntos donde se aplica, no shell arbitrario para "repararlo todo".

## Policy schema1

~~~
schema_version = 1
profile = "repository-foundation"

[[requirements]]
id = "readme-structure"
description = "README explica inicio, arquitectura y contribución."
severity = "error"
required = true
phase = "pre-install"
enforcement = "local-check"
depends_on = []
guidance_id = "readme-setup"
[requirements.check]
kind = "readme-sections"
path = "README.md"
headings = ["Inicio rápido", "Arquitectura", "Contribuir"]

[[requirements]]
id = "main-protected"
description = "Main requiere PR y checks obligatorios."
severity = "error"
required = true
phase = "pre-release"
enforcement = "host-rule"
depends_on = ["github-access"]
guidance_id = "github-protection"
[requirements.check]
kind = "github-branch-policy"
branch = "main"
require_pull_request = true
required_checks = ["quality"]
require_no_bypass = true
~~~

Enums phase = pre-install | commit | pull-request | ci | pre-release; severity info/warning/error. Checks AND internos; IDs únicos. depends_on apunta requisito mismo suite; ciclo rechaza. required y severity independientes: strict bloquea cualquier required no cumplido. NotApplicable solo applies_when con fact typed OS/stack/host/capability, un check nunca escoge NotApplicable al fallar.

## Catálogo obligatorio v1

| Check kind | Driver y condición de Pass | Cuando no se sabe |
|---|---|---|
| git-repository | Git rev-parse, root/worktree válido | Missing Git Blocked |
| gitignore-patterns | Usar git check-ignore --no-index con paths sintéticos seguros y normas del stack | Archivo existente solo no prueba protección |
| tracked-secrets | Gitleaks pinned driver, redaction y exit semantics documentadas; escanear tracked staged/history según fase | Driver falta Blocked; nunca imprimir findings sin redaction |
| readme-sections | Parser Markdown headings AST, sección no vacía, mínimo Inicio/arquitectura/contribuir/licencia configurado | Heading superficial vacío Fail |
| conventional-commit | Commitlint CLI+config Conventional Commits; input message temp seguro | commit message no disponible Unknown |
| protected-main-local | Branch actual y working mode PR | Advisory; no demuestra protección host |
| github-access | gh auth status + gh api lectura repo seleccionado | 401/403 Blocked guía permisos |
| github-branch-policy | Effective rulesets+branch protection sobre branch main, PR+checks activos, bypass evaluado | Permisos/API plan/bypass no visibles Unknown; combinación clásica+rulesets |
| ci-contract | Workflow configuración+jobs requeridos, no filtros que omitan obligatorios, permissions, refs pinned | YAML escrito no acredita último CI pasado |
| ci-evidence | gh API status/check-runs para SHA EXACTO y fuentes esperadas | Pending Blocked, SHA viejo Fail |
| release-contract | Semver tag, changelog actualizado, asset/checksum/config firma donde exige perfil | Protección tag o CI desconocida Unknown |
| toolchain-version | Registry version probe/range del perfil, no binario solo por nombre | Candidate sin identity ->Blocked |

repo profile Rust: cargo fmt/check clippy -Dwarnings/test y cargo-deny/audit; Node: npm ci, npm run lint/test/build con nombres detectados y explícitamente mapeados del proyecto. Nunca ejecutar package scripts importados sin trust del repo. Generic puede guiar definir commands y required CI, no presupone stack.

Driver VersionSpec tiene tool_id, probe argv, allowlist args, expected exit/JSON schema, supported versions contract. profiles/tools.toml solo de app; actualización versionado+fixture. Drivers primero check paths/procedencia de ejecutable; un binario malicious con nombre git no se declara confiable automáticamente.

## Autoridad y límites

Instruction enseña; LocalCheck valida una ejecución; LocalHook puede saltarse con --no-verify y no es security boundary; RequiredCi bloquea merge si host lo obliga; HostRule depende de repo/org/plan/permisos. Mostrar cobertura por requisito, no "100% garantizado" por cantidad archivos. Política externa de organización puede imponerse al config local; no debilitar rulesets para lograr un Pass.

Cambiar repo: plan_repo_changes produce diff de README/gitignore/workflow/commitlint solo managed sections o archivos nuevos. Mantener contenido ajeno y estructura existente. apply_repo_changes exige permiso concreto, hashes previos y journal; jamás git add/commit/push silencioso. Proteger main en remoto requiere preview settings, dueño autoriza cambio; v1 por defecto guía setup host y revalida API. No subir permisos por app ni tokens en manifest.

## Guía dinámica (sin magia)

guidance/<id>.toml schema1, planes reutilizables:
- prerequisites IDs de facts/checks; pasos con id, applies_when, requires, prompt_es y verification_requirement_ids.
- action enum ManualInstruction | OpenOfficialUrl | CopyApprovedCommand | SelectLocalPath | AnswerChoice | Recheck.
- URLs provienen registry oficial de app; skill puede sugerir enlace inerte reviewed pero no abrir arbitrary endpoint silencioso.
- CopyApprovedCommand representa argv de un driver a ejecutar POR usuario, redacted; botón copiar solo texto y no cambia status.
- El grafo tiene ramas por OS, architecture, tool presence, host permissions y stack; no es texto lineal fijo.
- next_step selecciona primero prereq bloqueante topológico; retries mismos facts no avanzan sin evidencia nueva.
- UserAnswer registra elección/camino no secreto. Datos secretos -> teclado credencial/keyring, fuera session JSON.
- "Lo completé" llama recheck; Pass con evidence fresh libera dependientes. Sin API posible ->AwaitingEvidence/ManualClaim, requisito sigue Unknown si exige machine-verification.
- Environment fingerprint por OS/arch/tool-version/repo SHA+remotes redacted relevante; cambiar branch/token/tool o revision invalida evidencia correspondiente, no toda biblioteca.
- Cooldown recheck y máximo intentos por sesión (10 antes ofrecer diagnóstico); cancel no deja tareas en loop.
- ManualClaim puede permitir continuar edición, no merge/strict que requiere Pass.

Caso Windows Pi falta Bash: detect pi y Linux tools ->guide Windows Git for Windows/custom shell -> usuario instala -> revisar path+version con operación trust -> Pi capability verified. No instalar Git automáticamente.
Caso branch ruleset falta permisos: mostrar permisos necesarios/link oficial; usuario pide admin configure; recheck remote. Otro trabajo local continúa.

## GUI, CLI y tests

Report rows: requisito/estado/autoridad/evidencia/acción. check CLI --strict no-success si required Unknown/Blocked/Missing. Guidance sidebar muestra paso+motivo+Verificar; nunca banner "listo" basado selección.
Tests policy.rs parsing, dependency applicability, all exit statuses, bypass classic+rulesets fixtures, stale CI SHA.
guidance.rs fact branch Linux vs Windows, missing tool, denied permission, manual claim remains Unknown, graph cycle rejected, stale facts invalidation.
infra/github.rs gh JSON redacted+bounded+timeouts; repo desde selection, no URLs skill.
Aceptación: una suite incompleta dice qué falta, conduce pasos correctos y solo declara validado con fuente verificable.

## Serialización de guía y condiciones

Schema1 guidance: raíz schema_version y plans array. Plan {id, requirement_ids, steps}; Step {id, prompt_es, requires:step_ids, verification_requirement_ids, applies_when?, action}. action tagged por kind kebab-case: manual-instruction, open-official-url{source_id}, copy-approved-command{tool_id,operation}, select-local-path{purpose}, answer-choice{choices}, recheck. requires es DAG de pasos del mismo plan; los requirement IDs existen en policies. applies_when={fact,equals}: fact enum os/arch/stack/host/context/capability; equals string de enum registrado. No expresiones scripting. Unknown fact no selecciona rama como verdadera, produce pregunta/recheck.

Fuente registry app por ID: git-install=https://git-scm.com/downloads; gitleaks=https://github.com/gitleaks/gitleaks; conventional-commits=https://www.conventionalcommits.org/en/v1.0.0/; github-cli=https://cli.github.com/manual/gh_auth_login; github-rulesets=https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets. Los ejemplos declaran IDs, nunca autorizan red arbitraria. La app extiende pasos genéricos con variante SO/tool-specific registrada y no ejecuta comandos solo por render.

Phase evaluation calcula dependency closure aunque prerequisito tenga phase distinta. Instalar user-scoped sin proyecto no ejecuta checks de repo: requirements cuyo check necesita project reciben NotApplicable por contexto confiable establecido en InstallRequest, y coverage se muestra limitado a instructions. Checks prerelease se evalúan exclusivamente cuando se pide ese gate. Bundle ejemplo inicial profiles Rust/GitHub; Node/generic aplica políticas/paths declarados para su stack y no exige target/ ni GitHub remoto si host distinto sin informar Unsupported.

## Evidencia de autoridad

requirement.enforcement es nivel exigido, CheckResult.enforcement es nivel observado. Pasar commitlint sobre el mensaje no prueba hook: inspeccionar configuración core.hooksPath efectiva, hook commit-msg gestionado/hashes/ejecución driver registrado, o dejar LocalCheck y explicar hook pendiente. LocalHook sigue eludible. Workflow YAML no prueba RequiredCi: pedir ruleset efectivo de host que exige quality y check-run SHA actual; son evidencia adicional al ci-contract. La app nunca copia el valor del manifest al resultado como evidencia de instalación. Tests cubren mensaje válido sin hook y workflow correcto sin regla de host: required authority no cumplida.
