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

**Módulo:** `desktop-app`. **Dependencias:** T003, T004. **Estado:** pendiente.

**Implementación y funciones:** bootstrap_desktop, render_platform_probe; Application.with_assets(assets::Assets), init(cx), gpui_kit::open_window sin doble Root. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Primero abrir fixture de ventana y comprobar foco/click/icono; si faltan init/assets, la prueba visual debe mostrar el fallo concreto antes de corregirlo.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/main.rs`
- `crates/jameskills-desktop/src/theme.rs`
- `crates/jameskills-desktop/src/views/platform_probe.rs`
- `crates/jameskills-desktop/src/composition.rs`
- `docs/PLATFORM-EVIDENCE.md`

**Aceptación:**
- [ ] Ventana renderiza Button/Input/Icon del kit e inicia tema sin wrappers Root duplicados.
- [ ] Linux y Windows tienen build/smoke registrados por separado.
- [ ] Backend incompatible queda como bloqueo del target; no se sustituye por web ni se afirma éxito sin ventana.

**Verificación:** cargo build -p jameskills-desktop --locked; cargo run -p jameskills-desktop --locked en Linux y Windows con sesión gráfica. Screenshot y cierre limpio; actualizar matriz.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t006"></a>

## T006 — Establecer CI mínima reproducible

- [ ] **T006 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T003, T004. **Estado:** pendiente.

**Implementación y funciones:** Jobs de fmt, clippy/core tests y builds de targets disponibles; cache por Cargo.lock; main/PR triggers y documented commands. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Introducir fixture/config inválida que el check de fmt o metadata detecte; comprobar que job no usa continue-on-error para quality gates.

**Archivos del incremento:**
- `.github/workflows/ci.yml`
- `.gitignore`
- `.gitattributes`
- `README.md`
- `scripts/check-workspace.sh`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T006.a — Convenciones de repo y licencia** (4 archivos): `.gitignore`; `.gitattributes`; `README.md`; `LICENSE`. Añadir reglas/Apache-2.0 propia o licencia elegida en requisitos; verificar ignore y comandos.
- [ ] **T006.b — Workflow y script de checks** (2 archivos): `.github/workflows/ci.yml`; `scripts/check-workspace.sh`. Quality gates reales, sin continue-on-error ni secrets en artifacts.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] CI corre ante pull_request y push con nombres estables de checks.
- [ ] Tokens mínimos, actions por SHA revisado y secrets fuera de logs/caches/artifacts.
- [ ] Gitignore excluye tokens, llaves, vault dumps, backups temporales y outputs sin excluir código/fixtures legítimos.

**Verificación:** bash scripts/check-workspace.sh; revisar workflow contra documentación oficial GitHub Actions y ejecutar en repo autorizado cuando exista. Un YAML escrito no equivale a CI verde.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C002 — Checkpoint tras T004–T006

- [ ] **C002 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Entorno por OS, ventana GPUI Kit real y CI mínima verificadas; bloqueos target registrados.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t007"></a>

## T007 — Definir errores, servicios y puertos comunes

- [x] **T007 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T002, T003. **Estado:** completada.

**Implementación y funciones:** AppError/AppResult, ClockPort, SystemClock y build_services; registro de módulos reales según subtareas. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** common_errors distingue Validation/Conflict/CapabilityUnavailable/AuthenticationRequired/PermissionDenied/Cancelled sin exponer payload secreto.

**Archivos del incremento:**
- `crates/jameskills-core/src/error.rs`
- `crates/jameskills-core/src/lib.rs`
- `crates/jameskills-core/src/domain/mod.rs`
- `crates/jameskills-core/src/ports/mod.rs`

**Descomposición obligatoria y wiring adicional:**
- [x] **T007.dep — Dependencias tipadas y contratos ajustados** (5 archivos): `crates/jameskills-core/Cargo.toml`; `crates/jameskills-infra/Cargo.toml`; `Cargo.lock`; `docs/SOURCES.md`; `docs/CONTRACTS.md`. Pins exactos `serde`, `uuid`, `sha2`, `thiserror`, `unicode-normalization`, `chrono`; `RevisionId` y `PortablePath` no se construyen por tuple pública y sus Deserialize valida.
- Evidencia T007.dep: RED `cargo check -p jameskills-core --locked` rechazó cambios pendientes al lock; GREEN `cargo check -p jameskills-core -p jameskills-infra --locked --offline`, metadata y `cargo fmt --all -- --check` pasan. Dependencias y APIs citadas por versión. Commit `048039a`.
- [x] **T007.a — Errores e IDs públicos reales** (5 archivos): `crates/jameskills-core/src/error.rs`; `crates/jameskills-core/src/domain/ids.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/src/lib.rs`; `crates/jameskills-core/tests/common_types.rs`. AppError/Diagnostic + UUID/hash/PortablePath validados; registrar solo domain+error. Tests common_errors y invalid IDs/paths. No DTO secretos ni source serializable.
- Evidencia T007.a: RED `cargo test -p jameskills-core --locked --offline common_types` falló porque faltaban los tipos/reexports públicos. GREEN `cargo test -p jameskills-core --locked --offline` 8/8; `cargo clippy -p jameskills-core --all-targets --locked --offline -- -D warnings`; fmt. Commit `b97f744`.
- [x] **T007.b — ClockPort y registro ports** (4 archivos): `crates/jameskills-core/src/ports/clock.rs`; `crates/jameskills-core/src/ports/mod.rs`; `crates/jameskills-core/src/lib.rs`; `crates/jameskills-core/tests/clock_contract.rs`. ClockPort object-safe y fake determinista; core sin SystemClock de infraestructura.
- Evidencia T007.b: RED `cargo test -p jameskills-core --locked --offline --test clock_contract` no encontró `ports`; GREEN contrato object-safe 1/1 y suite core 9/9; Clippy all-targets `-D warnings`, fmt. Commit `f057d6a`.
- [x] **T007.c — Config/factory disponible** (4 archivos): `crates/jameskills-infra/src/composition.rs`; `crates/jameskills-infra/src/lib.rs`; `crates/jameskills-infra/tests/service_factory.rs`; `docs/CONTRACTS.md`. SystemClock y validación config/paths; factory contiene únicamente runtime facts, dirs y reloj real. No instanciar servicios sin tipos/implementación ni renderizar acciones operativas sin backend.
- Evidencia T007.c: RED `cargo test -p jameskills-infra --locked --offline --test service_factory` falló al no existir el módulo composition. GREEN suite core+infra completa 18/18 tests, incluidos 2 del factory y 1 de rutas Windows insensibles a mayúsculas; Clippy all-targets `-D warnings`, fmt. Factory no crea carpetas y requiere rutas absolutas, distintas y no solapadas. Commit `2ff5483`.

La secuencia termina en T007.c. `ApplicationServices` de dominio se declara al existir sus servicios reales; no crear `application/mod.rs` vacío en este corte. Cerrar cada subtarea con prueba/evidencia/commit. Esta descomposición contiene el presupuesto/wiring real.

**Aceptación:**
- [x] AppError Display/source quedan saneados; DTO error UI/CLI es serializable, jamás una cadena source con secretos.
- [x] ClockPort permite reloj determinista y no decide causalidad por timestamp.
- [x] Factory comparte únicamente servicios de plataforma disponibles y rechaza config inválida, sin defaults inventados.

**Verificación:** cargo test -p jameskills-core --locked common_errors; cargo check -p jameskills-infra --locked. Si añadir ApplicationServices exige módulo nuevo extra, formalizar subtarea y registrar su wiring.

**Evidencia al ejecutar:** T007.dep RED lock desactualizado con `--locked`; GREEN core/infra checks offline y versioned source pins, commit `048039a`. T007.a RED faltaban API imports; core tests 8/8, clippy `-D warnings`, commit `b97f744`. T007.b RED faltaba `ports`; contract fake object-safe 1/1 y suite acumulada, commit `f057d6a`. T007.c RED módulo composition ausente; core/infra 18/18, clippy `-D warnings`, commit `2ff5483`. Sin smoke GUI en este slice.

<a id="t008"></a>

## T008 — Construir shell, rutas y bridge de UI

- [ ] **T008 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T005, T007. **Estado:** pendiente.

**Implementación y funciones:** AppState, Route, UiCommand, UiEvent, dispatch_command, apply_event; sidebar Library/Policies/Agents/Backup/Settings y panel de estados. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** ui_bridge ignora request_id antiguo, preserva ruta seleccionada y representa loading/error/blocked/cancelled sin convertirlos en success.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/state.rs`
- `crates/jameskills-desktop/src/routes.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-desktop/src/views/shell.rs`
- `crates/jameskills-desktop/src/composition.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T008.a — Shell y rutas** (5 archivos): `crates/jameskills-desktop/src/lib.rs`; `crates/jameskills-desktop/src/views/mod.rs`; `crates/jameskills-desktop/src/views/shell.rs`; `crates/jameskills-desktop/src/routes.rs`; `crates/jameskills-desktop/src/state.rs`. Introducir lib testable y shell real; routing/empty states tests inline antes del render.
- [ ] **T008.b — Bridge y arranque conectado** (5 archivos): `crates/jameskills-desktop/src/bridge.rs`; `crates/jameskills-desktop/src/composition.rs`; `crates/jameskills-desktop/src/main.rs`; `crates/jameskills-desktop/tests/async_lifecycle.rs`; `crates/jameskills-desktop/src/lib.rs`. Reemplazar probe de T005 por shell; request IDs/events tests ui_bridge antes del dispatch. src/lib registra bridge/composition.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Navegación y estados están visibles y manejados mediante bridge, sin I/O bloqueante en render.
- [ ] Request IDs y revisión base permiten invalidar completions obsoletas.
- [ ] Fixtures de preview se aíslan con cfg(test)/feature de desarrollo y release no los usa.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked ui_bridge; cargo run -p jameskills-desktop --locked y navegar/foco/retry. Tests del state/bridge son headless, la apertura real se verifica aparte.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t009"></a>

## T009 — Exponer parser CLI y contrato de salida

- [x] **T009 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T003, T007. **Estado:** completada.

**Implementación y funciones:** Cli::parse, dispatch_cli, render_text, render_json, map_exit_code; subcommands doctor/validate/check/library/agents/install/backup/sync. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** cli_contract invoca binario real con args inválidos/--help y compara esquema JSON/exit codes; subcommands sin backend responden Unsupported, nunca success falso.

**Archivos del incremento:**
- `crates/jameskills-cli/src/main.rs`
- `crates/jameskills-cli/src/commands.rs`
- `crates/jameskills-cli/src/output.rs`
- `crates/jameskills-cli/tests/cli_contract.rs`
- `crates/jameskills-cli/Cargo.toml`

**Descomposición obligatoria:**
- [x] **T009.dep — Fijar parser y serialización JSON** (4 archivos): `crates/jameskills-cli/Cargo.toml`; `Cargo.lock`; `docs/SOURCES.md`; `tasks/todo.md`. Pin exacto Clap+derive y serde_json; registrar fuente/licencia/MSRV.
- Evidencia T009.dep: `cargo check -p jameskills-cli --offline` identificó crates ausentes del caché; tras resolución/crates.io `cargo check -p jameskills-cli` generó lock y compiló correctamente. Pins/source/licencias registrados. Commit `765abba`.
- [x] **T009.a — Parser, dispatch y salida contractual** (5 archivos): los cinco archivos listados arriba. `doctor` reporta solo facts observados; comandos sin backend devuelven Unsupported/exit3; parse errors JSON tienen wrapper redacted.
- Evidencia T009.a: RED CLI temporal produjo cinco fallas de contrato; otro RED mostró que Clap imprimía un valor inválido y se corrigió a diagnóstico genérico redacted. GREEN `cargo test -p jameskills-cli --locked --offline` (3 unit + 8 integration); Clippy combinado all-targets `-D warnings`, suite core/infra completa, `cargo run ... --help`, `doctor --json`, check flags y build release. `check` y otros comandos sin proveedor devuelven `Unsupported`, código3. Commit `ad2b662`.

**Aceptación:**
- [x] Ayuda enumera comandos/flags reales y check admite --json --strict.
- [x] JSON estable y códigos distinguen fallos de checks (1), argumentos (2), auth/capacidad (3), operaciones (4) y cancelación (130). El motor check aún no está cableado y devuelve Unsupported.
- [x] No passphrases/tokens por argumentos; parser rechaza flags secretas y oculta valores inválidos también en stderr.

**Verificación:** cargo test -p jameskills-cli --locked cli_contract; cargo run -p jameskills-cli --locked -- --help. Registrar comandos existentes y todavía no conectados.

**Evidencia al ejecutar:** Linux Debian13 x86_64. Suite final core/infra/CLI: 29 tests pasan (18 core+infra, 11 CLI); cargo clippy all-targets `-D warnings`; fmt; CLI release build. `doctor --json` da observations sin directorios; `check --strict --json` se acepta y responde capability.unsupported/3 hasta T018/T028. Sin secretos por argv.

## C003 — Checkpoint tras T007–T009

- [ ] **C003 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Puertos/errores, shell/bridge y CLI no tienen éxito ficticio ni secretos.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t010"></a>

## T010 — Modelar y parsear manifest de suite portable

- [x] **T010 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T007. **Estado:** pendiente.

**Implementación y funciones:** SkillManifest, CapabilityDeclaration, parse_manifest, parse_frontmatter y validate_skill_pair; validación de UUID/schema_version/semver y Agent Skills. SkillId ya está implementado y validado por T007.a. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** bundle_manifest rechaza schema desconocido, ID inválido, semver inválida, frontmatter ausente e incompatible; acepta fixture estándar.

**Archivos del incremento:**
- `crates/jameskills-core/src/domain/skill.rs`
- `crates/jameskills-core/src/domain/mod.rs`
- `crates/jameskills-core/tests/bundle_manifest.rs`
- `tests/fixtures/valid-suite/jameskills.toml`
- `tests/fixtures/valid-suite/SKILL.md`

**Descomposición obligatoria y wiring adicional:**
- [x] **T010.a — Reutilizar IDs validados** (0 archivos nuevos): provistos por T007.a en `crates/jameskills-core/src/domain/ids.rs` y `crates/jameskills-core/tests/common_types.rs`; SkillId UUID privado, `parse` rechaza valores arbitrarios. Evidencia: common types 8/8, commit `b97f744`.
- [x] **T010.dep — Fijar parsers y versionado** (5 archivos): `crates/jameskills-core/Cargo.toml`; `Cargo.lock`; `docs/SOURCES.md`; `docs/CONTRACTS.md`; `tasks/todo.md`. Pins exactos `toml`, `semver` serde y `serde-saphyr`; budget YAML limita profundidad, eventos y escalares, sin aliases/anchors/tags custom/merge.
- Evidencia T010.dep: `yaml-rust2` inicial tenía parser recursivo sin quota de depth; cambiada selección antes del commit de T010.b. `serde-saphyr1.3.0` ya existe en lock como transitiva; APIs/options verificadas en source y sus budgets/MSRV1.89 documentados. GREEN: `cargo check -p jameskills-core --locked --offline` compiló con el lock fijado. Commit `1a354e5` deja el primer pin; el nuevo parser queda en T010.b.
- [x] **T010.b — Manifest y frontmatter** (5 archivos): `crates/jameskills-core/src/domain/skill.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/tests/bundle_manifest.rs`; `tests/fixtures/valid-suite/jameskills.toml`; `tests/fixtures/valid-suite/SKILL.md`. Parser TOML/YAML tipado con unknown fields/duplicate keys estrictos, límites, validaciones y fixture portable. Commit `521ea0e`.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [x] Parseo preserva SKILL.md portable y metadata JameSkills separada.
- [x] Schema v1 tiene errores localizables; campos/extensiones siguen contrato, no pérdida silenciosa.
- [x] ID estable independiente del slug/directorio y límites de strings/contenido aplicados.

**Verificación:** cargo test -p jameskills-core --locked bundle_manifest; revisar fixture con docs/SPEC-skill-format.md. Core no lee disco directamente.

**Evidencia al ejecutar:** Linux Debian 13, parser core sin IO; fixture golden + negativos. `cargo test -p jameskills-core --test bundle_manifest --locked --offline`: 13/13. Acumulado `cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked --offline`: 42/42. Clippy `--all-targets -- -D warnings` y `cargo fmt --all -- --check` pasan. Commits `17cfaeb` (parser budget), `521ea0e` (implementación). Windows build y smoke GPUI nativo siguen bloqueados por ambiente sin Windows/display; no aplican al parser puro.

<a id="t011"></a>

## T011 — Modelar políticas y requisitos de herramientas

- [x] **T011 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T010. **Estado:** pendiente.

**Implementación y funciones:** PolicyDeclaration, Requirement, ToolRequirement, Scope, parse_policy; registry IDs/argumentos tipados, no shell strings. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** policy_schema rechaza requirement/tool desconocido, severity inválida, shell libre, referencias fuera de suite y versiones de schema no soportadas.

**Archivos del incremento:**
- `crates/jameskills-core/src/domain/policy.rs`
- `crates/jameskills-core/src/domain/scope.rs`
- `crates/jameskills-core/src/domain/mod.rs`
- `crates/jameskills-core/tests/policy_schema.rs`
- `tests/fixtures/valid-suite/policies/repository.toml`
- `docs/CONTRACTS.md`
- `docs/SPEC-policy-engine.md`

**Descomposición verificada:**
- [x] **T011.contract — Contrato del schema**: tipos/signatura común en `docs/CONTRACTS.md` y registro de enum/tool/check en `docs/SPEC-policy-engine.md`. Commit `2c9d60c`.
- [x] **T011.schema — Parser y fixture tipados** (5 archivos): `policy.rs`, `scope.rs`, `domain/mod.rs`, `policy_schema.rs` y fixture. Sin shell strings ni comandos importados. Commit `f32f55c`.

**Aceptación:**
- [x] Formato expresa commits/README/gitignore/secrets/main/PR/CI/tests/releases y dependencias de entorno.
- [x] Políticas se enlazan mediante IDs/referencias válidas sin ciclo de guía.
- [x] La suite describe acciones registradas, nunca código que se ejecuta al importar.

**Verificación:** cargo test -p jameskills-core --locked policy_schema; fixture representa todas las categorías v1 con ejemplos verificables, sin promesas de enforcement absoluto.

**Evidencia al ejecutar:** RED `cargo test -p jameskills-core --test policy_schema --locked --offline` falló porque `parse_policy` devolvía `policy.unsupported`; GREEN focal 5/5. Suite core/infra/CLI 47/47; Clippy all-targets `-D warnings`; fmt y `git diff --check`. Linux Debian 13, parser puro, sin ejecutar herramientas declaradas. Implementación `f32f55c`; contrato `2c9d60c`.

<a id="t012"></a>

## T012 — Validar árboles e importaciones con límites portables

- [ ] **T012 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T010, T011, T004. **Estado:** pendiente.

**Implementación y funciones:** FileSystemPort, PortablePath, inspect_bundle_tree, validate_archive_entries; límites tamaño/número/rutas y symlinks/reparse points. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** safe_bundle_paths prueba ../, absolutas, device names Windows, case collisions, symlink/junction escape, zip bomb y archivos UTF8 inválidos donde el contrato lo exige.

**Archivos del incremento:**
- `crates/jameskills-core/src/ports/filesystem.rs`
- `crates/jameskills-core/src/ports/mod.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/src/lib.rs`
- `crates/jameskills-infra/tests/safe_bundle_paths.rs`

**Aceptación:**
- [ ] Ningún input sale del staging/root ni escribe antes de validación completa.
- [ ] Colisiones portables se detectan aunque el FS local tolere diferencias de case.
- [ ] Límites se comprueban al recorrer y al descomprimir, no tras agotar memoria/disco.

**Verificación:** cargo test -p jameskills-infra --locked safe_bundle_paths en Linux y Windows con temporales; confirmar archivos exteriores intactos y no ejecución de scripts.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

**Descomposición por dependencia nativa:**
- [x] **T012.dep — Fijar case-fold Unicode vigente** (4 archivos): `crates/jameskills-core/Cargo.toml`, `Cargo.lock`, `docs/SOURCES.md` y `tasks/todo.md`. Pin exacto `icu_casemap=2.3.0`; `unicode-casefold 0.2.0` usa tablas Unicode 9.0, insuficientes para la política, se descartó. Commit `48d8ebf`.
- Evidencia T012.dep: API oficial docs.rs 2.3.0 documenta `CaseMapper::new().fold_string` como full case-fold locale independiente; se normaliza el resultado NFC. `cargo check -p jameskills-core` resolvió/descargó y compiló 2.3.0 bajo Rust 1.95; `cargo check -p jameskills-core --locked --offline` pasa. Lock contiene `icu_casemap` y `icu_casemap_data`.

<a id="t012-a"></a>

- [x] **T012.a — Inventario portable puro** (3 archivos): `crates/jameskills-core/src/domain/bundle.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/tests/portable_bundle_inventory.rs`. Depende T010/T011, nunca abre/extracta archivos. Límite de número/tamaño, entries de solo fichero regular, PortablePath y colisiones tras ICU full case-fold + NFC. RED identificó colisión por prefijo de directorio (`Docs/...`/`docs/...`), GREEN focused 5/5; commit `56a28e3`.
- [x] **T012.contract — Contrato de FileSystemPort y límites** (3 archivos): `docs/CONTRACTS.md`; `docs/SPEC-skill-format.md`; `tasks/todo.md`. Separó la validación de inventario puro de la capa OS, sin cambiar la interfaz existente del port. Commit `ed801a9`.
- [ ] **T012.b — Filesystem/ZIP real** (5 archivos): ports Filesystem, infra fs/lib, test safe_bundle_paths. Depende T012.a y T004; valida no-follow/ancestor/symlink/reparse, entry ZIP real, bomb y staging sin escritura antes de validar.

El padre T012 no se cierra hasta completar T012.a, T012.contract y T012.b en Linux y Windows reales.

## C004 — Checkpoint tras T010–T012

- [ ] **C004 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Formato/policies/safe paths rechazan inputs inválidos; no side effects fuera staging.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t013"></a>

## T013 — Canonicalizar bundle y calcular hash de contenido

- [ ] **T013 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T010, T011, T012. **Estado:** pendiente.

**Implementación y funciones:** ValidatedBundle, canonical_inventory, hash_bundle; orden de paths y hashing con representación/versionado documentado. compute_revision usa bytes/hash exactos SPEC-skill-format; parent IDs ordenados, timestamps fuera hash; goldens content/tombstone. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** bundle_hash mismo contenido produce hash idéntico tras orden distinto/timestamps; cambiar bytes, metadata relevante o path produce hash distinto. Revision hash difiere por parents/kind/observed_heads y no por created_at; CRLF diferente cambia bundle hash.

**Archivos del incremento:**
- `crates/jameskills-core/src/domain/skill.rs`
- `crates/jameskills-core/tests/support/mod.rs`
- `crates/jameskills-core/tests/bundle_hash.rs`
- `crates/jameskills-infra/src/fs.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T013.a — DTO de revisión y hash causal** (4 archivos): `crates/jameskills-core/src/domain/library.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/src/domain/skill.rs`; `crates/jameskills-core/tests/revision_hash.rs`. Definir RevisionRecord/RevisionKind/parents/ContentHash usando IDs validados; compute_revision exacto antes de StoragePort. revision_hash golden content/tombstone, timestamps irrelevantes, parent ordering.
- [ ] **T013.b — Bundle hashing canónico** (4 archivos): `crates/jameskills-core/src/domain/skill.rs`; `crates/jameskills-core/tests/support/mod.rs`; `crates/jameskills-core/tests/bundle_hash.rs`; `crates/jameskills-infra/src/fs.rs`. Hash raw bytes ordenadas/inventory; tests/support/mod.rs local a integration tests, no helper de fixture en release.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Canonicalización no normaliza arbitrariamente instrucciones ni line endings fuera del contrato.
- [ ] Hash verifica bytes de assets y manifest, excluye solo metadata explícitamente no canónica.
- [ ] Inventario lleva tamaños/hashes y se usa por library/install/backup sin algoritmos duplicados.

