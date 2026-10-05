# Reanudación JameSkills

Fecha UTC: 2026-10-05
Rama / HEAD: `feat/t020-commit-test-checks` / `73a3f62`.
Base: `main`=`caa9a23`, merge squash de PR #21.
PR / CI remota previa: PR draft #22; el run conocido `37328599512` y Governance `37328595359` cubren `c976f02`, no los commits locales posteriores. No se ha hecho push.

## Checkpoint T020–T024 y C008 cerrado; T025 siguiente

- Estado comprobado: rama `feat/t020-commit-test-checks`, HEAD `73a3f62`, 11 commits por delante de `origin/feat/t020-commit-test-checks`; working tree limpio antes de este checkpoint. No hay push.
- T023.src `bd78191`, T023.a/b `db5df25`, T023.c `0989248`: rules efectivas/classic protection, bypass tri-state, PR/check contexts y `RequiredCi` solo con regla host activa y resultado successful del SHA local exacto; respeta `integration_id`/`app_id`.
- T023 verificada localmente: host_protection_checks 11 passed + 1 opt-in ignored; workspace tests, workspace Clippy `-D warnings`, fmt/diff-check pass. Integración GitHub CI read-only pasó 1/1, sin atribuir CI SUCCESS al SHA local.
- T024.src committed en `b4419fe` + precisión de versión en `dba65e5`; T024.a implementada en `73a3f62`. Proyecto se versiona desde Cargo workspace/package y/o Node `package.json`, no desde `SkillManifest`; releases publicadas deben tener tag SemVer coincidente; changelog, digest y tag signature solo se exigen según flags del profile.
- T024 RED `current_project_version_requires_a_published_matching_release_with_asset_digest`: `Unknown` porque no había dispatch para `ReleaseContract`; GREEN release_checks 12 passed + 1 opt-in ignored. Cubre conflicto de versiones, JSON duplicate key, semver/tag wrong, draft/prerelease, release ausente vs 403/404, lista de 100 sin paginar, SHA-256 asset digest, changelog con notas, firma annotated/tag lightweight y no requerir flags desactivados.
- T024 gates Windows MSVC: `cargo test --workspace --locked --features jameskills-desktop/test-support`, workspace Clippy con mismos features y `-D warnings`, `cargo fmt --all -- --check`, `git diff --check` pasan. Opt-in release read-only pasó 1/1; solo valida evidence/source/version bound, no exige que el checkout tenga release publicada ni marca el resultado Pass.
- C008 completado localmente: T022–T024 GitHub tests usan GET/read-only, los gates acumulados pasan, no se escribieron reglas/releases/tags. CI remoto conocido `37328599512` solo cubre `c976f02`; no atribuirlo al HEAD local.
- Próxima tarea DAG: T025 (`T017`, `T018`, `T023`, `T024`, `T037` satisfechas). Desarrollar guía dinámica; localizar firmas actuales/fixture, capturar RED y mantener el incremento en <=5 archivos.

## Historial inmediatamente anterior
- T020 completa: Cargo/npm runners con consentimiento explícito, Commitlint local y T020.b3 hook read-only. El hook del repo existe, pero el bootstrap `npm exec` no demuestra identidad/argv del entrypoint aprobado; autoridad observada se mantiene `LocalCheck`.
- T021.src, `.dep`, `.a`, `.b` y `.a2.src` completadas. CI contract usa YAML bounded/inerte, required job IDs, triggers push/PR, action refs pin, permissions y predicados conservadores; su resultado local solo es `LocalCheck`. T023 implementa por separado `CiEvidence`/`RequiredCi` de host+SHA exacto.
- T021.a2 RED/GREEN: test host mismatch fallaba si se asumía GitHub por la mera presencia de workflow; ahora exige Git native/fingerprinted, versión registrada y todos los URLs configurados de `git remote -v` en github.com. GitLab/GHES/no remote/mixed => Unknown. No hace request de red ni prueba existencia del repositorio, branch rule o CI SHA; URLs no entran en logs/evidence.
- Verificación local T021.a2: infra suite completa, 23 ci_definition_checks + un ignored test, Clippy infra `-D warnings`, fmt y diff-check pasan. Opt-in `real_git_remote_and_checked_in_workflow_produce_localcheck_only` ejecutado con Git 2.55 en el repo real y pasó 1/1.
- T022 implementación, aceptación y verificación local completas en `8d14689`; al cerrar T022, `CiEvidence`/RequiredCi seguían Unknown hasta completar T023. El último run remoto reportado sigue siendo para el SHA previo `c976f02` y no se atribuye a los commits locales posteriores.
- T022.src completada documentalmente en `8d14689`: manuales oficiales de gh auth/api, fuente upstream tag `v2.102.0`, REST `GET /repos/{owner}/{repo}`, autenticación y rate limits registrados en `docs/SOURCES.md`. JSON auth status puede salir 0 incluso fallando; se omite `--show-token`; no se registran identidades, errores crudos ni scopes; host/GET/path quedan fijos. REST 404 puede ocultar recurso privado; 403/429 se clasifican conservadoramente.
- T022.a completada: `CheckEvidence` ahora permite resumen dinámico bounded; `evidence_tests` 2/2. La RED previa no se capturó y no se inventa retrospectivamente.
- Driver/provider de T022 en `8d14689`: comprueba versión real `gh 2.102.0`; auth JSON sin `--show-token`; REST GET con host/argv/path fijos y salida bounded. `RepositoryPolicyCheckProvider` obtiene remote consistente y HEAD de Git aprobado antes de llamar a gh. Evidencia limita afirmación a repo/SHA/check/source + `repo-read`; 404 Unknown, auth/permission/rate Blocked y nunca eleva a RequiredCi. Fakes verifican auth fallida y remotes mixtos sin API.
- T022.registry.a/b/c cerradas: `github-access`, `gh repository-read`, nombre portable y loader runtime; profile general gh detecta v2 pero driver T022 solo acepta runtime exacto 2.102.0.
- T022.windows-env cerrada: RED focused falló por `APPDATA` no allowlisted; GREEN focused 1/1 al allowlistear la ruta de configuración Windows. `GH_HOST`, `GH_TOKEN`, `GITHUB_TOKEN` continúan rechazados.
- Gates finales: `cargo test --workspace --locked --features jameskills-desktop/test-support` pasó; workspace Clippy `-D warnings`, fmt y diff-check pasaron. Opt-in real GitHub read-only pasó 1/1 en este repo; comprobó Git remote/HEAD, gh auth y GET repo. No almacena token ni muestra identidad.
- T023.src committed as `bd78191 docs(policy-engine): cite GitHub protection API contracts`: fuentes oficiales establecen reglas efectivas active-only, bypass metadata posiblemente oculta, classic 404 ambiguo y checks exact-SHA con límites de 100/paginación.
- T023.a/b committed as `db5df25 feat(policy-engine): inspect active GitHub branch rules`: provider dispatches `github-branch-policy`, fixed GET effective rules + classic protection, compares PR/check contexts y registra bypass tri-state sin actor details. `host_protection_checks` pasa 4/4; parser unit bypass tri-state 1/1; Clippy infra `-D warnings`, fmt/diff-check pass. `require_no_bypass=true` permanece Unknown cuando bypass actors no son visibles, Fail cuando hay bypass, Pass HostRule solo con visibilidad completa y lista vacía.
- Al terminar T023, T024 estaba pendiente; el checkpoint actual arriba reemplaza ese estado.

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

