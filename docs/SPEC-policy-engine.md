# Spec: policy-engine y guía de requisitos

El motor verifica y explica requisitos; ejecuta drivers de herramientas estándar registrados en JameSkills. Una suite declara el resultado exigido y los puntos donde se aplica, no shell arbitrario para "repararlo todo".

## Policy schema1

~~~
schema_version = 1
profile = "repository-foundation"
scope = "project"

[[tool_requirements]]
tool_id = "git"
operation = "repository-root"
version = ">=2.0.0"

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
depends_on = []
guidance_id = "github-protection"
[requirements.check]
kind = "github-branch-policy"
branch = "main"
require_pull_request = true
required_checks = ["quality"]
require_no_bypass = true
~~~

Enums phase = pre-install | commit | pull-request | ci | pre-release; severity info/warning/error. Checks AND internos; IDs únicos. depends_on apunta requisito mismo suite; ciclo rechaza. required y severity independientes: strict bloquea cualquier required no cumplido. NotApplicable solo applies_when con fact typed OS/stack/host/capability, un check nunca escoge NotApplicable al fallar.

Policy schema se parsea desde UTF-8 TOML, `schema_version=1`, scope `user|project`,
profile slug y requisitos no vacíos. tool_id/operation pertenecen al registry
estático y tienen rangos SemVer; cada check tiene DTO y campos permitidos propios.
Unknown fields, línea shell, dependencia ajena/duplicada/cíclica, path no portable,
enum o check desconocido se rechazan. Referencias a documentos de guía se validan
contra el bundle completo durante su ensamblado.

## Catálogo obligatorio v1

| Check kind | Driver y condición de Pass | Cuando no se sabe |
|---|---|---|
| git-repository | Git rev-parse, root/worktree válido | Missing Git Blocked |
| gitignore-patterns | Usar git check-ignore --no-index -v -z con synthetic paths app-registered y patterns exactos | Archivo existente solo no prueba protección; pattern sin probe registrado Unsupported |
| tracked-secrets | Gitleaks 8.30.1 exacto, redaction JSON y exit code distinto para findings; verificar versión/fingerprint antes del scan | Otra versión, driver faltante o identidad no aprobada Blocked; malformed report Unknown; nunca imprimir findings |
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

Driver VersionSpec tiene tool_id, probe argv, allowlist args, expected exit/JSON schema, supported versions contract y source_id. `profiles/tools.toml` es solo de app y contiene guías oficiales Windows/Linux con IDs documentados en `docs/SOURCES.md`; actualización versionada+fixture. URLs nunca vienen de manifests/skills. Drivers primero comprueban fingerprint aprobado del ejecutable y vuelven a verificarlo antes de spawn; un binario malicioso con nombre git no se declara confiable automáticamente.

El stack del proyecto se obtiene únicamente de `Cargo.toml` y `package.json` en
el root aprobado; el nombre/metadatos de una skill no intervienen. La inspección
es de solo lectura, limitada a 1 MiB por manifiesto, rechaza symlinks y entradas
no regulares, y solo expone nombres acotados de scripts Node, nunca sus valores.
No ejecuta scripts. Manifiesto ausente produce Generic si no hay otro reconocido;
formato inválido, demasiado grande o inaccesible produce Unknown, no Unsupported.
`detect_tools` combina profiles app-owned y candidatos explícitos; ejecuta un
probe solo cuando recibe el fingerprint SHA-256 aprobado correspondiente.

T019 Gitleaks `dir` versión exacta 8.30.1 analiza el working tree sin historial
con output redacted y bounded; es un scope más amplio que los paths versionados.
Otra versión permanece Blocked hasta tener fuente/fixture y contrato verificados.
El driver pasa una
config efímera privada que extiende las reglas default, y no carga `.gitleaks.toml`
del repo. Gitleaks también carga `.gitleaksignore` desde el source sin una opción
independiente para desactivarlo; si el archivo existe (incluso vacío o symlink),
el check devuelve Blocked sin ejecutar el driver. `include_history=true` se
reporta Unsupported hasta que el Git hijo del modo `gitleaks git` tenga una
identidad verificada y bound al mismo permiso. Nunca ejecutar ese modo confiando
en el PATH heredado.