**Verificación:** cargo test -p jameskills-core --locked revision_hash; cargo test -p jameskills-core --locked bundle_hash; goldens causales/raw byte inventory en ambos OS.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t014"></a>

## T014 — Crear codec seguro para import/export portable

- [ ] **T014 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T012, T013. **Estado:** pendiente.

**Implementación y funciones:** read_bundle, write_bundle_archive, unpack_bundle_to_staging; archive determinista y límites, sin extracción directa sobre biblioteca. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** bundle_archive roundtrip conserva instrucciones/assets/policies y rechaza entrada duplicada, traversal, checksum mismatch y truncamiento.

**Archivos del incremento:**
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/tests/bundle_archive.rs`
- `crates/jameskills-core/src/ports/filesystem.rs`
- `crates/jameskills-infra/Cargo.toml`

**Aceptación:**
- [ ] Export portable es legible sin JameSkills y no contiene tokens, DB ni paths privados.
- [ ] Import directory/archive comparte validación y exact bytes canónicos.
- [ ] Errores borran staging y conservan fuente/destino; no follow symlinks.

**Verificación:** cargo test -p jameskills-infra --locked bundle_archive; abrir export con herramienta zip estándar y leer SKILL.md/TOML.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t015"></a>

## T015 — Añadir suite de ingeniería y validate CLI real

- [ ] **T015 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T009, T011, T014, T037. **Estado:** pendiente.

**Implementación y funciones:** validate_bundle_command; ejemplo explica Conventional Commits, README, secretos, pruebas, PR/main/CI/releases y limitaciones de evidencia. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** validate_bundle fixture inválida falla con path/código; ejemplo oficial pasa validación real usando el codec/core.

**Archivos del incremento:**
- `examples/repository-foundation/SKILL.md`
- `examples/repository-foundation/jameskills.toml`
- `examples/repository-foundation/policies/repository.toml`
- `crates/jameskills-cli/src/commands.rs`
- `crates/jameskills-cli/tests/validate_bundle.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T015.a — Ejemplo canónico mínimo** (3 archivos): `examples/repository-foundation/SKILL.md`; `examples/repository-foundation/jameskills.toml`; `examples/repository-foundation/policies/repository.toml`. Copiar/adaptar docs/examples/repository-foundation del dossier; usar UUID f9c0199f-c4ce-4b04-85dd-ae12a7db292b. No inventar política si falta fixture.
- [ ] **T015.b — Guía y referencias del ejemplo** (5 archivos): `examples/repository-foundation/guidance/repository.toml`; `examples/repository-foundation/references/standards.md`; `examples/repository-foundation/references/environment.md`; `examples/repository-foundation/templates/README.md`; `examples/repository-foundation/templates/.gitignore`. DAG tipado y material de lectura/templates; validar references y ausencia de shell/tool registration arbitrary.
- [ ] **T015.c — Template de CI y asset propio** (2 archivos): `examples/repository-foundation/templates/ci-rust.yml`; `examples/repository-foundation/assets/optional-brand.svg`. Assets seguros sin script/external href; template es dato; validación golden de suite completa.
- [ ] **T015.d — Validate service y CLI** (5 archivos): `crates/jameskills-core/src/application/library.rs`; `crates/jameskills-core/src/application/mod.rs`; `crates/jameskills-infra/src/composition.rs`; `crates/jameskills-cli/src/commands.rs`; `crates/jameskills-cli/tests/validate_bundle.rs`. LibraryService::validate_import usa FileSystemPort y domain::validate_bundle sin exigir storage aún; CLI llama service, no infra directa.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] validate conecta CLI→ApplicationServices/validador→FileSystemPort sin duplicar parseo.
- [ ] Suite portable tiene acciones/requisitos verificables y no instala tooling automáticamente.
- [ ] JSON y salida humana muestran warnings/errores concretos y límites del formato.

**Verificación:** cargo test -p jameskills-cli --locked validate_bundle; cargo run -p jameskills-cli --locked -- validate --path examples/repository-foundation.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C005 — Checkpoint tras T013–T015

