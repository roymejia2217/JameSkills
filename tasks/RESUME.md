# Reanudación JameSkills

Fecha UTC: 2026-10-04
Rama / HEAD: `feat/t020-commit-test-checks` / `HEAD` (base de slice `caa9a23`; T020.a `1cd44fc`, T020.b1 `d304f64`).
Base: `main`=`caa9a23`, merge squash de PR #21.
PR / CI remota previa: PR #21 fusionada; CI Linux/Windows, tests, fmt, Clippy, commitlint, README Policy y Required CI finalizaron SUCCESS.

## Estado real

- Se inspeccionaron `git status`, diff y log antes de continuar. El estado heredado tenía T017.a2–T019.c3 en una sola working tree dirty; no eran commits.
- Esa implementación quedó separada en commits locales funcionales, además de los dos previos T017.a/a1:
  - `05c6d07` — modelos cerrados de herramientas/evidencia de política.
  - `a8db8a8` — PolicyService y providers cancelables.
  - `7bd60f8` — fingerprint aprobado en el contrato de procesos.
  - `f3dcbc7` — verificación de identidad antes del spawn.
  - `1497ccf` — profiles/probes app-owned y stack desde manifests.
  - `ecf7025` — checks README/gitignore/Gitleaks.
  - `d27cb5c` — contratos, fuentes oficiales y evidencia de plataforma.
  - `5969cbe` — reconciliación de checklist y evidencia local/CI.
- Cada commit tuvo test focal y mensaje aceptado por el hook local. Las capas anteriores al punto de recuperación fueron reconstruidas desde el working tree; no hay CI remota para ellas.
- T017/T018 están implementadas, verificadas localmente y comprometidas. C006 sigue abierto por límites Linux/native de T005/C005.
- T019.a1/a1b/a2, b1/b1a, b2a/b2a2/b2b/b2c y c1–c3 están implementadas. T019 parent sigue incompleta porque el host no tiene Gitleaks instalado; falta contrato de integración real con el binario exacto 8.30.1.
- T020 es la tarea activa y no depende de cerrar T019. T019 permanece incompleta solo por la integración real Gitleaks 8.30.1 no disponible en este host.
- T020.a, T020.b1 y T020.b2 están comprometidas; T020 parent permanece abierta hasta verificar T020.c.

## T019: verificación actual

- Solo se acepta Gitleaks 8.30.1, versión contrastada con el README/CLI y fixture JSON del tag. Versiones no comprobadas quedan Blocked.
- El scan usa `dir` sobre working tree con argv fijo, fingerprint del ejecutable, output JSON bounded/redacted y config temporal privada `useDefault=true`; `.gitleaks.toml` del repo no controla las reglas.
- Gitleaks también carga `.gitleaksignore` desde el source independientemente de `--config`; su presencia o fallo de inspección produce Blocked antes de spawn. History sigue Unsupported.
- RED de comportamiento: sin la guard, un fixture con `.gitleaksignore` obtenía Pass y lanzaba procesos. GREEN: `cargo test -p jameskills-infra --locked --test repo_document_checks` — 11/11 Windows. Se confirma argv materializado, config fuera del repo y eliminación posterior.
- El test usa proceso fake para verificar el contrato; no se declara integración real. `Get-Command gitleaks` no encontró ejecutable instalado.

## Toolchain/fuentes revisadas