## Autoridad y límites

`CheckObservation` distingue predicado, evidencia y autoridad realmente observada.
`evaluate_predicate` nunca eleva Unknown/Blocked a Pass: evidence necesaria ausente
o caducada pasa a Unknown; Pass sin evidencia también es Unknown. Una autoridad
inferior a la exigida convierte el resultado en Blocked, usando una matriz explícita
por variante (no orden enum). Un provider que responda NotApplicable no basta.
NotApplicable solo se genera cuando una condición `applies_when` del profile
registrado no coincide con un fact tipado fresco que trae su propia evidencia.

`strict_exit` devuelve 1 si falta o no pasa cualquier requirement `required`; los
recomendados no bloquean. `RuntimeServices` instala un PolicyCheckProvider que
devuelve Unknown mientras el driver real no esté conectado; nunca produce un Pass
de placeholder. Los TTL son monotónicos del proceso; `observed_at` RFC3339 es
informativo y no decide freshness.

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

Schema1 guidance: raíz schema_version y plans array. Plan {id, requirement_ids, steps}; Step {id, prompt_es, requires:step_ids, verification_requirement_ids, applies_when?, action}. action tagged por kind kebab-case: manual-instruction, open-official-url{source_id}, copy-approved-command{tool_id,operation}, select-local-path{purpose}, answer-choice{choices}, recheck. `requires` es DAG de pasos del mismo plan. `verification_requirement_ids` no puede estar vacío y debe ser subconjunto de `plan.requirement_ids`; esos checks son la condición efectiva de completar el step y necesitan evidencia fresca Pass, nunca un answer del usuario. `applies_when={fact,equals}`: fact enum os/arch/stack/host/context/capability; equals string del enum registrado y con evidence fresca. Unknown fact no selecciona rama como verdadera, produce pregunta/recheck. No expresiones scripting ni actions shell arbitrarias.

Fuente registry app por ID: git-install=https://git-scm.com/downloads; gitleaks=https://github.com/gitleaks/gitleaks; conventional-commits=https://www.conventionalcommits.org/en/v1.0.0/; github-cli=https://cli.github.com/manual/gh_auth_login; github-rulesets=https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets. Los ejemplos declaran IDs, nunca autorizan red arbitraria. La app extiende pasos genéricos con variante SO/tool-specific registrada y no ejecuta comandos solo por render.

Phase evaluation calcula dependency closure aunque prerequisito tenga phase distinta. Instalar user-scoped sin proyecto no ejecuta checks de repo: requirements cuyo check necesita project reciben NotApplicable por contexto confiable establecido en InstallRequest, y coverage se muestra limitado a instructions. Checks prerelease se evalúan exclusivamente cuando se pide ese gate. Bundle ejemplo inicial profiles Rust/GitHub; Node/generic aplica políticas/paths declarados para su stack y no exige target/ ni GitHub remoto si host distinto sin informar Unsupported.

## Evidencia de autoridad

requirement.enforcement es nivel exigido, CheckResult.enforcement es nivel observado. Pasar commitlint sobre el mensaje no prueba hook: inspeccionar configuración core.hooksPath efectiva, hook commit-msg gestionado/hashes/ejecución driver registrado, o dejar LocalCheck y explicar hook pendiente. LocalHook sigue eludible. Workflow YAML no prueba RequiredCi: pedir ruleset efectivo de host que exige quality y check-run SHA actual; son evidencia adicional al ci-contract. La app nunca copia el valor del manifest al resultado como evidencia de instalación. Tests cubren mensaje válido sin hook y workflow correcto sin regla de host: required authority no cumplida.
