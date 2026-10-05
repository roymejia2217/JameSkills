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

## Checkpoint T020.c.b cerrado

- Commits en orden: `4ffa1e6` runner/provider Cargo; `94d9c59` contrato/fuentes Cargo; `7e876d0` checkpoint; `92ae96b` allowlist de rutas MSVC; `ef6b82d` fuentes Microsoft Learn; `b0a0404` integración real pass/fail.
- La allowlist de `ApprovedEnv` conserva exclusivamente rutas/toolchain MSVC (`INCLUDE`, `LIB`, `LIBPATH`, VS/SDK paths); `CL` y `_CL_` se rechazan porque permiten inyectar opciones.
- GREEN integración real: el commit `b0a0404 test(policy-engine): verify Cargo runner with MSVC` incluye en su slice de tres archivos el `real_environment` con variables VS/SDK aprobadas y la prueba de tool missing. Desde `VsDevCmd` x64, `cargo test -p jameskills-infra --locked --test test_suite_checks real_cargo_test_driver_passes_and_fails_from_approved_fixtures -- --ignored --exact` pasa 1/1 mediante `SystemProcessPort`; ejecuta y comprueba un fixture pass y otro fail.
- GREEN acumulado: `cargo test -p jameskills-infra --locked` suite completa; Clippy infra `-D warnings`; fmt, diff-check y Commitlint local pasan. El fake distingue Cargo ausente (Unknown/Blocked), suite ausente (Missing/NotRun) y fallo (Failed/exit).
- T020.c.b queda cerrada localmente. Próxima unidad elegible T020.c.c: driver Node/npm de suites. T020.c parent permanece abierta hasta implementarla y verificarla.
- CI remota sigue ausente: no hay upstream, PR ni runs asociados a `feat/t020-commit-test-checks`. Los resultados son locales; hace falta autorización expresa antes de push/PR para lanzar CI.

## Slice actual T020.c.c.a — contrato/fuentes npm

- Revisadas fuentes npm CLI `v11.16.0`, npm CLI docs v11 de run/config/.npmrc/folders y Node v24 docs. npm 11.16 declara Node `^20.17.0 || >=22.9.0`; host observado Node24.18.0/npm11.16.0 y probe vía `node npm-cli.js --version` dio `11.16.0`.
- npm ejecuta el texto del script por `/bin/sh` o `cmd.exe`; `--ignore-scripts` mantiene el evento pedido y suprime pre/post. El contrato lo define como ejecución consentida de código del repo, no sandbox.
- El diseño bloquea `.npmrc` del root y aísla config global/usuario. La huella de npm-cli.js no verifica todos los módulos relativos del paquete.

## Próxima acción exacta

1. Cerrar .c.c.a con fmt/diff-check y Commitlint local en su commit documental.
2. Implementar .c.c.b (Node/npm runner + tests) en máximo cuatro archivos; empezar por test que exija ejecutar solo el script solicitado y nunca npm.cmd, pre/post hooks ni repository `.npmrc`.
3. Ejecutar opt-in real con Node24.18/npm11.16 en fixtures pass/fail y fake suite; revalidar manifest/HEAD y aislamiento de npm config antes del spawn.
4. Solicitar autorización específica para publicar la rama/abrir PR y obtener CI remota; no reportar verdes locales como CI remota.
5. C006/C005 y T019 conservan sus bloqueos independientes.

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