- Host: Windows MSVC; `rustc 1.95.0 (59807616e 2026-04-14)`, Cargo 1.95.0.
- `rust-toolchain.toml` fija 1.95.0; Cargo workspace usa edition 2024 y `rust-version=1.95.0`.
- `cargo tree` confirma gpui-kit/base/component/assets 0.7.0 y snapshots `gpui-pre`/`gpui-pre-platform` 0.3.7. `gpui` no es el nombre package ID que se selecciona con `cargo tree -p`.
- Se revisaron GPUI Kit installation actual, tag/README v0.7.0 y Cargo 1.95. La instalación lista 0.7.0 y Rust 1.92+; README del tag conserva ejemplo `gpui-kit = "0.6"`. Prevalecen el pin exacto `=0.7.0` y la evidencia del workspace: `cold_path` requiere Rust 1.95.
- Cargo 1.95 define `--locked` como rechazo de cambios a la resolución; sigue siendo necesario ejecutar fmt, Clippy, tests y builds explícitamente.
- Detalle y fuentes actualizados en `docs/SOURCES.md` y `docs/PLATFORM-EVIDENCE.md`.

## Estado de ejecución reconstruido

- T020.b2 y T020.c.a están comprometidas; T020.c.b.a y T020.c.b.b.a tienen sus commits separados.
- T020.c.b.b.b.a terminó en cinco archivos y commit separado `4ffa1e6 feat(policy-engine): run approved Cargo test suites`; el hook local rechazó el primer intento porque una línea del body superó 100 columnas y el commit corregido pasó.
- GREEN local del slice: `cargo test -p jameskills-infra --locked --test test_suite_checks` 4/4 (un opt-in ignored); `cargo test -p jameskills-infra --locked` suite completa; `cargo test -p jameskills-core --locked`; `cargo clippy -p jameskills-infra -p jameskills-core --all-targets --locked -- -D warnings`; `cargo fmt --all -- --check`; `git diff --check`; `scripts/test-commitlint.sh` con Git Bash; `npm exec --no -- commitlint --from main --to HEAD --verbose` pasan. `cargo fmt --all` corrigió el formato observado durante el repaso.
- Prueba de integración real opt-in ejecutada explícitamente: `cargo test -p jameskills-infra --locked --test test_suite_checks real_cargo_test_driver_passes_and_fails_from_approved_fixtures -- --ignored --exact` falla porque el fixture de pass obtiene Cargo exit 101 compilando en este host Windows MSVC. No cuenta como pass real. El test fake sí confirma que inspección usa ReadOnlyCheck y que un HEAD alterado después de metadata detiene el spawn mutante.
- CI remota: `gh pr status` reporta que no hay PR asociada a `feat/t020-commit-test-checks`; ningún run puede atribuirse a este cambio. No confundir los GREEN locales con CI remota.
- Revisión de fuentes: Cargo 1.95 metadata v1 tiene `workspace_members`, `packages[].targets[].test`; `cargo test --workspace --locked` ejecuta targets del workspace con el lock sin resolver versiones distintas. La fuente oficial GPUI Kit installation actual lista Kit 0.7.0/Rust 1.92+; el tag v0.7.0 confirma GPUI snapshot exacto 0.3.7. El pin y baseline local continúan justificados por compilación/requisitos del grafo, no por extrapolación de docs GPUI.

## Estado verificado T020.c.b.b.b.a–b

- Runner + servicio comprometidos en `4ffa1e6 feat(policy-engine): run approved Cargo test suites` (5 archivos); fuentes/contrato Cargo comprometidos separadamente en `94d9c59 docs(policy-engine): cite Cargo suite metadata contract` (4 archivos).
- Cargo 1.95 metadata format 1 y test docs revisados desde URLs versionadas. La metadata local `--no-deps --format-version 1 --locked --offline` mostró miembros/targets `test=true` para los cuatro packages reales; no ejecutó las suites.
- GREEN local acumulado tras el runner: test suite checks 4/4 fake, suite infra y core completas, Clippy infra/core `-D warnings`, fmt, diff-check, self-test de commitlint y rango `main..HEAD` Commitlint verde.
- Opt-in real Cargo se ejecutó explícitamente pero falló: el fixture que debía pasar terminó con `exit 101`. `where.exe link.exe` no encuentra linker en el PATH del shell. `vswhere` sí encontró Visual Studio 2022 Build Tools y `VsDevCmd.bat`; aún no se probó usando ese entorno. No atribuir el 101 definitivamente a un error de dominio ni declarar integración real aprobada.
- T020.c.b.b queda abierta: falta ejecución Cargo real que demuestre el driver con fixtures pass/fail. T020.c.c no se declara elegible hasta resolver/verificar este gate de subtareas secuenciales.
- CI remota: branch sin tracking remoto, sin PR asociada y `gh run list --branch feat/t020-commit-test-checks` vacío. No hay verificación CI para estos commits.