## Corrección CI de Clippy — T020.c.c.b

- T020.c.c.b terminó en cinco archivos y commit `7cf6d6b feat(policy-engine): run approved npm quality scripts`; fuentes npm/run-script en `656a860 docs(policy-engine): cite npm script environment semantics`.
- Fuentes/tag npm CLI `v11.16.0`, docs v11 de run/config/.npmrc/folders, npm/run-script v10.0.4 y Node v24 consultados. Host probado: Node24.18.0/npm11.16.0; npm 11.16 engine `^20.17.0 || >=22.9.0`.
- La invocación Node directa al `npm-cli.js` no ejecuta `.cmd`; script ID solo `lint`/`test`/`build`; argv fija `--ignore-scripts` suprime lifecycle pre/post, pero el script pedido corre por shell del sistema bajo aprobación explícita. `.npmrc` root bloqueado; config global/usuario privada vacía; salida/cancelación/timeout acotados. La huella npm-cli.js no representa todo el árbol del paquete.
- Verificación local del code slice: suite infra completa, Clippy infra `-D warnings`, fmt y diff-check pasan. Fakes distinguen Unknown/Blocked, Missing/NotRun, versiones incompatibles, `.npmrc` bloqueado y los tres script IDs. Opt-in real `real_npm_test_driver_passes_and_fails_without_lifecycle_hooks` pasa 1/1 con Node24.18/npm11.16 vía `SystemProcessPort`: un fixture pass, uno fail, main script corre y pre/post markers no aparecen.
- Fuentes npm/run-script tag v10.0.4 revisadas para documentar herencia de `process.env`, shell configurable y command text desde package.json; esto respalda ApprovedEnv mínimo, `.npmrc` project bloqueado y aprobación explícita, sin afirmar sandbox ni aislamiento del mismo usuario.
- PR draft #22 para `feat/policy-engine: execute approved repository suites`, base `main`. CI `37266801368` en SHA `656a860`: Clippy falló por parámetro usado solo Windows; formatter, tests, Commitlint, README y builds pasaron. Fix `3ffc1c7` se subió.
- CI `37268520271` en SHA `3ffc1c7`: tests, Commitlint, fmt, README, build Linux/Windows y PR Governance SUCCESS; Clippy volvió a fallar en `test_suite_checks.rs:204` por `unused_mut` del helper de entorno Windows cfg en Linux.
- Corrección local actual: `node_process_environment()` ahora declara un binding independiente bajo `cfg(windows)` y `cfg(not(windows))`, quitando el `mut` visible en Unix. Falta commit/push y nuevo run Linux.
- T020.c está funcionalmente lista localmente; la PR sigue con Clippy remoto rojo. T020.b3 (autoridad LocalHook) sigue pendiente; T020/T021 permanecen abiertas.

## Próxima acción exacta

1. Leer T025, `docs/CONTRACTS.md`, `docs/SPEC-policy-engine.md`, y comprobar implementación/wiring actual de guidance antes de editar.
2. Escribir la primera prueba de comportamiento ausente en `guidance_planner`; dividir T025 explícitamente si el wiring supera cinco archivos.
3. Implementar hechos/evidencia/status/staleness sin guía “success” sin comprobación; ejecutar tests focales, workspace gates, registrar evidencia y commit.

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
