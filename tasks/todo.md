Warning: truncated output (original token count: 49353)
Total output lines: 2907

# Checklist ejecutable: JameSkills v1

**Estado inicial:** 84 tareas principales y 28 checkpoints pendientes. Subtareas obligatorias mantienen <=5 archivos por incremento. No hay implementación completada. Leer `tasks/plan.md` y `docs/CONTRACTS.md` antes de ejecutar.

## Índice y DAG

| ID | Tarea | Módulo | Depende de |
|---|---|---|---|
| [T001](#t001) | Fijar GPUI Kit, toolchain y evidencia de plataformas | `desktop-app` | — |
| [T002](#t002) | Crear workspace de dominio e infraestructura | `desktop-app` | T001 |
| [T003](#t003) | Añadir targets desktop y CLI con lockfile | `desktop-app` | T002 |
| [T004](#t004) | Crear diagnóstico e instrucciones de entorno por OS | `desktop-app` | T003 |
| [T005](#t005) | Abrir ventana real con componentes e iconos GPUI Kit | `desktop-app` | T003, T004 |
| [T006](#t006) | Establecer CI mínima reproducible | `desktop-app` | T003, T004 |
| [T007](#t007) | Definir errores, servicios y puertos comunes | `desktop-app` | T002, T003 |
| [T008](#t008) | Construir shell, rutas y bridge de UI | `desktop-app` | T005, T007 |
| [T009](#t009) | Exponer parser CLI y contrato de salida | `desktop-app` | T003, T007 |
| [T010](#t010) | Modelar y parsear manifest de suite portable | `skill-format` | T007 |
| [T011](#t011) | Modelar políticas y requisitos de herramientas | `skill-format` | T010 |
| [T012.a](#t012-a) | Validar inventario portable sin IO | `skill-format` | T010, T011 |
| [T012](#t012) | Validar árboles e importaciones con límites portables | `skill-format` | T010, T011, T004 |
| [T013](#t013) | Canonicalizar bundle y calcular hash de contenido | `skill-format` | T010, T011, T012 |
| [T014](#t014) | Crear codec seguro para import/export portable | `skill-format` | T012, T013 |
| [T015](#t015) | Añadir suite de ingeniería y validate CLI real | `skill-format` | T009, T011, T014, T037 |
| [T016](#t016) | Obtener hechos locales de un repositorio | `policy-engine` | T012, T007 |
| [T017](#t017) | Detectar herramientas y perfiles de entorno | `policy-engine` | T016, T004 |
| [T018](#t018) | Implementar evaluación de políticas y evidencia | `policy-engine` | T011, T016, T017 |
| [T019](#t019) | Comprobar README, gitignore y secretos locales | `policy-engine` | T018, T012 |
| [T020](#t020) | Comprobar Conventional Commits y pruebas declaradas | `policy-engine` | T018, T016, T017 |
| [T021](#t021) | Validar definición de CI y checks requeridos | `policy-engine` | T018, T020 |
| [T022](#t022) | Obtener auth y evidencia de GitHub con mínimos permisos | `policy-engine` | T018, T016 |
| [T023](#t023) | Validar rulesets, PR y estado CI de main | `policy-engine` | T021, T022 |
| [T024](#t024) | Validar versiones, tags y releases | `policy-engine` | T018, T022 |
| [T025](#t025) | Generar guía dinámica desde hechos y checks | `policy-engine` | T017, T018, T023, T024, T037 |
| [T026](#t026) | Ejecutar acciones registradas y doctor | `policy-engine` | T009, T025, T016 |
| [T027](#t027) | Aplicar templates y hooks locales con preview | `policy-engine` | T014, T025, T026 |
| [T028](#t028) | Conectar check CLI para uso en CI | `policy-engine` | T009, T018, T019, T020, T021, T023, T024, T025, T039, T042 |
| [T029](#t029) | Definir perfiles, capacidades y registry de agentes | `agent-adapters` | T010, T012, T016 |
| [T030](#t030) | Implementar perfil Codex documentado | `agent-adapters` | T029, T017 |
| [T031](#t031) | Implementar perfil OpenCode documentado | `agent-adapters` | T029, T017 |
| [T032](#t032) | Implementar perfil Pi con override de agent dir | `agent-adapters` | T029, T017 |
| [T033](#t033) | Implementar Antigravity CLI mediante plugin vendor | `agent-adapters` | T029, T017 |
| [T034](#t034) | Implementar Grok Build CLI y GROK_HOME | `agent-adapters` | T029, T017 |
| [T035](#t035) | Planear y aplicar instalaciones con journal | `agent-adapters` | T012, T013, T029, T030, T031, T032, T033, T034, T037, T038 |
| [T036](#t036) | Actualizar y retirar instalaciones propias con seguridad | `agent-adapters` | T035, T009 |
| [T037](#t037) | Crear SQLite y migraciones transaccionales | `skill-library` | T007, T010, T013 |
| [T038](#t038) | Persistir blobs y revisiones inmutables | `skill-library` | T037, T013, T012 |
| [T039](#t039) | Consultar catálogo, búsqueda y paginación | `skill-library` | T037, T038 |
| [T040](#t040) | Crear y guardar suites desde casos de uso | `skill-library` | T039, T011, T014 |
| [T041](#t041) | Gestionar assets y referencias como datos | `skill-library` | T040, T012, T038 |
| [T042](#t042) | Importar suites y resolver IDs/versiones duplicados | `skill-library` | T040, T041, T014 |
| [T043](#t043) | Exportar suites desde biblioteca y CLI | `skill-library` | T042, T009 |
| [T044](#t044) | Historial, rollback, fork y tombstones causales | `skill-library` | T038, T040, T042 |
| [T045](#t045) | Vincular repositorios y perfiles a suites | `policy-engine` | T039, T044, T025 |
| [T046](#t046) | Conectar biblioteca GPUI con catálogo real | `desktop-app` | T008, T039, T040, T042, T043, T044 |
| [T047](#t047) | Conectar editor, assets, políticas e historial | `desktop-app` | T046, T041, T044, T015 |
| [T048](#t048) | Renderizar checks y alcance de enforcement | `desktop-app` | T045, T046, T028 |
| [T049](#t049) | Integrar asistente de requisitos y previews | `desktop-app` | T048, T025, T026, T027 |
| [T050](#t050) | Integrar detección e instalación de cinco agentes | `desktop-app` | T036, T046, T049 |
| [T051](#t051) | Implementar header binario y KDF limitados | `cloud-sync` | T007, T012, T038 |
| [T052](#t052) | Cifrar y autenticar snapshots completos | `cloud-sync` | T051, T014, T038 |
| [T053](#t053) | Gestionar keyring, sesión de vault y lock | `cloud-sync` | T052, T004 |
| [T054](#t054) | Asistente de provisioning OAuth Desktop | `cloud-sync` | T025, T053 |
| [T055](#t055) | Implementar autorización OAuth PKCE loopback | `cloud-sync` | T054, T053 |
| [T056](#t056) | Persistir refresh tokens y manejar reauth/revoke | `cloud-sync` | T055, T053 |
| [T057](#t057) | Implementar cliente Drive appDataFolder | `cloud-sync` | T056, T022 |
| [T058](#t058) | Modelar snapshots, DAG y vault discovery | `cloud-sync` | T056, T051, T014, T044 |
| [T059](#t059) | Capturar snapshot consistente de biblioteca | `cloud-sync` | T058, T038, T044, T052 |
| [T060](#t060) | Subir snapshots inmutables con journal de sync | `cloud-sync` | T057, T059, T053 |
| [T061](#t061) | Descargar, unir y reconciliar snapshots | `cloud-sync` | T060, T058 |
| [T062](#t062) | Resolver conflictos mediante revisión explícita | `cloud-sync` | T061, T040 |
| [T063](#t063) | Restaurar con preview, recovery y transacción | `cloud-sync` | T061, T062, T042 |
| [T064](#t064) | Exportar backup portable, recuperar y rotar passphrase | `cloud-sync` | T063, T009, T052, T053 |
| [T065](#t065) | Programar sync durante app activa y cerrar limpio | `cloud-sync` | T060, T061, T053, T064 |
| [T066](#t066) | Integrar configuración Google, unlock y vault selection | `desktop-app` | T049, T056, T058, T065 |
| [T067](#t067) | Integrar progreso, conflictos y restauración GUI | `desktop-app` | T066, T062, T063, T064 |
| [T068](#t068) | Completar settings y health de capacidades | `desktop-app` | T050, T066, T067, T026 |
| [T069](#t069) | Cerrar concurrencia, cancelación y completions obsoletas | `desktop-app` | T047, T050, T067, T068 |
| [T070](#t070) | Completar teclado, foco, accesibilidad y tema | `desktop-app` | T069 |
| [T071](#t071) | Medir presupuestos de rendimiento y optimizar cuellos reales | `desktop-app` | T039, T069, T070 |
| [T072](#t072) | Instrumentar diagnósticos locales sin contenido privado | `desktop-app` | T065, T069, T071 |
| [T073](#t073) | Recuperar operaciones y coordinar instancias de app | `desktop-app` | T035, T037, T063, T069, T072 |
| [T074](#t074) | Ejecutar corpus adverso y revisión de invariantes | `skill-format` | T073, T052, T061, T063, T072 |
| [T075](#t075) | Probar flujos completos headless/CLI con dependencias reales | `desktop-app` | T028, T036, T043, T064, T065, T074 |
| [T076](#t076) | Validar todos los recorridos GPUI en Linux y Windows | `desktop-app` | T070, T073, T075 |
| [T077](#t077) | Completar CI, seguridad de dependencias y convenciones | `desktop-app` | T006, T074, T075, T076 |
| [T078](#t078) | Empaquetar e instalar release Linux | `desktop-app` | T071, T076, T077 |
| [T079](#t079) | Empaquetar e instalar release Windows | `desktop-app` | T071, T076, T077 |
| [T080](#t080) | Construir artifacts de release, licencias y firma | `desktop-app` | T078, T079, T077 |
| [T081](#t081) | Validar upgrades y compatibilidad de datos/exports | `skill-library` | T080, T037, T063, T064 |
| [T082](#t082) | Cerrar documentación para usuario y contribuidor | `desktop-app` | T081, T068, T072 |
| [T083](#t083) | Completar aceptación integrada de R01–R12 | `desktop-app` | T076, T080, T081, T082 |
| [T084](#t084) | Preparar entrega y registro final reproducible | `desktop-app` | T083 |

Los anchors del índice son estables. Las dependencias de la tabla y de cada ficha son la autoridad; un ID numérico no impone ejecución antes de sus proveedores. Todas las subtareas de un padre deben terminar antes de que una tarea dependiente pueda empezar.

## Reglas comunes de ejecución

- Las firmas finales CONTRACTS se construyen incrementalmente: DTOs antes de consumer, methods/provider solo al implementarlos; factory/UI solo registran lo disponible. Ningún stub success. Cada tarea sigue red → green → revisión → evidencia/commit. Los prefijos de prueba son nombres de test que el ejecutor crea primero, no targets existentes en este repositorio documental.
- `cargo test ... <prefijo>` debe ejecutar al menos una prueba; cero tests no pasa la tarea. Tests headless de desktop requieren crate lib y no prueban una ventana real.
- Aceptación se verifica completa antes de marcar tarea. Archivo/evidencia/función del dossier no significa código existente o compilado.
- Toda nueva dependencia exige fuente/pin/lock; si añadir Cargo.toml supera cinco archivos, registrar Txxx.deps antes de implementación. No esconder cambios de módulo/factory/manifest.
- Subtareas declaradas abajo se ejecutan secuencialmente; los acceptance/verification del padre se distribuyen y se comprueban nuevamente al cerrar el padre. Tras como máximo tres unidades verificadas, ejecutar también un checkpoint interno aunque el padre no haya terminado.
- Tests nuevos junto a archivos nuevos pueden ser `#[cfg(test)]` inline cuando el presupuesto lo requiera. Las firmas publicadas en CONTRACTS prevalecen sobre helpers sugeridos.
- `TrustState::Quarantined` de suite importada nunca se convierte en Reviewed por metadata interna: review explícito antes de instalar; scripts siguen inertes en JameSkills.
- No ejecutar cuenta Drive/host/CLI real por defecto en tests de CI. Contratos reales son opt-in explícitos, con perfil aislado y evidencia saneada.

## Orden de ejecución topológico concreto

T001 → T002 → T003 → T004 → T005 → T006 → T007 → T008 → T009 → T010 → T011 → T012.a → T012 → T013 → T014 → T016 → T017 → T018 → T019 → T020 → T021 → T022 → T023 → T024 → T029 → T030 → T031 → T032 → T033 → T034 → T037 → T015 → T025 → T026 → T027 → T038 → T035 → T036 → T039 → T040 → T041 → T042 → T028 → T043 → T044 → T045 → T046 → T047 → T048 → T049 → T050 → T051 → T052 → T053 → T054 → T055 → T056 → T057 → T058 → T059 → T060 → T061 → T062 → T063 → T064 → T065 → T066 → T067 → T068 → T069 → T070 → T071 → T072 → T073 → T074 → T075 → T076 → T077 → T078 → T079 → T080 → T081 → T082 → T083 → T084

Este orden respeta deps adicionales de factory SQLite, receipts y lookup UUID de CLI. T015 espera T037; T035 espera T037/T038; T028 espera T039/T042. Los IDs permanecen estables. Cada tres unidades implementadas en este orden ejecutar checkpoint acumulado y registrar RESUME; cerrar además C001–C028 cuando sus tres tareas/facts estén verificados. No esperar una agrupación numérica futura para verificar trabajo actual.

<a id="t001"></a>

## T001 — Fijar GPUI Kit, toolchain y evidencia de plataformas

- [x] **T001 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** Ninguna. **Estado:** completada.

**Implementación y funciones:** Verificar registry y source v0.7.0; registrar renderer, MSRV, licencia y dependencias por target, sin escribir app. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Antes de fijar versiones, comprobar que gpui-kit 0.7.0 no está yanked y su API coincide con el source publicado; registrar cada requisito aún no satisfecho.

**Archivos del incremento:**
- `rust-toolchain.toml`
- `docs/PLATFORM-EVIDENCE.md`
- `docs/SOURCES.md`

**Aceptación:**
- [x] Pin exacto gpui-kit =0.7.0 y baseline Rust 1.95.0; incremento desde1.92 queda justificado por source+compile.
- [x] Matriz Linux y Windows registra versión OS/target/display/GPU y prerequisitos con fuentes, incluidos estados no observados.
- [x] No mezclar gpui-kit con gpui-ui-kit; usar reexports compatibles de GPUI snapshot 0.3.7.

**Verificación:** rustup toolchain install 1.95.0 --profile minimal --component rustfmt --component clippy; `RUST_FONTCONFIG_DLOPEN=1 cargo check -p jameskills-desktop --locked`. Revisar registry/release/source oficiales de SOURCES. Build enlazado y ventana requieren prerrequisitos/runner nativo.

**Evidencia al ejecutar:** RED: Rust1.92 falla porque `gpui-pre-util 0.3.7` usa `slice::as_array`; 1.93/1.94 fallan por `std::hint::cold_path`. GREEN: Rust/Cargo1.95 instalados; fuente Rust oficial declara `cold_path` estable desde1.95; desktop `cargo check --locked` pasa con modo dlopen upstream. Kit index/tag/API/hash verificados. Debian13 container sin display/GPU; linker carece `xcb`, `xkbcommon`, `xkbcommon-x11`; Windows sin runner. Evidencia en `docs/PLATFORM-EVIDENCE.md`; commit `414be62`.

<a id="t002"></a>

## T002 — Crear workspace de dominio e infraestructura

- [x] **T002 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T001. **Estado:** completada.

**Implementación y funciones:** Definir workspace resolver y packages jameskills-core/jameskills-infra; el segundo depende del primero, nunca al revés. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados. Workspace members=["crates/*"] explícito; glob abarca solo packages realmente presentes.

**Red primero:** Primero documentar el grafo esperado; una comprobación de cargo metadata debe fallar si core depende de GPUI/SQLite/HTTP o falta un package.

**Archivos del incremento:**
- `Cargo.toml`
- `crates/jameskills-core/Cargo.toml`
- `crates/jameskills-core/src/lib.rs`
- `crates/jameskills-infra/Cargo.toml`
- `crates/jameskills-infra/src/lib.rs`

**Aceptación:**
- [x] cargo metadata reconoce ambos packages y mantiene core sin dependencias de plataforma.
- [x] Edición/MSRV/lints y nombre de crate coinciden con arquitectura.
- [x] No introducir mocks de servicios en compilación release.

**Verificación:** cargo metadata --format-version 1 --no-deps; cargo check -p jameskills-core -p jameskills-infra. Lockfile inicial todavía no obligatorio; T003 lo registra.

**Evidencia al ejecutar:** RED: `cargo metadata --format-version 1 --no-deps` falló porque no había `Cargo.toml` en `/workspace`. GREEN: metadatos con exactamente `jameskills-core` y `jameskills-infra`; core no tiene dependencias; infra depende solo de core. `cargo check -p jameskills-core -p jameskills-infra` y `cargo fmt --all -- --check` pasaron; ahora workspace fija MSRV1.95. Sin tests de comportamiento aún; grafo estructural validado. commit de implementación: `7bf2bd3`.

<a id="t003"></a>

## T003 — Añadir targets desktop y CLI con lockfile

- [x] **T003 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T002. **Estado:** completada.

**Implementación y funciones:** Registrar members mediante autodiscovery del workspace definido en T002; configurar gpui-kit =0.7.0 solo en desktop y main CLI independiente. Manifest desktop define feature test-support=["gpui-kit/test-support"] desde el inicio, según source0.7.0; no activarla en release normal. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Comprobación de manifests rechaza GPUI en CLI/core; comprobar --help básico antes de reemplazar placeholder por parser real T009.

**Archivos del incremento:**
- `crates/jameskills-desktop/Cargo.toml`
- `crates/jameskills-desktop/src/main.rs`
- `crates/jameskills-cli/Cargo.toml`
- `crates/jameskills-cli/src/main.rs`
- `Cargo.lock`

**Descomposición obligatoria y wiring adicional:**
- [x] **T003.a — Registrar members reales** (1 archivos): `Cargo.toml` ya declara `crates/*`; metadata reconoce exactamente los cuatro paquetes al aparecer, sin stubs.
- [x] **T003.b — Targets, test-support y lockfile** (5 archivos): manifests/main de desktop+CLI y `Cargo.lock`. Kit feature test-support forward; CLI sin display/GPU. Compatibilidad con source0.7.0 verificada.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [x] Cuatro packages reales; Cargo.lock fija Kit0.7.0 (checksum) y 862 crates bajo el resolver Rust1.95.
- [x] CLI arranca sin display/GPU y su --help no inicializa desktop; árbol normal es core+infra.
- [x] Desktop referencia GPUI Kit con assets Apache-2.0; `cargo check` pasa en MSRV1.95.

**Verificación:** cargo metadata --format-version 1 --no-deps; cargo check -p jameskills-core -p jameskills-infra -p jameskills-cli --locked. Si editar Cargo.toml requiere sexto archivo, separar T003.a member registration y T003.b targets+lock.

**Evidencia al ejecutar:** RED `cargo run -p jameskills-cli -- --help` fallaba porque paquete no existía; el primer CLI build corrigió el patrón de argumentos. GREEN metadata cuatro paquetes; CLI test1/1, --help, CLI release build; core/infra/CLI check+clippy; desktop check+clippy Rust1.95 con `RUST_FONTCONFIG_DLOPEN=1`; fmt clean. `cargo build` desktop no pudo linkar por `-lxcb`, `-lxkbcommon`, `-lxkbcommon-x11`; no hay ventana real. Implementación `ddd67ec`; ajuste MSRV `414be62`.

## C001 — Checkpoint tras T001–T003

- [ ] **C001 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Registry/pins, manifests y cuatro targets definidos; CLI independiente de GPU.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia parcial:** metadata cuatro packages; `cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked` (1 passed); CLI release build/--help; desktop check+clippy; `cargo fmt --all -- --check`; clippy core/infra/CLI sin warnings. Lock Kit0.7.0 y checksum inspeccionado. C001 permanece sin marcar: desktop link requiere development libs xcb/xkbcommon y no existe display/GPU; Windows/MSVC runner no disponible. Seguir T004 independiente según DAG y revalidar native gates en T005/C001.

<a id="t004"></a>

## T004 — Crear diagnóstico e instrucciones de entorno por OS

- [x] **T004 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T003. **Estado:** completada.

**Implementación y funciones:** PlatformFacts::detect, resolve_user_dirs; scripts aceptan check y print-install-plan con salida estructurada y sin sudo automático. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** platform_dirs respeta HOME/USERPROFILE/XDG_CONFIG_HOME y ausencia de vault; scripts check identifican dependency ausente antes de mostrar un plan.

**Archivos del incremento:**
- `crates/jameskills-infra/src/platform.rs`
- `crates/jameskills-infra/src/lib.rs`
- `scripts/setup-linux.sh`
- `scripts/setup-windows.ps1`
- `docs/PLATFORM-EVIDENCE.md`

**Descomposición para mantener <=5 archivos por incremento:**
- [x] **T004.a — Contrato y prueba de aislamiento de rutas** (3 archivos): `docs/CONTRACTS.md`; `crates/jameskills-infra/src/platform.rs`; `crates/jameskills-infra/src/lib.rs`. RED falló porque Linux path se devolvía sin namespace; GREEN 2/2 tests Linux/Windows. Commit `966caff`.
- [x] **T004.b — Resolver directorios y facts del host** (4 archivos): `crates/jameskills-infra/Cargo.toml`; `Cargo.lock`; `crates/jameskills-infra/src/platform.rs`; `docs/SOURCES.md`. Pin `directories=6.0.0`; `BaseDirs` usa Known Folders/XDG, nunca se concatena `$HOME`.
- Evidencia T004.b: RED detect facts devolvió Unknown aunque el fixture observó Wayland+GPU; GREEN `cargo test -p jameskills-infra --locked platform::tests` 6/6, `cargo clippy ... -D warnings`, fmt. Commit `4db2885`.
- [x] **T004.c — Doctor previo y planes de paquetes** (3 archivos): `scripts/setup-linux.sh`; `scripts/setup-windows.ps1`; `docs/PLATFORM-EVIDENCE.md`. Salidas estructuradas para check/plan, no mutación de sistema; ambos diagnósticos ejecutados en su OS, Windows con prerrequisitos faltantes documentados.
- Evidencia T004.c: `bash -n` y Linux JSON check/plan; PowerShell7.6.6 parser+plan pasan; check desde Linux reporta `unsupported`. En `DESKTOP-6PK09A2`, Windows 10 IoT Enterprise LTSC x64, el check real en PowerShell 5.1.19041.7725 detectó SDK (`pass`), Rust/toolchain/target MSVC, CMake y VS C++ (`missing`), display/GPU (`unknown`), y salió 1. `pwsh` no está instalado. Ver `docs/PLATFORM-EVIDENCE.md`.
- [x] **T004.d — Separar rutas Windows y validar host** (3 archivos): `crates/jameskills-infra/src/platform.rs`; `docs/CONTRACTS.md`; `docs/ARCHITECTURE.md`. RED: la prueba de LocalAppData compartido mostró que data/cache colisionaban bajo `JameSkills`; GREEN: rutas `JameSkills/Data` y `JameSkills/Cache`, sin aflojar validación de solapamiento. Commit `d1b9f07`.
- Evidencia T004.d: toolchain oficial instalado con aprobación del usuario; el check PowerShell5.1 detecta todos los build prereqs `pass`. Windows infra 10/10, core+infra+CLI 53/53 y `cargo build -p jameskills-desktop --target x86_64-pc-windows-msvc --locked` pasan. Display/GPU permanecen `unknown`; no se afirma smoke de ventana. Evidencia versionada en `docs/PLATFORM-EVIDENCE.md`.

Cada incremento tiene su propio test/evidencia/commit. T004 cierra con diagnóstico real Windows y build MSVC; el smoke visual se verifica por separado en T005.

**Aceptación:**
- [x] Linux y Windows resuelven directorios sin asumir rutas unix en Windows; las rutas Windows se probaron con el mismo Known Folder LocalAppData para datos y caché.
- [x] Scripts comprueban paquetes/build tools/SDK/backend de la versión fijada y muestran fuentes.
- [x] Una dependencia ausente genera diagnóstico concreto sin instalar ni cambiar defaults; las herramientas se instalaron por solicitud expresa y el recheck es verde.

**Verificación:** `cargo test -p jameskills-infra --locked`; `bash scripts/setup-linux.sh --check`; `powershell -File scripts/setup-windows.ps1 -Check`; `cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked`; build desktop Windows con target MSVC. Registrar ambas salidas saneadas.

**Evidencia al ejecutar:** `docs/PLATFORM-EVIDENCE.md` registra instalaciones, versiones, checks, 53 pruebas y build Windows verde. No se ejecutó ventana visible; display/GPU siguen `unknown` y ese gate corresponde a T005.

<a id="t005"></a>

## T005 — Abrir ventana real con componentes e iconos GPUI Kit

- [ ] **T005 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T003, …37353 tokens truncated…n checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t073"></a>

## T073 — Recuperar operaciones y coordinar instancias de app

- [ ] **T073 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T035, T037, T063, T069, T072. **Estado:** pendiente.

**Implementación y funciones:** acquire_app_instance, recover_operations, startup_health, graceful_shutdown; lock por data root y journals antes aceptar commands. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** startup_recovery dos procesos/misma library, kill durante install/save/restore/sync y crash luego commit recuperan receipts/head sin duplicar writes.

**Archivos del incremento:**
- `crates/jameskills-infra/src/composition.rs`
- `crates/jameskills-infra/src/platform.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-desktop/src/main.rs`
- `crates/jameskills-infra/tests/startup_recovery.rs`

**Aceptación:**
- [ ] Segunda instancia tiene comportamiento explícito seguro; profile distinto funciona separado.
- [ ] Mutaciones serializadas por destination/vault y recovery previa a UI active.
- [ ] Reparación visible con causa/estado; no eliminar DB/journals ni rehacer operaciones destructivas automáticamente.

**Verificación:** cargo test -p jameskills-infra --locked startup_recovery; procesos nativos reales por OS con fault flags solo tests/dev; confirmar release no expone fault injection.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t074"></a>

## T074 — Ejecutar corpus adverso y revisión de invariantes

- [ ] **T074 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T073, T052, T061, T063, T072. **Estado:** pendiente.

**Implementación y funciones:** Corpus de safe paths/KDF/header/process/JSON/snapshot; property tests IDs/hash/merge commutativo-idempotente-asociativo en dominio aplicable. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** security_regressions cada entrada maliciosa enumera rechazo/effect esperado; domain_properties detecta pérdida de revisión por permutación o clock skew.

**Archivos del incremento:**
- `tests/fixtures/archives/manifest.json`
- `tests/fixtures/snapshots/manifest.json`
- `crates/jameskills-infra/tests/security_regressions.rs`
- `crates/jameskills-core/tests/domain_properties.rs`
- `docs/SECURITY-EVIDENCE.md`

**Aceptación:**
- [ ] I01–I10 tienen tests/evidence mapped; SSRF/tool injection/archive bomb/secrets/tamper cubiertos.
- [ ] Revisión usa superficies reales, deps y trust boundaries; no declarar librerías auditadas por inventario.
- [ ] Regresiones conservan efectos exteriores/DB; hallazgos se corrigen en subtarea antes cerrar checkpoint.

**Verificación:** cargo test -p jameskills-infra --locked security_regressions; cargo test -p jameskills-core --locked domain_properties; cargo audit/cargo deny se instalan/pinnean en T077, no invocar tools ficticios.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t075"></a>

## T075 — Probar flujos completos headless/CLI con dependencias reales

- [ ] **T075 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T028, T036, T043, T064, T065, T074. **Estado:** pendiente.

**Implementación y funciones:** Binario real validate --path/check --skill UUID (import previo)/doctor/library/import/export/install plan/apply/remove/backup/sync; infra SQLite/FS/crypto reales, network/gh ProcessPort fakes explícitos. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** cli_e2e crate binary isolated data/profile y errores de scope; ci_guard requirement failure/Unknown strict no retorna0.

**Archivos del incremento:**
- `crates/jameskills-cli/tests/cli.rs`
- `crates/jameskills-cli/tests/ci_guard.rs`
- `crates/jameskills-infra/tests/end_to_end_library.rs`
- `tests/fixtures/skills/e2e/jameskills.toml`
- `docs/TEST-EVIDENCE.md`

**Aceptación:**
- [ ] Ciclo canonical create/edit/validate/install/export/backup/restore compara hashes/revisiones y exit codes.
- [ ] Tests aíslan user dirs/env y no escriben config/skills reales por defecto.
- [ ] Contrato fake y contrato agent/Drive real se reportan distinto; traces/artifacts no secrets.

**Verificación:** cargo test -p jameskills-cli -p jameskills-infra --locked cli_e2e; cargo test -p jameskills-cli --locked ci_guard; revisar que filtros ejecutaron tests reales.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C025 — Checkpoint tras T073–T075

- [ ] **C025 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Startup recovery, corpus adverso y CLI E2E conectan ports/infra reales.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t076"></a>

## T076 — Validar todos los recorridos GPUI en Linux y Windows

- [ ] **T076 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T070, T073, T075. **Estado:** pendiente.

**Implementación y funciones:** Harness headless view models + runbook GUI real con display/GPUI renderer; screenshots por ruta/estado y actions UI→service→infra. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Antes de registrar aceptación comprobar cada CTA/tab/modal/shortcut del GUI inventory y estados error/empty/blocked; cualquier botón sin wiring deja flow rojo.

**Archivos del incremento:**
- `crates/jameskills-desktop/tests/routing.rs`
- `crates/jameskills-desktop/tests/library_flow.rs`
- `crates/jameskills-desktop/tests/install_flow.rs`
- `crates/jameskills-desktop/tests/sync_flow.rs`
- `docs/GUI-EVIDENCE.md`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T076.a — Validar/expandir harness ya configurado** (3 archivos): `crates/jameskills-desktop/Cargo.toml`; `crates/jameskills-desktop/src/lib.rs`; `crates/jameskills-desktop/tests/routing.rs`. Feature test-support existe desde T003.b; validar forwards Kit/test-support y extender #[gpui_kit::test] pointer/keyboard real headless. No introducir feature recién al final.
- [ ] **T076.b — Matriz GUI real** (4 archivos): `crates/jameskills-desktop/tests/library_flow.rs`; `crates/jameskills-desktop/tests/install_flow.rs`; `crates/jameskills-desktop/tests/sync_flow.rs`; `docs/GUI-EVIDENCE.md`. Native display por OS, screenshots y model states sin fake success. No browser testing para GPUI.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Todos flujos GUI obligatorios ejercitados en ambos OS nativos, no solo compilados.
- [ ] Keyboard/focus/DPI/loading/cancel/error/offline/conflicts/recovery verificados con artifacts.
- [ ] Tests no afirman que un reducer test es una GUI real; integración account/agent ausente permanece pendiente.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked; cargo run -p jameskills-desktop --locked en ambos OS nativos. Matriz GUI/TESTING con screenshots/versions; headless no reemplaza display.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t077"></a>

## T077 — Completar CI, seguridad de dependencias y convenciones

- [ ] **T077 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T006, T074, T075, T076. **Estado:** pendiente.

**Implementación y funciones:** CI Linux/Windows native fmt/clippy/tests/build/coverage/security; herramientas cargo-audit/cargo-deny pin exacto y checks requeridos stable names. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Config negative test rompe gate por failing test/secreto sintético/license deny/dependency vulnerable fixture segura; pipeline no ignora salida ni tests cero.

**Archivos del incremento:**
- `.github/workflows/ci.yml`
- `.github/workflows/security.yml`
- `deny.toml`
- `scripts/check-workspace.sh`
- `docs/CI-EVIDENCE.md`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T077.a — Commit/release tooling** (5 archivos): `commitlint.config.cjs`; `package.json`; `package-lock.json`; `cliff.toml`; `.github/pull_request_template.md`. Node solo commitlint, no frontend; pin tooling, commitlint test rechaza bad subject/acepta convencional.
- [ ] **T077.b — Ownership y dependency updates** (3 archivos): `.github/CODEOWNERS`; `.github/dependabot.yml`; `THIRD-PARTY-NOTICES.md`. Revisar owners reales; ningún username ficticio; inventario licencias runtime para packages.
- [ ] **T077.c — Gates CI y supply chain** (5 archivos): `.github/workflows/ci.yml`; `.github/workflows/security.yml`; `deny.toml`; `scripts/check-workspace.sh`; `docs/CI-EVIDENCE.md`. Pin audit/deny tools y acciones; nombres stable; native Windows/Linux build/test, artifacts saneados.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Actions SHA/min permissions/cache lock/artifacts sin secretos y matrix en runners aptos.
- [ ] Conventional Commits/PR/main/release checks se conectan sin node frontend; setup tooling separado T077.a.
- [ ] GitHub required checks/protection guiados/verificados con permisos reales; no etiquetar host protegido por YAML.

**Verificación:** cargo fmt --all -- --check; cargo clippy --workspace --all-targets --locked -- -D warnings; cargo test --workspace --features jameskills-desktop/test-support --locked; cargo audit --file Cargo.lock; cargo deny --locked check. Tools pin y help oficiales, runs de workflows documentados.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t078"></a>

## T078 — Empaquetar e instalar release Linux

- [ ] **T078 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T071, T076, T077. **Estado:** pendiente.

**Implementación y funciones:** Linux release tar.gz x86_64 + .desktop/icon/installer user ~/.local/bin, runtime/glibc floor y licencias según OPERATIONS. deb/AppImage opcionales solo nueva tarea; no son requisito v1. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Primero probar paquete en máquina limpia: missing runtime lib/icon/permissions/entrypoint debe detectarse; install/remove no borra biblioteca por defecto.

**Archivos del incremento:**
- `scripts/package-linux.sh`
- `packaging/linux/jameskills.desktop`
- `packaging/linux/README.md`
- `packaging/assets/app.png`
- `docs/LINUX-PACKAGE-EVIDENCE.md`

**Aceptación:**
- [ ] Bundle nativo tar.gz contiene binarios release/assets/licenses y launcher GPUI Kit real; no promete .deb/AppImage ni todas distros.
- [ ] Install/upgrade/uninstall con data conservada y dependencias distro documentadas.
- [ ] Checksums/versión/arch y build provenance trazables; no presentar build developer como paquete probado.

**Verificación:** bash scripts/package-linux.sh --target x86_64-unknown-linux-gnu; probar artifact en Ubuntu referencia/entorno limpio con renderer, documentar distro coverage y rutas.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C026 — Checkpoint tras T076–T078

- [ ] **C026 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- GUI nativa ambos OS, full CI y paquete Linux tienen evidencia; gaps permanecen pendientes.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t079"></a>

## T079 — Empaquetar e instalar release Windows

- [ ] **T079 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T071, T076, T077. **Estado:** pendiente.

**Implementación y funciones:** Package MSI con WiX versión documentada/pinneada y target x86_64-pc-windows-msvc; icon/resources/runtime dependencies oficiales. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Primero instalar artifact en Windows limpio y comprobar launcher/paths/SDK-runtime requirements; upgrade no fuerza schema downgrade ni uninstall borra library.

**Archivos del incremento:**
- `scripts/package-windows.ps1`
- `packaging/windows/jameskills.wxs`
- `packaging/windows/README.md`
- `packaging/assets/app.ico`
- `docs/WINDOWS-PACKAGE-EVIDENCE.md`

**Aceptación:**
- [ ] Installer y binario x64 verificados nativamente; scope/permisos/shortcut/uninstall documentados.
- [ ] Firma si identidad disponible se valida; unsigned artifact local se etiqueta y no afirma firma.
- [ ] Upgrade/migración/recovery/offline funcionan con data conservada y userprofile con espacios.

**Verificación:** pwsh -File scripts/package-windows.ps1 -Target x86_64-pc-windows-msvc; usar herramientas WiX reales de versión fijada; smoke Windows y firma/checksum según OPERATIONS.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t080"></a>

## T080 — Construir artifacts de release, licencias y firma

- [ ] **T080 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T078, T079, T077. **Estado:** pendiente.

**Implementación y funciones:** Release tagged semver genera packages, checksum manifest/provenance/SBOM según soporte real, firma mediante vault CI y publicación gated por autorización. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Primero comprobar wrong tag/version/unsigned expected/signer absent/checksum mismatch impiden declarar release firmado; artifact no contiene tokens.

**Archivos del incremento:**
- `.github/workflows/release.yml`
- `THIRD-PARTY-NOTICES.md`
- `LICENSE`
- `CHANGELOG.md`
- `docs/RELEASE-EVIDENCE.md`

**Aceptación:**
- [ ] Build reproducible locked con artifacts Windows/Linux/licencias y semver consistente.
- [ ] Secrets signing solo secret manager CI permisos limitados; ninguna llave commit/log/cache.
- [ ] Sin credencial/publicación autorizada producir artifacts locales y pasos precisos, release externa queda pendiente.

**Verificación:** Ejecutar pipeline autorizado o workflow_dispatch artifact-only según permiso; verificar sha256 de todos assets y firma con herramientas oficiales de proveedor, registrar run IDs.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t081"></a>

## T081 — Validar upgrades y compatibilidad de datos/exports

- [ ] **T081 completada y verificada**

**Módulo:** `skill-library`. **Dependencias:** T080, T037, T063, T064. **Estado:** pendiente.

**Implementación y funciones:** Matriz versión app/schema/bundle/envelope; abrir DB anterior, export portable anterior, snapshot v1 y future schema safe handling. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** upgrade_compatibility nueva app carga v1/datos previos; version futura bloquea writes; backup preupgrade restaura; crypto header futuro no se interpreta como v1.

**Archivos del incremento:**
- `crates/jameskills-infra/tests/upgrade_compatibility.rs`
- `crates/jameskills-core/tests/schema_compatibility.rs`
- `tests/fixtures/skills/schema-v1/jameskills.toml`
- `docs/COMPATIBILITY.md`
- `docs/OPERATIONS.md`

**Aceptación:**
- [ ] Actualizaciones usan paquetes/versiones y migraciones comprobadas, no updater automático extra.
- [ ] Rollback de binary no downgrade destructivo DB; guía recovery verificable.
- [ ] Compatibilidad de agentes se recheck con versión nueva sin overwrite de install receipt.

**Verificación:** cargo test -p jameskills-infra --locked upgrade_compatibility; cargo test -p jameskills-core --locked schema_compatibility; upgrade artifacts instalables reales en ambos OS y recuperación por backup.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C027 — Checkpoint tras T079–T081

- [ ] **C027 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Windows MSI/release/upgrade tienen artifacts/checksums/licencias/compatibility comprobados.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t082"></a>

## T082 — Cerrar documentación para usuario y contribuidor

- [ ] **T082 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T081, T068, T072. **Estado:** pendiente.

**Implementación y funciones:** Runbooks quickstart offline, suites, policies/authority, 5 agentes, OAuth/keyring, conflicts/restore, Linux/Windows builds y release; índice dossier. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Primero ejecutar quickstart tal como lo leería usuario de perfil limpio; comando/ruta/flag inexistente deja criterio fallido y se corrige.

**Archivos del incremento:**
- `README.md`
- `CONTRIBUTING.md`
- `SECURITY.md`
- `AGENTS.md`
- `docs/OPERATIONS.md`

**Aceptación:**
- [ ] Docs tienen comandos reales, links/source y pasos de entorno revalidables; no afirmaciones absolutas de main/text enforcement.
- [ ] AGENTS define límites/tests/commits y RESUME sin exigir gates de fase contrarios al usuario.
- [ ] Security explica amenazas/recovery/passphrase/secret handling y soporte/reporting sin publicar secretos.

**Verificación:** Recorrer documentación en perfiles limpios Linux/Windows; validar enlaces y correspondencia con --help doctor/gui reales. No usar un README como evidencia de tests.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t083"></a>

## T083 — Completar aceptación integrada de R01–R12

- [ ] **T083 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T076, T080, T081, T082. **Estado:** pendiente.

**Implementación y funciones:** Matriz requisitos→tareas/tests/artifacts/manual evidence; repetir solo flows que cambios recientes invalidaron y cerrar pendientes reales. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Antes de aceptación comparar requirements/specs/invariants contra evidencia; TODO crítico o test con fake presentado como real produce fail.

**Archivos del incremento:**
- `docs/ACCEPTANCE-EVIDENCE.md`
- `docs/GUI-EVIDENCE.md`
- `docs/TEST-EVIDENCE.md`
- `docs/SECURITY-EVIDENCE.md`
- `docs/PERFORMANCE-EVIDENCE.md`

**Aceptación:**
- [ ] Cada R01–R12 cumple con evidencia fechada/version/OS y todos UI handlers son reales.
- [ ] Cinco adapters aceptados en scopes oficiales; unsupported por capability es explícito, no sustituto de adaptador omitido.
- [ ] Google dos perfiles/dispositivos restore/conflict reales y packages ambos OS; credenciales/hardware ausentes permanecen blocked.

**Verificación:** cargo fmt --all -- --check; cargo clippy --workspace --all-targets --locked -- -D warnings; cargo test --workspace --features jameskills-desktop/test-support --locked; runbooks account/agents/GUI/packaging e integración de TESTING con artifacts saneados.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t084"></a>

## T084 — Preparar entrega y registro final reproducible

- [ ] **T084 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T083. **Estado:** pendiente.

**Implementación y funciones:** Revisión final de checklist/DAG/evidence/commits, resumen release y estado de blockers; preparar artifacts y descripción de PR/release concreta. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Comparar checklist completado con Git/build/artifacts reales; inconsistencia de un checkbox/evidence obliga reabrir tarea, no retocar conclusiones.

**Archivos del incremento:**
- `tasks/RESUME.md`
- `tasks/todo.md`
- `docs/ACCEPTANCE-EVIDENCE.md`
- `docs/RELEASE-EVIDENCE.md`
- `CHANGELOG.md`

**Aceptación:**
- [ ] Entrega distingue app implementada, tests, packages, firma y publicación efectiva con enlaces/checksums.
- [ ] No hay pendientes obligatorios ni stubs críticos para declarar v1 terminada; bloquear declaración si falta evidencia OS/OAuth.
- [ ] RESUME señala siguiente acción exacta y blockers si no terminó; no borrar historial ni falsear autorización de publicar.

**Verificación:** Revisión read-only del repo/CI artifacts y matrices de aceptación; verificar git status/branch/commits y que ningún secreto está tracked. Entrega final autocontenida.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C028 — Checkpoint tras T082–T084

- [ ] **C028 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Docs/aceptación/entrega coinciden con repo y evidencia; ningún checkbox falso.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

## Registro de bloqueos del entorno

| ID / tarea | Fecha / plataforma | Prueba intentada | Error saneado | Fuente y acción siguiente | Tareas independientes |
|---|---|---|---|---|---|
| — | — | — | Ningún bloqueo comprobado aún; solo riesgos planificados | — | — |

No rellenar esta tabla con riesgos hipotéticos ni credenciales. Mantener tareas bloqueadas pendientes y actualizar RESUME.