## Próxima acción exacta

1. Verificar el fixture real dentro de un `VsDevCmd` oficial ya instalado sin cambiar la máquina; documentar solo resultados observados. Revisar además variables MSVC que el ProcessPort permite, sin pasar entorno arbitrario.
2. Si el entorno approved-env es insuficiente, crear un slice <=5 archivos para ampliar su allowlist de forma mínima y probada; no relajar a herencia libre. Repetir el opt-in Cargo pass/fail; mantener T020.c.b.b abierto hasta tener evidencia real verde.
3. CI remota aún no existe para la rama. Hace falta autorización expresa para publicar la rama/abrir PR; no confundir los commits locales validados por Commitlint con CI remota.
4. Cuando b.b.b cierre, continuar el siguiente slice productivo T020.c.c (driver Node) según el DAG; C006/C005 y T019 conservan sus bloqueos independientes.

## T020.b2.a completado

- Git verificado al iniciar: rama `feat/t020-commit-test-checks`, HEAD `0342d39`, working tree limpia.
- RED de comportamiento: `cargo test -p jameskills-infra --locked --lib process_rejects_modified_approved_script_before_spawning_runtime` falla porque el proceso se ejecutó a pesar de que el fingerprint del entrypoint no coincidía. Un intento previo con `unwrap_err` no compiló porque `ProcessOutput` no implementa `Debug`; no cuenta como RED.
- GREEN: `cargo test -p jameskills-infra --locked --lib process_rejects_modified_approved_script_before_spawning_runtime` pasa 1/1; `cargo clippy -p jameskills-infra --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check` y `git diff --check` pasan.
- Cambios locales del slice: tipo `ApprovedScript`, fingerprint asociado en `ProcessSpec` y validación del archivo canónico regular en `SystemProcessPort` justo antes del spawn.
- `ProcessSpec` protege el entrypoint aprobado; la confianza del paquete y sus dependencias debe resolverse por separado en el driver Commitlint. No afirmar integración real.
- Commit: `945167c feat(process): fingerprint approved script entrypoints`.

## T020.b2.b1 completado

- La fuente oficial `@commitlint/cli` tag v21.2.2 declara en su `package.json` `engines.node >=22.12.0`; Node release schedule registra v24 LTS hasta 2028-04-30.
- RED: `cargo test -p jameskills-infra --locked --test tool_detection node_profile_covers_commitlint_supported_node_24_runtime` falla porque el profile Node existente excluye v24 (`<23`).
- Cambio propuesto: ampliar solo el rango de detección genérico a `>=18,<25`; el driver Commitlint comprobará aparte el mínimo oficial v22.12.0. No se autoriza Node 25 sin revisar el nuevo major.
- GREEN: `cargo test -p jameskills-infra --locked --test tool_detection` pasa 13/13; Clippy infra `-D warnings`, fmt y diff check pasan. Node 24 LTS ahora está dentro del rango genérico y Node 25 continúa fuera.
- Commit: `d2a859f build(policy-engine): recognize Node 24 for Commitlint`.

## T020.b2.b2.a completado

