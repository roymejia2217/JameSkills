# Reanudación JameSkills

Fecha UTC: 2026-10-04
Rama / HEAD: `feat/t020-commit-test-checks` / `HEAD` (base de slice `caa9a23`; T020.a `1cd44fc`, T020.b1 `d304f64`).
Base: `main`=`caa9a23`, merge squash de PR #21.
PR / CI remota previa: PR #21 fusionada; CI Linux/Windows, tests, fmt, Clippy, commitlint, README Policy y Required CI finalizaron SUCCESS.

## Checkpoint T020 cerrado; T021 iniciado

- Estado comprobado 2026-10-05: rama `feat/t020-commit-test-checks`, HEAD `4d4d1ae` publicado en PR draft #22; T021.b guidance está en working tree.
- CI run `37308536795` para SHA `c8d76bb` terminó SUCCESS en fmt, Clippy, tests Linux, builds Linux/Windows, validate-commit-messages, README Policy y Required CI. PR Governance run `37308533712` también SUCCESS.
- T020.c está cerrada localmente; T020.b3.src registra fuentes oficiales y workflows live consultados vía WebFetch. Commitlint/Husky recomiendan hook local para feedback, pero documentan CI remota para enforcement; Git permite `--no-verify`, Husky `HUSKY=0`.
- El workflow observado de `nodejs/node` valida el primer mensaje del PR con versión y action fijadas; sus workflows de plataforma están separados. `microsoft/vscode` separa Linux/Windows/macOS y `rust-lang/rust` enumera OS/arquitecturas explícitamente. No se generaliza que cada proyecto aplique idénticas reglas.
- T020.b3.src committed/pushed as `849d6ed docs(policy-engine): cite local hook and CI authority`; implementación + test en `dc033f5`, corrección de fixture Unix en `c8d76bb`. No se cambió el hook gestionado.
- El primer pre-push en PowerShell plano falló por `STATUS_DLL_INIT_FAILED (0xc0000142)` sin entorno MSVC. VsDevCmd después encontró que Git Bash resolvía `C:\Program Files\Git\usr\bin\link.exe` antes que el linker MSVC. Push+gate pasaron al inicializar VS 2026 Developer environment y fijar explícitamente `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` al `link.exe` de VS; no se omitió ningún gate.
- T020.b3 completada: provider consulta `git rev-parse --git-path hooks/commit-msg` con Git aprobado/read-only, rechaza targets externos/symlinks/no-regulares/over-limit, y compara SHA-256 de dos lecturas bounded. El check agrega evidencia sin paths/hashes ni contenido y conserva `LocalCheck`; inspección no prueba argv, identidad del driver ni invocación.
- RED focal observado antes de la implementación: faltaba evidencia `repo.commit-hook`; GREEN pasó en Windows. El opt-in Node24.18/Commitlint21.2.2 con Git real verificó que el hook instalado se lee, sin ejecutarlo.
- CI run `37307758381` en `dc033f5` detectó fixture Unix sin modo ejecutable; `c8d76bb` lo corrige con permisos `0755` bajo `cfg(unix)` sin ejecutar el hook. CI `37308536795` completa confirma GREEN en Linux y Windows.
- T020 y subtareas verificadas cerradas. No se atribuye `LocalHook`; no se verificó aún una regla host que exija checks en `main` (eso corresponde a T022/T023).
- T021.src/T021.dep completadas; dependency commit local/pushed `2c6a75f`. RED observada: `cargo check -p jameskills-infra --locked --offline` rechazó lock desactualizado; GREEN offline actualizó Cargo.lock y checks locked pasaron.
- T021.a completada en `4d4d1ae`: parser bounded/inert, 21 pruebas focused + suite infra en Windows y Clippy `-D warnings`. CI run `37317369713` para SHA `4d4d1ae` pasó pruebas Linux, Clippy, fmt, builds Linux/Windows, README Policy, Commitlint y Required CI; PR Governance run `37317365655` pasó.
- El contrato solo ofrece `LocalCheck` para definición local. `ci-evidence` y reglas host no se infieren de YAML ni del status de este proceso; su proveedor remoto depende de T022/T023.
- T021.b (4 archivos) RED/GREEN: la prueba de paridad falló con `examples/repository-foundation/guidance/repository.toml` desactualizada; ambas copias ahora separan configuración del workflow, permisos/actions y evidencia host/SHA en pasos manual/recheck. `cargo test -p jameskills-core --locked --test bundle_manifest` pasa 23/23.
- Próximo: verificar bundle/fmt/Commitlint y pre-push; commit/push autorizado y CI. Siguiente sub-slice T021.a2 debe observar `git remote -v` con Git aprobado y mantener workflow GitHub como Unknown si remotes no identifican `github.com`; no inferir regla activa ni CI del SHA actual.

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

1. Repetir tests/fmt/Clippy infra local, `scripts/check-workspace.sh` en Developer environment, diff-check y Commitlint; crear un commit de corrección específico.
2. Push de la corrección al PR #22 autorizado y verificar el nuevo run CI una vez; no marcar Clippy como verde hasta verlo.
3. Después continuar T020.b3 para la evidencia LocalHook, manteniendo T020/T021 abiertas hasta pasar acceptance.
4. C006/C005 y T019 conservan sus bloqueos independientes.

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