- RED de comportamiento: el test de PolicyService devolvía Unknown porque no existía el dispatch `conventional-commit`.
- GREEN fake: `cargo test -p jameskills-infra --locked --test commit_test_checks` pasa 6 tests y deja la integración real explícitamente ignored.
- Integración real Windows: el host tiene Node 24.18.0 y `@commitlint/cli`/`@commitlint/config-conventional` 21.2.2 instalados desde lock. Ejecutar explícitamente `cargo test -p jameskills-infra --locked --test commit_test_checks real_node_commitlint_package_passes_through_repository_policy_provider -- --ignored --exact`: 1/1 Pass LocalCheck usando SystemProcessPort y no ejecutando `.cmd`.
- Hallazgo corregido por evidencia: `--edit` de Commitlint falla desde cwd externo al repo. El proceso mantiene cwd privado y pasa `--cwd` con root aprobado; `--config` es JSON absoluto privado y `load-config.ts` de la fuente tag selecciona carga explícita (sin búsqueda cosmiconfig ascendente). Rutas Windows `\\?\` se normalizan en argv Node para que el runtime reconozca el entrypoint y staging privados.
- Suite completa `cargo test -p jameskills-infra --locked`, Clippy `-D warnings`, fmt y diff check pasan. Un warning Clippy `nonminimal_bool` se corrigió y la verificación se repitió.
- Commit: `ae19691 feat(policy-engine): dispatch Commitlint through approved Node`.
- Citas fuente: `docs/SOURCES.md` vincula `load-config.ts` y `get-edit-commit.ts`, tag v21.2.2, y registra el motivo técnico del `--cwd` de Git más `--config` absoluto.
- Commit documental: `c9127cb docs(policy-engine): cite Commitlint cwd behavior`.
- T020.b2 cerrada; T020.c queda pendiente. La integración real comprueba el paquete/entrypoint instalado en este host, no la supply chain completa de dependencias ni CI remota.

## T020.c.a completado

- RED: `cargo test -p jameskills-infra --locked --test process_execution process_runner_executes_only_when_explicit_mutation_permission_is_present` falla con `ExplicitMutation` porque SystemProcessPort rechazaba todos los permisos no ReadOnlyCheck.
- GREEN: el mismo test pasa 1/1 y confirma ejecución argv-only del test harness aprobado, limitando salida y timeout; `cargo test -p jameskills-infra --locked` completo, Clippy `-D warnings`, fmt y diff check pasan.
- c.a habilita solo la semántica de permiso low-level. No se añadió ruta desde inspección, manifiestos, UI o CLI; el siguiente c.b debe exigir aprobación tipada/trust, fijar HEAD/manifests y construir driver Cargo app-owned antes de usarlo.
- Commit: `992160a feat(process): honor explicit mutation permission`.

## T020.c.b.b.a completado

- RED: `cargo test -p jameskills-core --locked --test policy_evaluation suite_run_result_rejects_exit_and_execution_status_mismatches` falla porque el modelo inicial aceptaba exit 1 como Passed.
- GREEN: `cargo test -p jameskills-core --locked --test policy_evaluation suite_run_result` pasa 2/2; la declaración de suite permanece independiente del resultado de ejecución y exit code.
- RED aprobación: `cargo test -p jameskills-core --locked --lib repository_head_rejects_unreviewed_or_malformed_identifiers` falla si el parser acepta mayúsculas/longitud inválida; el constructor validado restaura la distinción de trust.
- Approval DTO: `TestSuiteRunApproval` no deriva Deserialize, liga root/suite/head/hash de manifiestos/OperationId y no ejecuta procesos ni declara un resultado. El servicio/runner real se añade junto con el driver Cargo; `PolicyService.check` no se conecta a suites.
- GREEN full: `cargo test -p jameskills-core --locked`, Clippy core `-D warnings`, fmt y diff check pasan.
- Commit: `3949ed3 feat(policy-engine): add explicit test suite approval`.

## Preservación y lecturas

Preservar `target/` y `JameSkills-implementation-dossier.zip` locales sin seguimiento. Revisar `AGENTS.md`, T020/C006 en `tasks/todo.md`, `docs/CONTRACTS.md`, `docs/SECURITY.md`, el spec pertinente y `.github/workflows/ci.yml` antes del siguiente incremento.
