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

**Evidencia parcial:** metadata cuatro packages; suite core/infra/CLI 53/53 en Windows MSVC; desktop Windows `cargo build -p jameskills-desktop --target x86_64-pc-windows-msvc --locked` pasó; fmt y Clippy/check Linux previos registrados arriba. Lock Kit0.7.0 y checksum inspeccionado. C001 permanece sin marcar: el build/link Linux sigue requiriendo development libs `xcb`, `xkbcommon`, `xkbcommon-x11`; en Windows display/GPU siguen `unknown` y no hubo smoke de ventana/renderer. T004 ya está completo; revalidar los native gates en T005/C001.

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

- [x] **T006 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T003, T004. **Estado:** completada.

**Implementación y funciones:** Jobs de fmt, clippy/core tests y builds de targets disponibles; cache por Cargo.lock; main/PR triggers y documented commands. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Introducir fixture/config inválida que el check de fmt o metadata detecte; comprobar que job no usa continue-on-error para quality gates.

**Archivos del incremento:**
- `.github/workflows/ci.yml`
- `.gitignore`
- `.gitattributes`
- `README.md`
- `scripts/check-workspace.sh`

**Descomposición obligatoria y wiring adicional:**
- [x] **T006.a — Convenciones de repo y licencia** (4 archivos): `.gitignore`; `.gitattributes`; `README.md`; `LICENSE`. Añadir reglas/Apache-2.0 propia o licencia elegida en requisitos; verificar ignore y comandos.
- Evidencia T006.a: RED `git ls-files --error-unmatch .gitignore .gitattributes LICENSE` sin coincidencias. GREEN `LICENSE` Apache-2.0 (texto apache.org, titular roymejia2217 pendiente de validación del owner antes de publicar), `text=auto`+LF en shell, ignore con target/secretos/respaldos/salidas sin tocar fixtures, README con convenciones. `ls-files` coincide, `check-ignore` cubre `target/` y `.env`, `diff --check` limpio. Commit `9f6f8ed`.
- [x] **T006.b — Workflow y script de checks** (2 archivos): `.github/workflows/ci.yml`; `scripts/check-workspace.sh`. Quality gates reales, sin continue-on-error ni secrets en artifacts.
- Evidencia T006.b: RED ambos ausentes. GREEN ci.yml con actions fijados por SHA, jobs fmt/clippy/tests/builds linux+windows sin continue-on-error y cache por Cargo.lock; `bash scripts/check-workspace.sh` código 0. Remoto: PR #2 8/8, PR #3 y PR #4 9/9, runs push en main verificados. Commits `903153d`, `628cb8c` (fix fontconfig), `931a356` (fix aggregator).

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [x] CI corre ante pull_request y push con nombres estables de checks.
- [x] Tokens mínimos, actions por SHA revisado y secrets fuera de logs/caches/artifacts.
- [x] Gitignore excluye tokens, llaves, vault dumps, backups temporales y outputs sin excluir código/fixtures legítimos.

**Verificación:** bash scripts/check-workspace.sh; revisar workflow contra documentación oficial GitHub Actions y ejecutar en repo autorizado cuando exista. Un YAML escrito no equivale a CI verde.

**Evidencia al ejecutar:** T006.a y T006.b verificadas en Windows local y remoto (PR #2 8/8, PR #3 y #4 9/9, push runs en main). `git diff --check` limpio.

## C002 — Checkpoint tras T004–T006

- [ ] **C002 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Entorno por OS, ventana GPUI Kit real y CI mínima verificadas; bloqueos target registrados.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia parcial:** Windows `cargo test --workspace --features jameskills-desktop/test-support --locked` pasó; workspace Clippy `-D warnings`, fmt y diff check pasaron. CLI 16 tests, desktop headless 16, core 77, infra 82 passed/3 ignored. C006 sigue sin marcar por smoke nativo display/GPU/ventana pendiente en T005; continuar T019 independiente.

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

**Evidencia parcial:** Windows `cargo test --workspace --features jameskills-desktop/test-support --locked` pasó; workspace Clippy `-D warnings`, fmt y diff check pasaron. CLI 16, desktop headless 16, core 77, infra 82 passed/3 ignored. C006 sigue sin marcar por smoke nativo display/GPU/ventana pendiente en T005; se continúa T019 independiente.

<a id="t010"></a>

## T010 — Modelar y parsear manifest de suite portable

- [x] **T010 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T007. **Estado:** completada.

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

**Módulo:** `skill-format`. **Dependencias:** T010. **Estado:** completada.

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

- [x] **T012 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T010, T011, T004. **Estado:** completada.

**Implementación y funciones:** FileSystemPort, PortablePath, inspect_bundle_tree, validate_archive_entries; límites tamaño/número/rutas y symlinks/reparse points. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** safe_bundle_paths prueba ../, absolutas, device names Windows, case collisions, symlink/junction escape, zip bomb y archivos UTF8 inválidos donde el contrato lo exige.

**Archivos del incremento:**
- `crates/jameskills-core/src/ports/filesystem.rs`
- `crates/jameskills-core/src/ports/mod.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/src/lib.rs`
- `crates/jameskills-infra/tests/safe_bundle_paths.rs`

**Aceptación:**
- [x] Ningún input sale del staging/root ni escribe antes de validación completa.
- [x] Colisiones portables se detectan aunque el FS local tolere diferencias de case.
- [x] Límites se comprueban al recorrer y al descomprimir, no tras agotar memoria/disco.

**Verificación:** cargo test -p jameskills-infra --locked safe_bundle_paths en Linux y Windows con temporales; confirmar archivos exteriores intactos y no ejecución de scripts.

**Evidencia al ejecutar:** RED imports sin resolver en safe_bundle_paths. GREEN 10/10 Windows local y suite remota ubuntu en PR #4 9/9. Symlink escape con assert real en Linux; en Windows sin privilegios queda UNSUPPORTED visible y el resto del slice verifica igual. Sin ZIP crate nueva: parser central a mano. Commit `2c04106` (squash PR #4).

**Descomposición por dependencia nativa:**
- [x] **T012.dep — Fijar case-fold Unicode vigente** (4 archivos): `crates/jameskills-core/Cargo.toml`, `Cargo.lock`, `docs/SOURCES.md` y `tasks/todo.md`. Pin exacto `icu_casemap=2.3.0`; `unicode-casefold 0.2.0` usa tablas Unicode 9.0, insuficientes para la política, se descartó. Commit `48d8ebf`.
- Evidencia T012.dep: API oficial docs.rs 2.3.0 documenta `CaseMapper::new().fold_string` como full case-fold locale independiente; se normaliza el resultado NFC. `cargo check -p jameskills-core` resolvió/descargó y compiló 2.3.0 bajo Rust 1.95; `cargo check -p jameskills-core --locked --offline` pasa. Lock contiene `icu_casemap` y `icu_casemap_data`.

<a id="t012-a"></a>

- [x] **T012.a — Inventario portable puro** (3 archivos): `crates/jameskills-core/src/domain/bundle.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/tests/portable_bundle_inventory.rs`. Depende T010/T011, nunca abre/extracta archivos. Límite de número/tamaño, entries de solo fichero regular, PortablePath y colisiones tras ICU full case-fold + NFC. RED identificó colisión por prefijo de directorio (`Docs/...`/`docs/...`), GREEN focused 5/5; commit `56a28e3`.
- [x] **T012.contract — Contrato de FileSystemPort y límites** (3 archivos): `docs/CONTRACTS.md`; `docs/SPEC-skill-format.md`; `tasks/todo.md`. Separó la validación de inventario puro de la capa OS, sin cambiar la interfaz existente del port. Commit `ed801a9`.
- [x] **T012.b — Filesystem/ZIP real** (5 archivos): ports Filesystem, infra fs/lib, test safe_bundle_paths. Depende T012.a y T004; valida no-follow/ancestor/symlink/reparse, entry ZIP real, bomb y staging sin escritura antes de validar.

El padre T012 no se cierra hasta completar T012.a, T012.contract y T012.b en Linux y Windows reales.

## C004 — Checkpoint tras T010–T012

- [ ] **C004 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Formato/policies/safe paths rechazan inputs inválidos; no side effects fuera staging.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia 2026-10-04:** Windows host: `cargo test --workspace --features jameskills-desktop/test-support --locked` pasó; `cargo clippy --workspace --all-targets --features jameskills-desktop/test-support --locked -- -D warnings`, fmt/diff check y `cargo build -p jameskills-desktop --target x86_64-pc-windows-msvc --locked` pasaron. No hay PR/runs remotos y Linux no se ejecutó aquí. C006 permanece sin marcar por los requisitos nativos pendientes de T005/C005; no se infiere estado Linux/GPU.

<a id="t013"></a>

## T013 — Canonicalizar bundle y calcular hash de contenido

- [x] **T013 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T010, T011, T012. **Estado:** completada.

**Implementación y funciones:** ValidatedBundle, canonical_inventory, hash_bundle; orden de paths y hashing con representación/versionado documentado. compute_revision usa bytes/hash exactos SPEC-skill-format; parent IDs ordenados, timestamps fuera hash; goldens content/tombstone. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** bundle_hash mismo contenido produce hash idéntico tras orden distinto/timestamps; cambiar bytes, metadata relevante o path produce hash distinto. Revision hash difiere por parents/kind/observed_heads y no por created_at; CRLF diferente cambia bundle hash.

**Archivos del incremento:**
- `crates/jameskills-core/src/domain/skill.rs`
- `crates/jameskills-core/tests/support/mod.rs`
- `crates/jameskills-core/tests/bundle_hash.rs`
- `crates/jameskills-infra/src/fs.rs`

**Descomposición obligatoria y wiring adicional:**
- [x] **T013.a — DTO de revisión y hash causal** (3 archivos): `crates/jameskills-core/src/domain/library.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/tests/revision_hash.rs`. Definir RevisionRecord/RevisionKind/parents/ContentHash usando IDs validados; compute_revision exacto antes de StoragePort. revision_hash golden content/tombstone, timestamps irrelevantes, parent ordering.
- Evidencia T013.a: RED E0432 imports sin resolver. GREEN 8/8 Windows local (`--test revision_hash`), suite core completa, clippy all-targets `-D warnings`, fmt/diff limpios; PR #7 9/9 remoto (tests ubuntu + builds). Kind bytes propios Content 0x01/Tombstone 0x02 documentados; `skill.rs` sin cambios (versión pasa como `&str` validado semver). Commit `9cf9a71` (squash PR #7).
- [x] **T013.b — Bundle hashing canónico** (5 archivos): `crates/jameskills-core/src/domain/skill.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/tests/support/mod.rs`; `crates/jameskills-core/tests/bundle_hash.rs`; `crates/jameskills-infra/src/fs.rs`. Hash raw bytes ordenadas/inventory; tests/support/mod.rs local a integration tests, no helper de fixture en release.
- Evidencia T013.b: RED E0432 en `canonical_inventory`/`hash_bundle`. GREEN 7/7 Windows local más 3 unit tests de `read_bundle_bytes` en fs; suites core/infra completas, clippy workspace `-D warnings`, fmt/diff limpios; PR #9 9/9 remoto. El quinto archivo es el reexport en `mod.rs`; `skill.rs` no toca parsers. Commit `27325af` (squash PR #9).

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [x] Canonicalización no normaliza arbitrariamente instrucciones ni line endings fuera del contrato.
- [x] Hash verifica bytes de assets y manifest, excluye solo metadata explícitamente no canónica.
- [x] Inventario lleva tamaños/hashes y se usa por library/install/backup sin algoritmos duplicados.

**Verificación:** cargo test -p jameskills-core --locked revision_hash; cargo test -p jameskills-core --locked bundle_hash; goldens causales/raw byte inventory en ambos OS.

**Evidencia al ejecutar:** T013.a 8/8 y T013.b 7/7 Windows local con goldens independientes del digest; remoto PR #7 y PR #9 9/9 (tests ubuntu + builds). CRLF cambia bundle hash, orden no; tamaños fuera del digest. Commits `9cf9a71`, `27325af`.

<a id="t014"></a>

## T014 — Crear codec seguro para import/export portable

- [x] **T014 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T012, T013. **Estado:** completada.

**Implementación y funciones:** read_bundle, write_bundle_archive, unpack_bundle_to_staging; archive determinista y límites, sin extracción directa sobre biblioteca. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** bundle_archive roundtrip conserva instrucciones/assets/policies y rechaza entrada duplicada, traversal, checksum mismatch y truncamiento.

**Archivos del incremento:**
- `crates/jameskills-core/src/ports/filesystem.rs`
- `crates/jameskills-core/src/ports/mod.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/tests/bundle_archive.rs`

**Aceptación:**
- [x] Export portable es legible sin JameSkills y no contiene tokens, DB ni paths privados.
- [x] Import directory/archive comparte validación y exact bytes canónicos.
- [x] Errores borran staging y conservan fuente/destino; no follow symlinks.

**Verificación:** cargo test -p jameskills-infra --locked bundle_archive; abrir export con herramienta zip estándar y leer SKILL.md/TOML.

**Evidencia al ejecutar:** RED E0432 en las tres funciones del codec. GREEN 11/11 Windows local (roundtrip con igualdad de hash, exports byte-idénticos, traversal/absoluta/duplicada/device rechazadas, mismatch/truncamiento sin pánico, staging con limpieza y rechazo de destino existente). Manual: motor ZIP de Windows extrajo bytes exactos con CRLF intacto. Remoto PR #11 9/9. Writer stored-only sin dependencias nuevas (`infra/Cargo.toml` intacto); deflated valida inventario pero no se extrae hasta inflate verificado; staging Unix 0700/0600. Commit `4d4179f` (squash PR #11).

<a id="t015"></a>

## T015 — Añadir suite de ingeniería y validate CLI real

- [x] **T015 completada y verificada**

**Módulo:** `skill-format`. **Dependencias:** T009, T011, T014, T037. **Estado:** completada.

**Implementación y funciones:** `domain::validate_bundle`, `LibraryService::validate_import`, `FileSystemPort::read_bundle_directory`, comando `validate`. El ejemplo explica Conventional Commits, README, secretos, pruebas, PR/main/CI/releases y límites de evidencia. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** validate_bundle fixture inválida falla con path/código; ejemplo oficial pasa validación real usando el codec/core.

**Archivos del incremento:** ver los incrementos desglosados a continuación; cada uno mantiene un máximo de cinco archivos.

**Descomposición obligatoria y wiring adicional:**
- [x] **T015.a — Ejemplo canónico mínimo** (completada con a1+a2): copia compatible con parsers reales T010/T011 y test de los tres archivos del runtime fixture. UUID `f9c0199f-c4ce-4b04-85dd-ae12a7db292b`; no inventar política si falta fixture.
- [x] **T015.a1 — Adaptar fuentes y fijar RED/GREEN de parser** (4 archivos): `docs/examples/repository-foundation/SKILL.md`; `docs/examples/repository-foundation/policies/repository.toml`; `crates/jameskills-core/tests/bundle_manifest.rs`; `tasks/todo.md`. Corrige solo desajustes del ejemplo fuente con contratos/parsers ya implementados; el test demuestra tanto el parse como la ausencia del destino runtime antes de copiar.
- Evidencia T015.a1: RED source example: `frontmatter.unknown_field` por `license` no soportado, luego `policy.invalid` por keys/checks fuera de T010/T011. RED runtime path: test falla en tiempo de ejecución porque falta `examples/repository-foundation/jameskills.toml`. GREEN parser del ejemplo fuente `official_repository_example_passes_manifest_skill_and_policy_parsers` 1/1; `policy_schema` 5/5.
- [x] **T015.a2 — Copiar fixture canónico runtime** (5 archivos): `examples/repository-foundation/SKILL.md`; `examples/repository-foundation/jameskills.toml`; `examples/repository-foundation/policies/repository.toml`; `tasks/todo.md`; `tasks/RESUME.md`. El test existente verifica manifest, frontmatter+par y policy desde esa ruta.
- Evidencia T015.a2: RED `portable_repository_example_is_available_at_the_runtime_fixture_path` falló porque no existía el manifest en `examples/`; GREEN focused 1/1, `bundle_manifest` 15/15 y core completo 49/49 Windows. `cargo clippy -p jameskills-core --all-targets --locked -- -D warnings`, fmt y diff check pasan. Fuente oficial conservada bajo `docs/examples`; el runtime copia solo los tres archivos planificados.
- [x] **T015.b — Guía y referencias del ejemplo** (completada con b1+b2+b3): guidance ligada a requirements existentes, referencias y templates inertes con rutas consistentes.
- [x] **T015.b1 — Alinear guidance con policy y actions registradas** (5 archivos): `docs/examples/repository-foundation/guidance/repository.toml`; `examples/repository-foundation/guidance/repository.toml`; `crates/jameskills-core/tests/bundle_manifest.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Prueba verifica requisitos, references, DAG de steps y acciones registry-only; eliminar plan cuya requirement no existe.
- Evidencia T015.b1: RED `runtime_guidance_references_known_requirements_and_registered_actions` por guidance runtime ausente. GREEN focused 1/1; `bundle_manifest` 16/16 y core 50/50 Windows; `policy_schema` 5/5; core Clippy, fmt y diff check limpios. La guía coincide semánticamente con la fuente; todas las plan/step refs pertenecen a policy; DAG acíclico; acciones se limitan a manual-instruction/recheck o URLs registradas. Se eliminó solo `github-access-setup`, que referenciaba una requirement no soportada; tool operations validadas por `parse_policy`.
- [x] **T015.b2 — Copiar referencias y README template** (completada con b2a+b2b).
- [x] **T015.b2a — Testear referencias/template ausentes** (3 archivos): `crates/jameskills-core/tests/bundle_manifest.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Fixture-test exige destinos presentes y contenido alineado con las fuentes documentales.
- Evidencia T015.b2a RED: `runtime_reference_material_matches_documented_sources` compila y falla en runtime porque falta `references/standards.md`; test normaliza CRLF/LF para comparar material de texto sin cambiar los bytes fuente.
- [x] **T015.b2b — Copiar referencias y README template** (5 archivos): `examples/repository-foundation/references/standards.md`; `examples/repository-foundation/references/environment.md`; `examples/repository-foundation/templates/README.md`; `tasks/todo.md`; `tasks/RESUME.md`. Preservar advertencias sobre Unsupported/Unknown y no afirmar gates remotos.
- Evidencia T015.b2: RED por destino ausente; al copiar, una discrepancia de texto `Conventional Commits1.0` vs `Conventional Commits 1.0` detectó copia no fiel. GREEN `runtime_reference_material_matches_documented_sources` 1/1 tras alinear; compara texto sin alterar line endings y cubre dos referencias + README template. Core 51/51 Windows; core Clippy `-D warnings`, fmt y diff check verdes.
- [x] **T015.b3 — Copiar template gitignore inerte** (5 archivos): `examples/repository-foundation/templates/.gitignore`; `examples/repository-foundation/SKILL.md`; `crates/jameskills-core/tests/bundle_manifest.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Mantener lockfiles/fixtures y exclusiones secretas; nunca sobreescribir gitignore de usuario.
- Evidencia T015.b3: RED el link SKILL aún apuntaba `templates/gitignore.txt`; GREEN test `runtime_skill_links_to_the_user_safe_gitignore_template` 1/1: link corregido, template copiado fiel y exclusiones/lockfile conservados. Core 52/52 Windows; core Clippy, fmt y diff check verdes.
- [x] **T015.c — Template de CI y asset propio** (5 archivos): `examples/repository-foundation/templates/ci-rust.yml`; `examples/repository-foundation/assets/optional-brand.svg`; `crates/jameskills-core/tests/bundle_manifest.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Asset propio sin script/event/foreignObject/external href; template sigue fail-closed e inerte.
- Evidencia T015.c: RED `runtime_ci_template_is_fail_closed_and_svg_asset_has_no_active_content` por template runtime ausente. GREEN 1/1; `bundle_manifest` 19/19 y core 53/53 Windows; CI template igual a fuente, conserva `run: exit 1` placeholder; SVG own/static sin script/event/foreignObject/href/image. Core Clippy, fmt y diff check verdes.
- [x] **T015.d — Validate service y CLI** (completada con d1–d5): domain validator, FileSystemPort, LibraryService, composition real y CLI JSON/text sin success stubs.
- [x] **T015.d1 — Validación pura de bundle** (5 archivos): `crates/jameskills-core/src/domain/skill.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/tests/bundle_manifest.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Validar manifest/frontmatter/policy/guidance/resources sobre `BundleFiles` y producir hash canónico/summary.
- Evidencia T015.d1: RED E0432 por `validate_bundle` ausente; GREEN core 56/56 Windows, con bundle oficial hash canónico, policy resource ausente y action guidance no registrada rechazadas. Clippy core `-D warnings`, fmt y diff check verdes. La validación semántica actual solo admite fuentes/action IDs del contrato registry.
- [x] **T015.d1a — Unir guidance de varios archivos referenciados** (4 archivos): `crates/jameskills-core/src/domain/skill.rs`; `crates/jameskills-core/tests/bundle_manifest.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Los manifiestos permiten múltiples guidance_files; validar referencias contra su unión global sin exigir que cada archivo replique todos los planes.
- Evidencia T015.d1a: RED `bundle_validation_combines_guidance_plans_from_multiple_manifest_files` detecta rechazo falso cuando los planes están repartidos entre dos archivos. GREEN core 59/59 Windows, incluida cobertura de split; core Clippy `-D warnings`, fmt y diff check verdes.
- [x] **T015.d2 — Port filesystem y LibraryService** (4 archivos): `crates/jameskills-core/src/ports/filesystem.rs`; `crates/jameskills-core/src/lib.rs`; `crates/jameskills-core/src/application/library.rs`; `crates/jameskills-core/src/application/mod.rs`. Servicio inyectable y sin SQLite/filesystem directo en core; tests inline en `application/library.rs`.
- [x] **T015.d2e — Registrar evidencia del service slice** (2 archivos): `tasks/todo.md`; `tasks/RESUME.md`. Cerrar d2 tras su test RED/GREEN y focused check.
- Evidencia T015.d2: tests `validation_uses_the_injected_filesystem_and_returns_domain_summary` y `validation_preserves_filesystem_diagnostics` 2/2; core 58/58 Windows; core Clippy `-D warnings`, fmt y diff check verdes. El service no depende de storage ni lee filesystem directamente.
- [x] **T015.d3 — Adaptador filesystem y factory** (5 archivos): `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/src/composition.rs`; `crates/jameskills-infra/tests/library_validation.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Implementar lectura segura existente y wiring del provider real; no crear carpetas de usuario al validar.
- Evidencia T015.d3: RED E0599 porque `RuntimeServices::library` no existía. GREEN `cargo test -p jameskills-infra --locked` 61/61 Windows; composición valida el fixture runtime por walk+bytes reales sin crear config/data/cache. Workspace Clippy `-D warnings`, fmt y diff check verdes.
- [x] **T015.d4 — Comando validate JSON/text real** (5 archivos): `crates/jameskills-cli/src/commands.rs`; `crates/jameskills-cli/src/main.rs`; `crates/jameskills-cli/src/output.rs`; `crates/jameskills-cli/tests/validate_bundle.rs`; `tasks/todo.md`. Exit/error conserva diagnostics con paths/codes y no echoa contenido externo.
- Evidencia T015.d4: RED `validate_bundle` oficial devolvía Unsupported/exit 3; fixture inválida también no exponía path/code. GREEN CLI focused 2/2; core 58/58, infra 61/61, CLI 16/16 Windows. JSON éxito incluye slug/version/count/hash/warnings; error y texto reportan path/code/mensaje relativo sin reflejar contenido. Workspace Clippy `-D warnings`, fmt y diff check verdes.
- [x] **T015.d5 — Cierre documental y checkpoint** (2 archivos): `tasks/todo.md`; `tasks/RESUME.md`. Capturar evidencia local/remota y próximo DAG; no marcar T015 padre hasta test CLI/fixture y CI verde.
- Evidencia d5: Windows local core 59/59, infra 61/61, CLI 16/16 y desktop test-support 16/16; workspace Clippy `-D warnings`, fmt y diff check verdes. `cargo run` JSON/human valida la suite oficial (10 archivos, SHA-256 `63ca5ff22016cc1cbc5936a7bdcc0588a863204ce5cdaf906a5a46844341a661`); inválido informa `jameskills.toml` / `manifest.invalid` sin echo. PR #19 Required CI, Linux/Windows build, tests, clippy, fmt, commitlint, README Policy y PR Governance 9/9.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [x] `validate` conecta CLI → RuntimeServices/LibraryService → domain validator + FileSystemPort sin duplicar parseo.
- [x] Suite portable tiene acciones/requisitos verificables y no instala tooling automáticamente.
- [x] JSON y salida humana muestran warnings/errores concretos y límites del formato.

**Verificación:** cargo test -p jameskills-cli --locked validate_bundle; cargo run -p jameskills-cli --locked -- validate --path examples/repository-foundation.

**Evidencia al ejecutar:** RED CLI Unsupported exit 3, d1 API ausente E0432, factory missing `library()` E0599. GREEN Windows: core 59/59, infra 61/61, CLI 16/16, desktop test-support 16/16; workspace Clippy/fmt/diff clean. PR #19 CI Linux/Windows 9/9.

## C005 — Checkpoint tras T013–T015

- [ ] **C005 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Hash y .jskill roundtrip; suite oficial y validate CLI real funcionan.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia parcial:** tras T013–T015 pasan core/infra/CLI suites, desktop test-support, workspace checks, hash/codec y validate CLI; PR #19 CI 9/9. C005 permanece sin marcar: T005 carece del smoke de ventana visible/captura y display/GPU observados; bloqueo documentado en `docs/PLATFORM-EVIDENCE.md`. Continuar una tarea independiente si T005 sigue bloqueada.

<a id="t016"></a>

## T016 — Obtener hechos locales de un repositorio

- [x] **T016 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T012, T007. **Estado:** completada.

**Implementación y funciones:** ProcessSpec, ApprovedExecutable, ApprovedEnv, ProcessPort, RepositoryFacts, collect_repository_facts; Git executable resuelto/aprobado, cwd seguro, argv fijo y timeout. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** repository_facts path con espacios/metacaracteres no se interpreta como shell; repo inexistente, detached HEAD, worktree y submodule tienen estados explícitos.

**Archivos por incremento:** desglosados en T016.dep, T016.a–c; máximo cinco archivos en cada hijo.

**Descomposición obligatoria:**
- [x] **T016.dep — Pin de process runtime y evidencia upstream** (5 archivos): `crates/jameskills-core/Cargo.toml`; `crates/jameskills-infra/Cargo.toml`; `Cargo.lock`; `docs/SOURCES.md`; `tasks/todo.md`. Pins exactos de async-trait, Tokio runtime y `command-group`; verificar versión/licencia/MSRV y process groups/job objects.
- Evidencia T016.dep: RED `cargo check -p jameskills-core -p jameskills-infra --locked` requirió actualizar lock. GREEN `cargo check ...` resolvió/compiló async-trait 0.1.92 (MSRV1.71), Tokio 1.53.1 (MSRV1.71), command-group 5.0.1 (MSRV1.68, nix 0.27.1); crates.io yanked=false/licencias fijadas en `docs/SOURCES.md`; Windows MSVC compiló command-group/Job Object.
- [x] **T016.dep.e — Registrar evidencia de dependencias y reanudación** (2 archivos): `tasks/todo.md`; `tasks/RESUME.md`.
- Evidencia T016.dep: RED locked check requirió actualizar `Cargo.lock`; GREEN `cargo check -p jameskills-core -p jameskills-infra --locked` pasó Windows MSVC. Crates.io/docs.rs fuentes y MSRV/licencias registradas; CI procesa Linux/Windows en PR.
- [x] **T016.a — Contratos de proceso aprobados** (5 archivos): `crates/jameskills-core/src/ports/process.rs`; `crates/jameskills-core/src/ports/mod.rs`; `crates/jameskills-core/tests/process_contract.rs`; `tasks/todo.md`; `tasks/RESUME.md`. DTOs privados/validated, ProcessPort object-safe async, CancellationToken, argv/env/output budgets.
- Evidencia T016.a: RED E0432 porque `ports::process` y tipos aprobados no existían. GREEN `process_contract` 3/3; core 62/62 Windows, core Clippy `-D warnings`, fmt y diff check verdes. Rechaza env key `XAI_API_KEY`, rutas relativas, límites fuera de rango; argv conserva espacios/metacaracteres como valores separados.
- [x] **T016.b — Runner de proceso con límite y cancelación de grupo** (5 archivos): `crates/jameskills-infra/src/process.rs`; `crates/jameskills-infra/src/lib.rs`; `crates/jameskills-infra/tests/process_execution.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Usar group_spawn de `command-group`, stdout/stderr drenados concurrentes con cap, timeout/cancel terminan el grupo en Unix/Windows.
- Evidencia T016.b: RED E0432 porque no existían `SystemProcessPort`/módulo infra process. GREEN `process_execution` 3/3 Windows: stdout+stderr simultáneos, overflow mata grupo, timeout/cancel mata child group. Infra 64/64 y core 62/62; workspace Clippy `-D warnings`, fmt y diff check verdes.
- [x] **T016.c — Capturar RepositoryFacts por Git readonly** (5 archivos): `crates/jameskills-core/src/ports/process.rs`; `crates/jameskills-infra/src/process.rs`; `crates/jameskills-infra/tests/repository_facts.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Sólo argv internos fijos; estados normal/detached/worktree/submodule; no hooks/fetch/push.
- Evidencia T016.c: RED E0432 por collect_repository_facts/RepositoryState ausentes. GREEN repository_facts 3/3 Windows: ProcessPort fake verifica argv fijos con ruta `spaces; $(...)`, no-repo explícito; Git real temporal verifica attached/detached/linked-worktree/submodule. Core 62/62, infra 67/67; workspace Clippy `-D warnings`, fmt y diff check verdes; commands de inspección solo rev-parse/symbolic-ref/version y hooks path deshabilitados en setup fixture.
- [x] **T016.d — Checkpoint y documentación local** (2 archivos): `tasks/todo.md`; `tasks/RESUME.md`. Dejar evidencia local, PR remoto pendiente, T005 blocker y próxima tarea independiente.
- Evidencia T016.d: deps/process/facts y suites Windows verificadas; PR #20 CI Linux/Windows 9/9. C005 permanece abierto porque T005 no tiene smoke nativo demostrado.

**Aceptación:**
- [x] Hechos Git/paths/versiones se obtienen sin comandos arbitrarios de políticas.
- [x] Process output/tamaño/timeout/cancel se acotan y secretos se redactan.
- [x] No ejecutar hooks ni fetch/push al inspeccionar; lectura conserva worktree.

**Verificación:** cargo test -p jameskills-infra --locked repository_facts con repos temporales y ProcessPort fake; comprobar argv y kill/cancel de proceso hijo en ambos OS.

**Evidencia al ejecutar:** RED E0432 por `collect_repository_facts`/`RepositoryState` ausentes. GREEN Windows: core 62/62, infra 67/67, incluyendo argv fake/real Git, timeout/cancel, output cap, attached/detached/worktree/submodule y not-a-repository; workspace Clippy `-D warnings`, fmt y diff check. PR #20 CI Linux/Windows 9/9.

<a id="t017"></a>

## T017 — Detectar herramientas y perfiles de entorno

- [x] **T017 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T016, T004. **Estado:** completada localmente; evidencia Windows, Linux CI pendiente.

**Implementación y funciones:** ToolRegistry, DriverVersionSpec, EnvironmentFacts, detect_tools; profiles/tools.toml solo de app: Git, cargo, npm/node, gh, Gitleaks, Commitlint, cargo-audit/deny con source/pin/args/schema/exit semantics. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** tool_detection executable ausente/incompatible, version output no parseable y user-supplied shell string producen Missing/Unknown/Unsupported.

**Archivos del incremento:**
- `crates/jameskills-core/src/domain/guidance.rs`
- `crates/jameskills-core/src/domain/mod.rs`
- `crates/jameskills-infra/src/process.rs`
- `crates/jameskills-infra/src/platform.rs`
- `crates/jameskills-infra/tests/tool_detection.rs`

**Descomposición obligatoria y wiring adicional:**
- [x] **T017.a — Facts/capacidades de herramientas**: tipos puros, estados distintos, enum registry ampliado y consumidores alineados.
- [x] **T017.a1 — Modelar tool observations puros** (5 archivos): `crates/jameskills-core/src/domain/guidance.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/tests/tool_capabilities.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Missing/Blocked/Unknown/Candidate/Verified, compatibilidad SemVer, capabilities y Evidence app-owned.
- Evidencia T017.a1: RED E0432 porque `domain::guidance::ToolDetection` no existía. GREEN `tool_capabilities` 3/3; core 65/65 Windows; core Clippy `-D warnings`, fmt/diff clean. Candidate compatible no pasa capability hasta verificación; summary es estático y source ID/timestamp bounded.
- [x] **T017.a2 — Ampliar IDs/operations cerradas en el policy registry** (5 archivos): `crates/jameskills-core/src/domain/policy.rs`; `crates/jameskills-core/tests/policy_schema.rs`; `tests/fixtures/valid-suite/policies/repository.toml`; `tasks/todo.md`; `tasks/RESUME.md`. Registrar Node/Rustc/cargo-audit/cargo-deny y pares permitidos; exigir operation `version` al check toolchain y probar pares cruzados inválidos.
- Evidencia T017.a2: RED focused `policy_schema` con E0599 por ToolId/ToolOperation ausentes; GREEN `cargo test -p jameskills-core --locked --test policy_schema` 6/6. Fixture incluye `cargo/version`; rechazo de `node/scan-tracked` confirma allowlist de pares.
- [x] **T017.a3 — Alinear consumidores de los nuevos registry IDs** (4 archivos): `crates/jameskills-core/src/domain/skill.rs`; `crates/jameskills-infra/src/process.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Mapear IDs/operaciones estables en guía/diagnósticos sin tratar cargo-audit/deny como shell.
- Evidencia T017.a3: RED E0004 al compilar consumidores exhaustivos tras ampliar enums; GREEN `cargo test -p jameskills-core --locked` 66/66 y `cargo test -p jameskills-infra --locked` 67 passed, 3 ignored; Clippy core+infra `-D warnings` pasó.
- [x] **T017.b — Registry, probes y guías oficiales** (b1+b2+b3+b4+b5): perfiles app-owned, discovery conservador, argv fijo, fingerprint y fuentes por plataforma.
- [x] **T017.b1 — Definir y parsear perfiles de tools** (5 archivos): `profiles/tools.toml`; `crates/jameskills-infra/Cargo.toml`; `Cargo.lock`; `crates/jameskills-infra/src/platform.rs`; `crates/jameskills-infra/tests/tool_detection.rs`. Schema cerrado, parsers/rangos semver y argv literal app-owned.
- Evidencia T017.b1: RED `cargo test -p jameskills-infra --locked --test tool_detection` E0432 por falta del loader; GREEN integración 1/1 y parser unitario 1/1. Schema rechaza campos desconocidos y combinaciones herramienta/operación inválidas.
- [x] **T017.b2 — Descubrir candidatos y parsear versiones** (5 archivos): `profiles/tools.toml`; `crates/jameskills-infra/src/platform.rs`; `crates/jameskills-infra/tests/tool_detection.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Resolver candidatos sin ejecutarlos; outputs no reconocidos permanecen Unknown.
- Evidencia T017.b2: RED `tool_detection` E0432 por falta de candidate/version parsers; GREEN `cargo test -p jameskills-infra --locked --test tool_detection` 4/4 y Clippy infra `-D warnings`. PATH produce solo candidatos (nunca ejecución), rutas relativas se omiten, shims `.cmd` quedan tipados aparte y outputs desconocidos/oversized no parsean.
- [x] **T017.b3 — Vincular aprobación de ejecutable con fingerprint** (4 archivos): `crates/jameskills-core/src/ports/process.rs`; `crates/jameskills-core/tests/process_contract.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Un probe ejecutable debe declarar SHA-256 observado/aprobado; paths sin fingerprint no autorizan probes.
- Evidencia T017.b3: RED `process_contract` E0432/E0599 al faltar el tipo/builder de fingerprint; GREEN `cargo test -p jameskills-core --locked --test process_contract` 4/4. `ProcessSpec::new` no incluye identidad aprobada por defecto.
- [x] **T017.b4 — Ejecutar probes registrados** (b4a+b4b): fingerprint de ejecutable y ProcessPort con argv/environment/budgets fijos.
- [x] **T017.b4a — Verificar fingerprint al ejecutar procesos** (4 archivos): `crates/jameskills-infra/Cargo.toml`; `Cargo.lock`; `crates/jameskills-infra/src/process.rs`; `crates/jameskills-infra/tests/process_execution.rs`. SHA-256 streaming limitado; cambio/no match bloquea antes de spawn.
- Evidencia T017.b4a: RED al retirar el guard, test de ejecutable reemplazado no obtuvo `process.executable.identity_changed`; GREEN process_execution 5 passed/3 ignored, process_contract 4/4 y Clippy core+infra `-D warnings`. SHA-256 streaming <=512 MiB; mismatch bloquea antes del spawn.
- [x] **T017.b4b — Probar perfiles con aprobación de fingerprint** (5 archivos): `crates/jameskills-infra/src/platform.rs`; `crates/jameskills-infra/tests/tool_detection.rs`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. Missing/Unknown/Incompatible/shim Blocked; fixed argv; solo native candidate con digest aprobado ejecuta ProcessPort.
- Evidencia T017.b4b: RED `tool_detection` E0432 por falta de `probe_registered_tool_version`; GREEN detección 7/7, core 67/67, infra 77 passed/3 ignored, Clippy core+infra `-D warnings` y fmt check. Sin fingerprint/missing/shim no hay spawn; versión desconocida no pasa; incompatible se marca Unsupported.
- [x] **T017.b5a — Documentar fuentes oficiales por plataforma** (4 archivos): `docs/SOURCES.md`; `docs/SPEC-policy-engine.md`; `tasks/todo.md`; `tasks/RESUME.md`. Confirmar URLs oficiales de instalación/version probes para cada herramienta, sin publicar ni ejecutar cambios externos.
- Evidencia T017.b5a: consultadas páginas oficiales de Git SCM, Node/npm, rustup, GitHub CLI, Gitleaks, Commitlint, RustSec y cargo-deny; se registran guías Windows/Linux y fuentes de comandos version en `docs/SOURCES.md`.
- [x] **T017.b5b — Enlazar guías de instalación en perfiles** (5 archivos): `profiles/tools.toml`; `crates/jameskills-infra/src/platform.rs`; `crates/jameskills-infra/tests/tool_detection.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Fuentes por Windows/Linux app-owned, typed y fail-closed; ningún URL llega desde manifest/skill.
- Evidencia T017.b5b: RED integration E0599 por falta de `ToolProfile::install_guides`; GREEN tool_detection 8/8 y parser unit 1/1; infra 81 passed/3 ignored; Clippy core+infra `-D warnings`, fmt/diff check. Solo acepta los 18 IDs oficiales por SO; `Other`, URLs no registradas y guías cruzadas no resuelven.
- [x] **T017.c — Resolver stack desde manifests del proyecto** (c1+c2+c3): inferir stack solo desde `Cargo.toml`/`package.json` acotados; nunca del nombre o metadatos de skill.
- [x] **T017.c1 — Especificar RED de facts de manifests** (5 archivos): `crates/jameskills-infra/Cargo.toml`; `Cargo.lock`; `crates/jameskills-infra/tests/tool_detection.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Dependencia JSON pinneada y tests para Rust/Node/mixed/malformed/oversized.
- Evidencia T017.c1: RED E0432 al pedir `ProjectStack`/`inspect_project_manifests` inexistentes; añadida dependencia directa `serde_json=1.0.149` ya presente en lock; `cargo check -p jameskills-infra --offline` pasó.
- [x] **T017.c2 — Implementar inspección segura de manifests** (5 archivos): `crates/jameskills-infra/src/platform.rs`; `crates/jameskills-infra/tests/tool_detection.rs`; `docs/SPEC-policy-engine.md`; `tasks/todo.md`; `tasks/RESUME.md`. Root validado, symlinks/non-regular rechazados, límites, nombres de scripts sin ejecutar ni preservar sus valores.
- Evidencia T017.c2: RED E0432 por APIs ausentes; GREEN `tool_detection` 11/11 y suites core 67/67, infra 81 passed/3 ignored; Clippy core+infra `-D warnings`, fmt check. Cargo.toml/package.json solo lectura con límite 1 MiB; malformados/oversized/non-regular -> Unknown; ausencia -> Generic; scripts no ejecutados.
- [x] **T017.c3 — Incorporar firmas públicas en contratos** (3 archivos): `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. Documentar ToolDetection/evidence, ToolProfile/candidate/guides, detect_tools y manifest facts.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [x] Detección separa presencia/version/capacidad y guarda fuente/revisión de la comprobación.
- [x] No inferir npm/cargo/etc solo por nombre del skill; leer manifests y configuración.
- [x] Registry define probes seguros por tool ID y guía de instalación oficial por OS.

**Verificación:** cargo test -p jameskills-infra --locked tool_detection; doctor se conectará en T026. No activar una herramienta por checkbox manual.

**Evidencia al ejecutar:** RED por APIs ausentes; GREEN `cargo test -p jameskills-core --locked` 67/67 y `cargo test -p jameskills-infra --locked` 81 passed/3 ignored en Windows; Clippy core+infra `-D warnings`, fmt check y diff check pasaron. Linux CI/runtime no observado en esta sesión.

<a id="t018"></a>

## T018 — Implementar evaluación de políticas y evidencia

- [x] **T018 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T011, T016, T017. **Estado:** completada localmente; verificada en Windows.

**Implementación y funciones:** async `PolicyService::check(CheckRequest) -> AppResult<CheckReport>`; domain `evaluate_predicate` y `strict_exit`; provider async inyectable y expiración monotónica de evidencia. APIs públicas siguen docs/CONTRACTS.md.

**Red primero:** policy_evaluation no transforma Unknown/Blocked en Pass; una evidencia caducada invalida resultado y requerido vs recomendado se distingue. Enforcement exigida no se copia como observada: conventional message válido sin hook no pasa LocalHook; workflow válido sin host mandatory no pasa RequiredCi.

**Descomposición obligatoria:**
- [x] **T018.a — Declarar condiciones tipadas de aplicabilidad** (5 archivos): `crates/jameskills-core/src/domain/policy.rs`; `crates/jameskills-core/tests/policy_schema.rs`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. `applies_when` solo acepta facts/valores registrados; desconocidos fallan cerrados.
- Evidencia T018.a: RED E0432/E0599 por ApplicabilityFact/applies_when ausentes; GREEN `cargo test -p jameskills-core --locked --test policy_schema` 7/7. OS/architecture/stack/host/context/capability aceptan solo valores registrados.
- [x] **T018.b — Evaluador, servicio e integración fail-closed** (5 archivos): `crates/jameskills-core/src/domain/policy.rs`; `crates/jameskills-core/src/application/policy.rs`; `crates/jameskills-core/src/application/mod.rs`; `crates/jameskills-core/tests/policy_evaluation.rs`; `crates/jameskills-infra/src/composition.rs`. Status/evidence/expiry, autoridad observada separada y provider desconocido por defecto.
- Evidencia T018.b: RED `policy_evaluation` E0432 por tipos/report/service ausentes; GREEN policy_evaluation 9/9, core 77/77, infra 82 passed/3 ignored; Clippy core+infra `-D warnings`, fmt check. Unknown/Blocked no pasan; expiry -> Unknown; hook/CI authority insuficiente -> Blocked; solo requeridos bloquean strict; provider runtime default Unknown.
- [x] **T018.c — Documentar contratos de evaluación y reportes** (4 archivos): `docs/CONTRACTS.md`; `docs/SPEC-policy-engine.md`; `tasks/todo.md`; `tasks/RESUME.md`. Firmas exactas, evidencia monotónica/UTC, autoridad observada y regla de N/A basada en applies_when+fact evidence.

**Aceptación:**
- [x] Evaluador puro produce resultados/next actions estructurados por requisito.
- [x] Evidence/CheckResult registran enforcement OBSERVADA, separada de autoridad EXIGIDA por requirement; una autoridad inferior no satisface superior. Ningún enum del manifest es prueba.
- [x] Excepciones/no aplicable exigen razón verificable del perfil, no toggle de ocultación.

**Verificación:** cargo test -p jameskills-core --locked policy_evaluation; property tables cubren cada estado y ausencia de facts.

**Evidencia al ejecutar:** RED `policy_evaluation` E0432/E0599 por APIs ausentes; GREEN policy_evaluation 9/9, policy_schema 7/7, core 77/77 e infra 82 passed/3 ignored en Windows; Clippy core+infra `-D warnings`, fmt y diff check. Linux CI/runtime no observado; provider sin driver permanece Unknown.

## C006 — Checkpoint tras T016–T018

- [ ] **C006 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Facts/probes/check states confiables y errores de entorno no equivalen a pass.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia 2026-10-04:** Windows host: `cargo test --workspace --features jameskills-desktop/test-support --locked` pasó; `cargo clippy --workspace --all-targets --features jameskills-desktop/test-support --locked -- -D warnings`, fmt/diff check y `cargo build -p jameskills-desktop --target x86_64-pc-windows-msvc --locked` pasaron. No hay PR/runs remotos y Linux no se ejecutó aquí. C006 permanece sin marcar por los requisitos nativos pendientes de T005/C005; no se infiere estado Linux/GPU.

<a id="t019"></a>

## T019 — Comprobar README, gitignore y secretos locales

- [ ] **T019 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T018, T012. **Estado:** pendiente.

**Implementación y funciones:** check_readme con parser Markdown AST; check_gitignore mediante git check-ignore --no-index; check_tracked_secrets usa el driver Gitleaks registrado para el working tree, con config temporal app-owned, redacción y límites. El history permanece Unsupported y un `.gitleaksignore` del repo bloquea antes del spawn porque Gitleaks lo aplica desde el source. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** repo_document_checks secreto sintético tracked falla aunque esté en gitignore; Gitleaks ausente Blocked; README heading vacío Fail; safe path/symlink escape rechazado.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/policy.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/tests/repo_document_checks.rs`
- `tests/fixtures/repo-policy/README.md`
- `tests/fixtures/repo-policy/.gitignore`

**Descomposición obligatoria:**
- [x] **T019.a1 — Check estructural README con AST** (5 archivos): `crates/jameskills-infra/Cargo.toml`; `Cargo.lock`; `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/repo_document_checks.rs`; `tests/fixtures/repo-policy/README.md`. Parser markdown pinneado; read bounded/no-follow; headings requeridos con body no vacío.
- Evidencia T019.a1: RED E0599 por falta de `LocalFileSystem::check_readme_sections`; GREEN repo_document_checks 3/3. Markdown `1.0.0` con AST; headings en fences no cuentan; secciones vacías fallan; no regular/oversized Blocked.
- [x] **T019.a1b — Registrar fuente/API del parser Markdown** (3 archivos): `docs/SOURCES.md`; `tasks/todo.md`; `tasks/RESUME.md`. Pin/source `markdown=1.0.0`, firma `to_mdast` verificada.
- Evidencia T019.a1b: docs.rs markdown 1.0.0 confirmó `to_mdast(&str, &ParseOptions) -> Result<Node, Message>`; referencia versionada añadida a SOURCES.
- [x] **T019.a2 — Check gitignore vía Git** (5 archivos): `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/repo_document_checks.rs`; `tests/fixtures/repo-policy/.gitignore`; `tasks/todo.md`; `tasks/RESUME.md`. argv fijo `check-ignore --no-index -v -z`, paths sintéticos; comparar el pattern real sin ejecutar valores importados.
- Evidencia T019.a2: RED E0599 por falta de `check_gitignore_patterns`; GREEN `repo_document_checks` 5/5; Clippy infra `-D warnings`, fmt/diff check. Solo ejecuta samples app-owned, exige fingerprint Git y compara output NUL delimitado sin exponerlo.
- [x] **T019.b1 — Declarar driver de scan Gitleaks** (5 archivos): `profiles/tools.toml`; `crates/jameskills-infra/src/platform.rs`; `crates/jameskills-infra/tests/repo_document_checks.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Args/redaction/exit semantics app-owned y schema de reporte cerrado.
- Evidencia T019.b1: RED al omitir scan spec, loader rechaza registry incompatible; GREEN `gitleaks_scan_profile_is_redacted_bounded_and_has_distinct_exit_codes` pasa. Args `dir`, redaction, JSON, findings exit 3 y output cap 64 KiB.
- [x] **T019.b1a — Forzar configuración Gitleaks app-owned** (5 archivos): `profiles/tools.toml`; `crates/jameskills-infra/src/platform.rs`; `crates/jameskills-infra/tests/repo_document_checks.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Scan requiere config placeholder privado con `useDefault=true`; ignorar `.gitleaks.toml` del repo; aceptar únicamente Gitleaks 8.30.1, cuya CLI/schema se verificaron.
- Evidencia T019.b1a: RED focused porque el argv del scan no incluía `--config {APP_GITLEAKS_CONFIG}`; GREEN `gitleaks_scan_profile_is_redacted_bounded_and_has_distinct_exit_codes` y suite `repo_document_checks` 11/11. El loader solo acepta el placeholder fijo, fuerza default rules y rechaza versiones distintas a 8.30.1.
- [x] **T019.b2a — Parsear reportes Gitleaks bounded** (5 archivos): `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/repo_document_checks.rs`; `tests/fixtures/repo-policy/gitleaks-findings.json`; `tasks/todo.md`; `tasks/RESUME.md`. Findings nunca salen en CheckEvidence/diagnostics; malformed/oversized -> Unknown.
- Evidencia T019.b2a: RED E0432 por parser/status ausentes; GREEN `repo_document_checks` 7/7. Reporte array/schema validado, bytes >64 KiB/malformed -> Unknown; parser retorna solo Findings/NoFindings/Unknown sin valores reportados.
- [x] **T019.b2a2 — Documentar fuente/schema JSON Gitleaks** (3 archivos): `docs/SOURCES.md`; `tasks/todo.md`; `tasks/RESUME.md`. Fuente oficial v8.30.1 y fixture sanitizada; no guardar secret material.
- Evidencia T019.b2a2: referencia al fixture upstream v8.30.1; test fixture solo incluye marcadores sintéticos redacted.
- [x] **T019.b2b — Ejecutar Gitleaks con identidad aprobada** (4 archivos): `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/repo_document_checks.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Candidate exacto 8.30.1 + fingerprint; exit/phase explícitos; history Unsupported sin nested-Git identity.
- Evidencia T019.b2b: RED al desactivar parser, scan con findings dejó de producir Fail; GREEN repo_document_checks 9/9 y Clippy infra `-D warnings`. Versión exacta 8.30.1 y fingerprint verificados antes de ambos spawns; JSON no se expone; history queda Unsupported.
- [x] **T019.b2c — Staging privado de config Gitleaks** (5 archivos): `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/repo_document_checks.rs`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. Config efímera modo privado, argv `--config` app-owned, se limpia siempre; la config del repo no altera reglas.
- Evidencia T019.b2c: RED `gitleaks_scan_blocks_repository_ignore_file_before_spawning` obtuvo Pass y lanzó el fake al encontrar `.gitleaksignore`; GREEN test pasa en `repo_document_checks` 11/11 Windows. Config incluye `useDefault=true`, vive fuera del repo, su argv se confirma y el path ya no existe tras el scan. Si `.gitleaksignore` existe/no puede inspeccionarse se devuelve Blocked sin spawn.
- [x] **T019.c1 — Propagar fallos/cancelación del provider** (5 archivos): `crates/jameskills-core/src/application/policy.rs`; `crates/jameskills-infra/src/composition.rs`; `crates/jameskills-core/tests/policy_evaluation.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Provider async retorna AppResult; Cancelled no se convierte en check Unknown/Blocked silencioso.
- Evidencia T019.c1: RED E0053 cuando el provider debía retornar AppResult pero el trait solo permitía Observation; GREEN policy_evaluation 10/10 y composition unavailable-provider 1/1.
- [x] **T019.c2 — Conectar repo document provider a PolicyService** (5 archivos): `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/repo_document_checks.rs`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. Dispatch por Check, README/gitignore/Gitleaks evidence redacted; unavailable tools status fail-closed.
- Evidencia T019.c2: RED al desconectar README dispatch, PolicyService no reportó Pass; GREEN repo_document_checks 10/10 y Clippy core+infra `-D warnings`. Evidence summaries son app-authored; no se copian findings ni stdout/stderr.
- [x] **T019.c3 — Documentar fuentes y semánticas de repo checks** (4 archivos): `docs/SOURCES.md`; `docs/SPEC-policy-engine.md`; `tasks/todo.md`; `tasks/RESUME.md`. Git check-ignore, Gitleaks dir/redaction/exit codes, working tree vs history Unsupported.
- Evidencia T019.c3: Git SCM `git-check-ignore` y Gitleaks v8.30.1 CLI/source verifican precedencia `--config`, default rules y carga de `.gitleaksignore`; docs declaran working-tree, bloqueo por ignore file, history Unsupported y redaction.

**Aceptación:**
- [ ] Cada check explica archivo/regla y evidencia sin imprimir valor secreto.
- [ ] No scanner propio sustituye al estándar Gitleaks: missing/incompatible/version no probada Blocked; evidencia especifica fase/rango y límites.
- [ ] `.gitleaksignore` presente o no inspeccionable bloquea antes del scan; el archivo `.gitleaks.toml` del repo no controla reglas.
- [ ] No borrar/rewrite archivos automáticamente; proporcionar remediación e información de rotación cuando aplica.

**Verificación:** cargo test -p jameskills-infra --locked repo_document_checks; fixture usa findings JSON de Gitleaks y proceso fake redacted; contrato opt-in con driver real pinneado sin secreto real.

**Evidencia al ejecutar:** T019.b1a/b2c y `repo_document_checks` 11/11 pasan localmente en Windows. El workspace test-support, Clippy, fmt, diff y build desktop Windows pasan; los commits locales `1497ccf` (profiles/probes), `ecf7025` (repo checks) y `d27cb5c` (docs) registran estas capas. No hay PR ni ejecución remota de CI. `gitleaks` no está instalado en el host (`Get-Command gitleaks` sin resultado), así que falta integración real con binario exacto 8.30.1; no sustituirla por fake. Historia de Git y checkpoint C006 siguen abiertos como abajo.

**Secuencia local por capas (2026-10-04):** `05c6d07` registry/evidence; `a8db8a8` PolicyService/providers; `7bd60f8` contrato fingerprint; `f3dcbc7` ejecución con fingerprint; `1497ccf` profiles/detection; `ecf7025` checks de repo; `d27cb5c` contratos/fuentes. Los mensajes pasan hooks locales; ninguno tiene resultado CI remoto por falta de PR.

<a id="t020"></a>

## T020 — Comprobar Conventional Commits y pruebas declaradas

- [ ] **T020 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T018, T016, T017. **Estado:** pendiente.

**Implementación y funciones:** `check_conventional_commits` obtiene el mensaje de HEAD con Git aprobado, lo escribe en archivo temporal privado y ejecuta Commitlint 21.2.2 con `--default-config --edit`, cwd/config privados y config convencional incorporada; nunca carga config JS del repo. `check_test_commands` usa drivers Rust/Node registrados y solo corre tras acción explícita con trust del repo. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** commit_test_checks Commitlint fixture acepta/rechaza subjects/scopes/footer; tool missing/incompatible Blocked, exit code1 fallo real; package script repo no reviewed no se ejecuta. Mensaje válido sin core.hooksPath efectivo+hook own hash/executable+driver invocado solo LocalCheck, no LocalHook.

**Descomposición obligatoria:**
- [x] **T020.a — Pin de Commitlint con default config verificado** (5 archivos): `profiles/tools.toml`; `crates/jameskills-infra/tests/tool_detection.rs`; `docs/SOURCES.md`; `tasks/todo.md`; `tasks/RESUME.md`. Aceptar solo CLI 21.2.2, cuya fuente confirma `--default-config` y `--edit`; versiones sin este contrato quedan Blocked.
- Evidencia RED T020.a: `cargo test -p jameskills-infra --locked --test tool_detection commitlint_profile_supports_the_reviewed_default_config_cli` falla porque el profile existente limita Commitlint a `<21.0.0` y no admite la versión fuente 21.2.2.
- Evidencia GREEN T020.a: el mismo test focal pasa 1/1 tras fijar `=21.2.2`; rechaza tanto 20.2.0 como 21.2.3 no revisada. Fuente oficial tag v21.2.2 documenta `--default-config` y `--edit <file>`.
- [x] **T020.b1 — Driver oficial Commitlint para mensaje HEAD privado** (5 archivos): `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/commit_test_checks.rs`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. Git y Commitlint native/fingerprint aprobado, config JSON privada vacía + `--default-config`, cwd privado, PATH antepone Git aprobado para el `git config` interno oficial; salida redacted.
- Evidencia RED/GREEN T020.b1: RED E0599 por `check_conventional_commit` ausente. GREEN `cargo test -p jameskills-infra --locked --test commit_test_checks` 3/3; suite infra pasa; Clippy infra `-D warnings`, fmt y diff check pasan. Verifica argv, config/mensaje fuera del repo, cleanup, exit 0/1, versiones no revisadas, Git secundario acotado al binario aprobado y no ejecución de candidatos desconocidos. El test fake verifica contrato; instalación estándar Windows `.cmd` sigue Blocked, sin ejecución real afirmada. Commit `d304f64`.
- [x] **T020.b2 — Resolver CLI Commitlint y conectar PolicyService** (subdividida por límite de cinco archivos por incremento): dispatch `conventional-commit` en `RepositoryPolicyCheckProvider`; soporte seguro del package CLI Node en Windows sin ejecutar `.cmd`; aprobar ambos ejecutables/entrypoint; tool no disponible -> Blocked; no declarar LocalHook.
- [x] **T020.b2.a — Atar fingerprint del entrypoint al spawn Node** (5 archivos): `crates/jameskills-core/src/ports/process.rs`; `crates/jameskills-infra/src/process.rs`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. `ProcessSpec` conserva el entrypoint JS y su fingerprint y la infraestructura los revalida inmediatamente antes de spawn.
- Evidencia RED/GREEN T020.b2.a: `cargo test -p jameskills-infra --locked --lib process_rejects_modified_approved_script_before_spawning_runtime` primero falla porque el runtime se inicia aun cuando el fingerprint aprobado no coincide y luego pasa 1/1 al rechazarlo antes del spawn. `cargo clippy -p jameskills-infra --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check` y `git diff --check` pasan. Un intento inicial de test no compiló por `unwrap_err` requiriendo `ProcessOutput: Debug`; no se cuenta como RED.
- [x] **T020.b2.b — Resolver CLI Commitlint y conectar PolicyService** (subdividida para incorporar el engine Node oficial y mantener <=5 archivos por incremento): aprobar Node y `@commitlint/cli/cli.js`; ejecutar Commitlint 21.2.2 vía Node sin shell shim; despachar `conventional-commit` desde el provider como LocalCheck.
- [x] **T020.b2.b1 — Alinear el rango de descubrimiento Node con LTS v24** (5 archivos): `profiles/tools.toml`; `crates/jameskills-infra/tests/tool_detection.rs`; `docs/SOURCES.md`; `tasks/todo.md`; `tasks/RESUME.md`. El profile genérico incluye Node 18–24, y el driver Commitlint mantiene su mínimo separado `>=22.12.0` desde el package manifest oficial.
- Evidencia RED/GREEN T020.b2.b1: `cargo test -p jameskills-infra --locked --test tool_detection node_profile_covers_commitlint_supported_node_24_runtime` falla antes y pasa 1/1 después de actualizar el profile; suite `cargo test -p jameskills-infra --locked --test tool_detection` pasa 13/13; Clippy infra `-D warnings`, fmt y diff check pasan. Node 25 se mantiene fuera del rango.
- [x] **T020.b2.b2 — Cablear el CLI Commitlint mediante Node** (subdividida para registrar el source gate en un segundo slice <=5 archivos): aprobar Node y script CLI por separado; version probe y lint con argv fijo/fingerprints; dispatch `conventional-commit` en `RepositoryPolicyCheckProvider`; autoridad `LocalCheck` únicamente.
- [x] **T020.b2.b2.a — Provider Node + LocalCheck** (5 archivos): `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/commit_test_checks.rs`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. Node 22.12+, script `@commitlint/cli/cli.js` fingerprinted; ruta CLI root Git solo mediante `--cwd` explícito y carga de config JSON privada; ningún shim `.cmd`.
- Evidencia RED/GREEN T020.b2.b2.a: RED provider devolvía Unknown porque conventional-commit no tenía dispatch. GREEN `cargo test -p jameskills-infra --locked --test commit_test_checks` pasa 6/6 (un test real queda ignored por defecto); cubre autoridad LocalCheck, Blocked sin tool y Node <22.12.0. Integración real explícita `cargo test -p jameskills-infra --locked --test commit_test_checks real_node_commitlint_package_passes_through_repository_policy_provider -- --ignored --exact` ejecutó Node 24.18.0 + Commitlint 21.2.2 vía `SystemProcessPort` y pasó 1/1. `cargo test -p jameskills-infra --locked` pasó; Clippy infra `-D warnings`, fmt y diff check pasan. Primero detectó que `--edit` necesita root Git; se resolvió con `--cwd <repo-root>` junto a `--config` absoluto.
- La integración real prueba solo el paquete instalado/entrypoint fingerprinted en este host; no certifica la supply chain completa de dependencias ni equivale a CI remota.
- [x] **T020.b2.b2.b — Registrar fuentes del cwd/config explícitos** (4 archivos): `docs/SOURCES.md`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. Citar `@commitlint/load` v21.2.2 `load-config.ts` y `@commitlint/read` v21.2.2 `get-edit-commit.ts`, para respaldar que `--cwd` encuentra repo Git mientras `--config` explícito solo carga el JSON privado.
- Evidencia GREEN T020.b2.b2.b: fuentes oficiales tag v21.2.2 enlazadas; `cargo fmt --all -- --check` y `git diff --check` pasan. T020.b2 queda verificada con fake y una integración Node real en Windows; T020 parent sigue abierta por T020.c.
- [ ] **T020.c — Ejecutar suites solo bajo acción explícita** (subdividida: la inspección no debe lanzar suites y el test execution no se declara Pass sin prueba y aceptación separadas).
- [x] **T020.c.a — Hacer efectiva la autorización de proceso mutante** (5 archivos): `crates/jameskills-infra/src/process.rs`; `crates/jameskills-infra/tests/process_execution.rs`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. `ExplicitMutation(OperationId)` permite spawn acotado en SystemProcessPort; la variante ReadOnlyCheck permanece disponible. El permiso low-level no sustituye trust/consent de repo.
- Evidencia RED/GREEN T020.c.a: `cargo test -p jameskills-infra --locked --test process_execution process_runner_executes_only_when_explicit_mutation_permission_is_present` primero falla porque SystemProcessPort rechaza el permiso explícito y luego pasa 1/1. Suite `cargo test -p jameskills-infra --locked` pasa; Clippy infra `-D warnings`, fmt y diff check pasan. Se ejecuta un test binario fixture con exe fingerprinted, argv fijo, timeout/output acotado; no se ejecuta código de manifiesto del proyecto.
- [ ] **T020.c.b — Consentimiento tipado y driver Rust registrado** (subdividida en slices <=5 archivos): Cargo test argv fijo; aprobación explícita atada a repo seleccionado/HEAD/manifest; timeout/output/cancel acotados; reportar suite declarada por separado del exit status.
- [x] **T020.c.b.a — Modelo de resultado de suite sin conflar presencia/ejecución** (5 archivos): `crates/jameskills-core/src/domain/policy.rs`; `crates/jameskills-core/tests/policy_evaluation.rs`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. Enums tipados para Cargo test/Node lint-test-build, declaración vs ejecución y código de salida con combinaciones validadas.
- Evidencia RED/GREEN T020.c.b.a: `cargo test -p jameskills-core --locked --test policy_evaluation suite_run_result_rejects_exit_and_execution_status_mismatches` falla antes al aceptar exit 1 como Passed; tras validación GREEN los tests c.b.a pasan 2/2, incluyendo suite Missing/NotRun y resultado Failed con exit 1.
- [ ] **T020.c.b.b — Aprobación explícita + driver Cargo** (subdividida en slices <=5 archivos): aprobación no deserializable ligada al root/HEAD/manifiesto; validar snapshot antes spawn; argv fijo `cargo test --locked`; solo suite solicitada; resultados distintos de cargo-not-found, build/test failure y falta de suite.
- [x] **T020.c.b.b.a — DTO de aprobación explícita** (5 archivos): `crates/jameskills-core/src/domain/policy.rs`; `crates/jameskills-core/src/application/policy.rs`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. Approval app-owned, no deserializable, liga root seleccionado, suite enum fija, Git head validado, manifest fingerprint y OperationId; no ejecuta procesos ni declara éxito.
- RED/GREEN T020.c.b.b.a: `cargo test -p jameskills-core --locked --lib repository_head_rejects_unreviewed_or_malformed_identifiers` falla si el parser acepta SHA malformado/mayúsculas; después pasa 1/1. `cargo test -p jameskills-core --locked` y Clippy `-D warnings` pasan; el test de approval confirma root/head/hash/suite ligados al consentimiento.
- [ ] **T020.c.b.b.b — Runner Cargo + servicio con revalidación de aprobación** (subdividida para documentar la fuente oficial aparte y conservar <=5 archivos por slice): Cargo metadata declarativo; root/head/manifests revalidados; argv fijo `cargo test --locked`; ExplicitMutation y límites; test fixture pass/fail sin ejecución desde inspección.
- [x] **T020.c.b.b.b.a — TestSuiteService + provider Cargo real** (5 archivos, commit `4ffa1e6`): `crates/jameskills-core/src/application/policy.rs`; `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/test_suite_checks.rs`; `tasks/todo.md`; `tasks/RESUME.md`. Inspección Cargo metadata read-only y `run(approval)` revalida root/HEAD/manifests antes de `ExplicitMutation`.
- Evidencia RED/GREEN T020.c.b.b.b.a: RED focal `cargo test -p jameskills-infra --locked --test test_suite_checks` ejercitó suites Cargo declaradas/ausentes, read-only hasta aprobación, argv acotado, resultado exit distinto de cero y HEAD obsoleto; el nuevo caso cambia HEAD tras la última metadata y demuestra que no hay spawn mutante. GREEN focal 4/4 y suite infra completa pasan; también core tests, Clippy infra/core `-D warnings`, fmt, diff-check y Commitlint local pasan (detalle en `tasks/RESUME.md`). Opt-in real ejecutado explícitamente y no pasa: Cargo devuelve exit 101 al compilar el fixture en este Windows; no se afirma pass real. T020.c.b.b queda abierta hasta integración Cargo real pass/fail. Sin PR abierta no hay CI remota para esta rama.
- [x] **T020.c.b.b.b.b — Fuentes/contrato Cargo metadata/test** (4 archivos, commit `94d9c59`): `docs/SOURCES.md`; `docs/CONTRACTS.md`; `tasks/todo.md`; `tasks/RESUME.md`. Cita Cargo 1.95 versionado para `metadata --no-deps --format-version 1`, `workspace_members`, `targets[].test` y `cargo test --workspace --locked`.
- Evidencia T020.c.b.b.b.b: consultadas las páginas oficiales de Cargo 1.95 para metadata y test; `cargo metadata --no-deps --format-version 1 --locked --offline` local devuelve miembros/targets con los campos documentados. Aceptación documental y shape de metadata registrados; no implica ejecutar las suites.
- [ ] **T020.c.c — Driver Node/npm de suites registradas** (subdividida en slices <=5 archivos): solo script id explícito mapeado a script detectado; Node + npm CLI oficial fingerprinted sin ejecutar `.cmd`; trust/argv/cwd/timeout bounded; no inferir test coverage del script.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/policy.rs`
- `crates/jameskills-core/src/domain/policy.rs`
- `crates/jameskills-infra/src/process.rs`
- `crates/jameskills-infra/tests/commit_test_checks.rs`

**Aceptación:**
- [ ] Conventional Commits se verifica vía Commitlint; LocalHook exige evidencia de hook efectivo/config/hash/ejecutable/driver y sigue eludible; no convertir un mensaje válido en hook aplicado.
- [ ] Pruebas se ejecutan solo mediante acción explícita con cwd/argv/timeout conocidos; inspección no las dispara.
- [ ] Logs acotados/saneados y evidencia del exit code no se confunde con existencia de tests.
- [ ] Commitlint no carga configuración ejecutable del repositorio y solo prueba LocalCheck; LocalHook exige evidencia independiente de hook efectivo.

**Verificación:** cargo test -p jameskills-infra --locked commit_test_checks; ejecutar check sobre repo fixture con test registrado que pasa y otro que falla.

**Evidencia al ejecutar:** T020.a/T020.b1–b2 y T020.c.a/T020.c.b.a/T020.c.b.b.a están registradas en sus slices. T020.c.b.b.b.a ya tiene GREEN local y commit pendiente; la integración Cargo real no obtuvo Pass en este host (exit 101), la documentación oficial Cargo va en T020.c.b.b.b.b y el driver Node/npm de suites permanece pendiente. T020 parent no se cierra hasta cubrir todas esas acceptance items.

<a id="t021"></a>

## T021 — Validar definición de CI y checks requeridos

- [ ] **T021 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T018, T020. **Estado:** pendiente.

**Implementación y funciones:** check_ci_definition, inspect_workflow_requirements; separar archivo existente, jobs declarados y evidencia de ejecución/remoto. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** ci_definition_checks workflow sin test/PR trigger o con continue-on-error en gate requerido falla; YAML inválido/host distinto es Unknown con razón. Workflow válido con SHA viejo o sin rule host obligatoria no prueba RequiredCi.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/policy.rs`
- `crates/jameskills-core/src/domain/policy.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/tests/ci_definition_checks.rs`

**Aceptación:**
- [ ] Parser no ejecuta YAML y valida solo proveedores/versiones soportados.
- [ ] Workflow válido prueba definición local; RequiredCi exige host required rule y ejecución para SHA EXACTO, no pasar enum requerido de la suite como observado.
- [ ] Guía declara tools/jobs/permissions pendientes y prepara requests de evidencia remota.

**Verificación:** cargo test -p jameskills-infra --locked ci_definition_checks; comparar ejemplos buenos/malos y limitaciones documented.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C007 — Checkpoint tras T019–T021

- [ ] **C007 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- README/gitignore/secrets/commits/tests/CI tienen checks y outputs saneados.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t022"></a>

## T022 — Obtener auth y evidencia de GitHub con mínimos permisos

- [ ] **T022 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T018, T016. **Estado:** pendiente.

**Implementación y funciones:** GitHubEvidenceDriver, inspect_host_auth, identify_repository_host; gh auth status y gh api read-only aprobado con JSON bounds/typed observations, sin extraer token del CLI. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** github_evidence gh exit/JSON 401/403/404/429 y provider no GitHub distinguen reauth/permission/unknown/retry; remote malicioso no provoca request arbitraria.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/policy.rs`
- `crates/jameskills-infra/src/github.rs`
- `crates/jameskills-infra/src/lib.rs`
- `crates/jameskills-infra/src/process.rs`
- `crates/jameskills-infra/tests/github_evidence.rs`

**Aceptación:**
- [ ] Remote host Git validada y API host permitido; ningún cambio remoto durante checks.
- [ ] Evidencia incluye repo/ref/check/source y permisos disponibles mediante gh aprobado; nunca auth tokens en argv/log/DB.
- [ ] Tokens no se copian del CLI al bundle/DB; rate limit/red limitada no convierte check en pass.

**Verificación:** cargo test -p jameskills-infra --locked github_evidence con ProcessPort fake y fixtures JSON/argv gh; integración opt-in read-only en repo de prueba autorizado sin almacenar token propio.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t023"></a>

## T023 — Validar rulesets, PR y estado CI de main

- [ ] **T023 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T021, T022. **Estado:** pendiente.

**Implementación y funciones:** check_main_protection, check_pull_request_policy, check_required_status_checks; interpretar branch protection/rulesets y effective ref según API oficial. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** host_protection_checks PR requirement ausente, bypass actor, required check faltante, permiso insuficiente y ref distinta no pasan protección. RequiredCi solo pasa con rule host mandatory+currentSHA successful; valid workflow sin rule falla autoridad exigida.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/policy.rs`
- `crates/jameskills-infra/src/github.rs`
- `crates/jameskills-infra/tests/host_protection_checks.rs`
- `docs/POLICY-LIMITS.md`

**Aceptación:**
- [ ] Separar autoridad exigida/observada: RequiredCi requiere host regla mandatory y check actualSHA; HostRule exige configuración efectiva incluyendo classic+rulesets+bypass.
- [ ] Excepciones/bypass y privilegios se exponen sin afirmar main absolutamente inaccesible.
- [ ] Proveedor unsupported ofrece guía documentada; cambios remotos no se aplican sin acción y autorización explícitas.

**Verificación:** cargo test -p jameskills-infra --locked host_protection_checks con fixtures classic branch protection+rulesets vía gh api; integración read-only opt-in fecha/SHA/permisos saneados.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t024"></a>

## T024 — Validar versiones, tags y releases

- [ ] **T024 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T018, T022. **Estado:** pendiente.

**Implementación y funciones:** check_version_consistency, check_release_artifacts, check_release_policy; semver/tag/changelog/artifacts/signature metadata según requisito. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** release_checks tag incorrecto, prerelease, release ausente/permiso desconocido y asset checksum faltante tienen resultados distintos.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/policy.rs`
- `crates/jameskills-core/src/domain/policy.rs`
- `crates/jameskills-infra/src/github.rs`
- `crates/jameskills-infra/tests/release_checks.rs`

**Aceptación:**
- [ ] Versionado de skill y release de repo no se confunden.
- [ ] Solo pedir criterios declarados por perfil; presencia de tag no prueba publicación.
- [ ] Explicar pasos de release sin crear tags/publicar durante check.

**Verificación:** cargo test -p jameskills-infra --locked release_checks; Git local y gh API JSON fake cubren casos; host real solo read-only autorizado.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C008 — Checkpoint tras T022–T024

- [ ] **C008 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- GitHub evidence/protection/PR/releases verifican privilegios y fuente; no writes no solicitados.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t025"></a>

## T025 — Generar guía dinámica desde hechos y checks

- [ ] **T025 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T017, T018, T023, T024, T037. **Estado:** pendiente.

**Implementación y funciones:** GuidanceService::start_guidance, advance, recheck; domain::next_step/validate_guidance_graph; helpers privados build_guidance_plan y environment fingerprint. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** guidance_planner misma policy en Linux/Windows o cargo/npm/missing auth produce pasos distintos; una dependencia imposible bloquea descendientes, no todos los checks.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/guidance.rs`
- `crates/jameskills-core/src/domain/guidance.rs`
- `crates/jameskills-core/src/application/mod.rs`
- `crates/jameskills-core/tests/guidance_planner.rs`
- `crates/jameskills-infra/src/composition.rs`

**Aceptación:**
- [ ] Cada paso tiene condición de éxito comprobable, tool/link aprobado y explicación del requisito.
- [ ] Plan no prescribe comandos de otro OS/provider ni asegura privilegios inexistentes.
- [ ] Recheck invalida evidence antigua y actualiza pendientes/completos mediante resultado real.

**Verificación:** cargo test -p jameskills-core --locked guidance_planner; snapshots de planes con fixtures de entornos diferentes y ciclo rechazado.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t026"></a>

## T026 — Ejecutar acciones registradas y doctor

- [ ] **T026 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T009, T025, T016. **Estado:** pendiente.

**Implementación y funciones:** GuidanceService::advance + recheck, doctor_command; ManualInstruction/OpenOfficialUrl/CopyApprovedCommand/SelectLocalPath/AnswerChoice/Recheck. Mutaciones reales pasan por PolicyService ApprovedRepoChange, no Guidance action libre. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** guidance_actions cambio de facts entre preview/apply invalida acción; registry no ejecuta shell; doctor missing tool produce guía JSON y no instala.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/guidance.rs`
- `crates/jameskills-infra/src/process.rs`
- `crates/jameskills-cli/src/commands.rs`
- `crates/jameskills-infra/tests/guidance_actions.rs`
- `crates/jameskills-cli/tests/doctor_command.rs`

**Aceptación:**
- [ ] La guía no ejecuta shell ni registra herramientas arbitrarias; copiar comando no cambia estado. Las acciones de repositorio tienen preview/digest y API aprobada.
- [ ] Elevación no se obtiene automáticamente; guiar y revalidar tras acción del usuario.
- [ ] Doctor usa las mismas capacidades/resultados que GUI y errores tienen exit codes reales.

**Verificación:** cargo test -p jameskills-infra --locked guidance_actions; cargo test -p jameskills-cli --locked doctor_command; cargo run -p jameskills-cli --locked -- doctor --json.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t027"></a>

## T027 — Aplicar templates y hooks locales con preview

- [ ] **T027 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T014, T025, T026. **Estado:** pendiente.

**Implementación y funciones:** PolicyService::plan_repo_changes, apply_repo_changes; helpers plan_template_changes y plan_local_hook para templates propios/managed sections, journal y diff. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** template_plan archivo ajeno/dirty/version changed impide apply; hook existente se conserva; remover JameSkills no elimina hooks ajenos.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/guidance.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/tests/template_plan.rs`
- `examples/repository-foundation/templates/ci-rust.yml`
- `examples/repository-foundation/guidance/repository.toml`

**Aceptación:**
- [ ] Preview muestra contenido/paths y cambios aplican transaccionalmente tras validación.
- [ ] Hooks son opcionales y se etiquetan eludibles; CI remota y host tienen evidencia aparte.
- [ ] Templates importados se tratan como datos y no se ejecutan ni habilitan acciones nuevas.

**Verificación:** cargo test -p jameskills-infra --locked template_plan; probar cancel/crash en repo temporal y confirmar main/history sin cambios.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C009 — Checkpoint tras T025–T027

- [ ] **C009 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Guía dinámica, acciones registradas y templates tienen preview/recheck y rollback.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t028"></a>

## T028 — Conectar check CLI para uso en CI

- [ ] **T028 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T009, T018, T019, T020, T021, T023, T024, T025, T039, T042. **Estado:** pendiente.

**Implementación y funciones:** check_command; --skill --repo --profile --json --strict, selección de checks y exit codes definidos por scope/severity. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** check_command ejecuta binario real sobre fixtures pass/fail/Unknown/blocked; strict requerido no retorna cero ante requisito no verificado.

**Archivos del incremento:**
- `crates/jameskills-cli/src/commands.rs`
- `crates/jameskills-cli/src/output.rs`
- `crates/jameskills-cli/tests/check_command.rs`
- `docs/CLI.md`
- `.github/workflows/ci.yml`

**Aceptación:**
- [ ] CLI y GUI reciben mismos CheckResult; JSON schema estable apto artifacts CI sin secretos.
- [ ] Modo strict diferencia required/recommended según spec y documenta Unknown.
- [ ] Workflow invoca CLI real con suite ejemplo y no crea protección remota fingida.

**Verificación:** cargo test -p jameskills-cli --locked check_command; cargo run -p jameskills-cli --locked -- library import --path examples/repository-foundation --json; cargo run -p jameskills-cli --locked -- check --skill f9c0199f-c4ce-4b04-85dd-ae12a7db292b --repo . --profile rust --json --strict. Policies reales pueden fallar: conservar resultado, no relajar reglas.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t029"></a>

## T029 — Definir perfiles, capacidades y registry de agentes

- [ ] **T029 completada y verificada**

**Módulo:** `agent-adapters`. **Dependencias:** T010, T012, T016. **Estado:** pendiente.

**Implementación y funciones:** AgentProfile, AgentCapabilities, AgentDetection, AgentPort, InstallPlan, AgentRegistry; Scope User/Project; plugin es mecanismo vendor de User y compatibilidad de schema/version. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** agent_registry agente desconocido, version no reconocida y scope unsupported no generan destino; overrides vacíos/relativos peligrosos se rechazan.

**Archivos del incremento:**
- `crates/jameskills-core/src/domain/agent.rs`
- `crates/jameskills-core/src/ports/agent.rs`
- `crates/jameskills-core/src/lib.rs`
- `crates/jameskills-infra/src/agents/mod.rs`
- `crates/jameskills-core/tests/agent_registry.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T029.a — Dominio/puerto de agente** (5 archivos): `crates/jameskills-core/src/domain/agent.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/src/ports/agent.rs`; `crates/jameskills-core/src/ports/mod.rs`; `crates/jameskills-core/tests/agent_registry.rs`. Scope User/Project; plugin vendor en export/action. Validar caps/version/profile roots.
- [ ] **T029.b — Registry infraestructura** (3 archivos): `crates/jameskills-infra/src/agents/mod.rs`; `crates/jameskills-infra/src/lib.rs`; `crates/jameskills-infra/tests/agent_registry.rs`. Registrar solo adapters implementados de T030–T034; ausentes Candidate/NeedsVerification, nunca fake Verified.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Registry incluye cinco perfiles con evidencia/source version y features independientes.
- [ ] Formato canonical nunca se sustituye por artifact generado.
- [ ] Destinos derivados de plataforma/documentación; no inferir CLI de app con nombre parecido.

**Verificación:** cargo test -p jameskills-core --locked agent_registry; revisar catálogo contra SPEC-agent-adapters y SOURCES.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t030"></a>

## T030 — Implementar perfil Codex documentado

- [ ] **T030 completada y verificada**

**Módulo:** `agent-adapters`. **Dependencias:** T029, T017. **Estado:** pendiente.

**Implementación y funciones:** CodexAdapter::detect, capabilities, plan_artifact; project .agents/skills, user HOME/.agents/skills según fuente vigente y scope/version detectados. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** codex_adapter fixture SKILL.md+assets produce artifact esperado; no usar ~/.codex/skills como default actual; CLI output desconocido bloquea.

**Archivos del incremento:**
- `crates/jameskills-infra/src/agents/codex.rs`
- `crates/jameskills-infra/src/agents/mod.rs`
- `crates/jameskills-infra/src/platform.rs`
- `crates/jameskills-infra/tests/codex_adapter.rs`

**Aceptación:**
- [ ] Detección usa probe oficial/argv seguro y destino respeta directorios user/repo reales.
- [ ] Suite y assets se conservan con instrucciones portables sin ejecutar contenido.
- [ ] Compatibilidad/settings del agente se informa con fuente; no prometer enforcement del agente.

**Verificación:** cargo test -p jameskills-infra --locked codex_adapter; smoke opt-in con Codex real comprueba descubrimiento de fixture sin tareas de código ni acceso externo.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C010 — Checkpoint tras T028–T030

- [ ] **C010 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Check CLI strict y registry/adaptador Codex ejercitados con negativos.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t031"></a>

## T031 — Implementar perfil OpenCode documentado

- [ ] **T031 completada y verificada**

**Módulo:** `agent-adapters`. **Dependencias:** T029, T017. **Estado:** pendiente.

**Implementación y funciones:** OpenCodeAdapter::detect, capabilities, plan_artifact; .opencode/skills y directorio de config user de docs, overrides solo acreditados. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** opencode_adapter XDG_CONFIG_HOME/custom root/Windows derivation comprobada; destino incompatible no se adivina usando APPDATA.

**Archivos del incremento:**
- `crates/jameskills-infra/src/agents/opencode.rs`
- `crates/jameskills-infra/src/agents/mod.rs`
- `crates/jameskills-infra/src/platform.rs`
- `crates/jameskills-infra/tests/opencode_adapter.rs`

**Aceptación:**
- [ ] SKILL.md/frontmatter y assets coinciden con formato upstream.
- [ ] Scope project/user/version checked y no pisa carpetas existentes.
- [ ] CLI faltante genera guía/recheck; artifact existe solo tras validar bundle.

**Verificación:** cargo test -p jameskills-infra --locked opencode_adapter; smoke opt-in OpenCode real verifica listado/lectura con fuente actual y logs saneados.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t032"></a>

## T032 — Implementar perfil Pi con override de agent dir

- [ ] **T032 completada y verificada**

**Módulo:** `agent-adapters`. **Dependencias:** T029, T017. **Estado:** pendiente.

**Implementación y funciones:** PiAdapter::detect, capabilities, resolve_agent_dir, plan_artifact; project .pi/skills y user PI_CODING_AGENT_DIR/skills o ~/.pi/agent/skills. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** pi_adapter env override válido se usa, vacío/relativo no se acepta silenciosamente; caminos espacios/userprofile y assets funcionan.

**Archivos del incremento:**
- `crates/jameskills-infra/src/agents/pi.rs`
- `crates/jameskills-infra/src/agents/mod.rs`
- `crates/jameskills-infra/src/platform.rs`
- `crates/jameskills-infra/tests/pi_adapter.rs`

**Aceptación:**
- [ ] Origen docs pi.dev y upstream registran versión y comportamiento del override.
- [ ] Detección identifica Pi coding agent, no otro ejecutable pi no relacionado.
- [ ] No configurar extensiones/tools o ejecutar scripts del skill durante instalación.

**Verificación:** cargo test -p jameskills-infra --locked pi_adapter; smoke opt-in Pi real verifica skill fixture con agent dir aislado.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t033"></a>

## T033 — Implementar Antigravity CLI mediante plugin vendor

- [ ] **T033 completada y verificada**

**Módulo:** `agent-adapters`. **Dependencias:** T029, T017. **Estado:** pendiente.

**Implementación y funciones:** AntigravityAdapter::detect, plan_plugin_artifact, apply_vendor_install, verify_vendor_receipt; plugin.json name/description y skills/<slug>/SKILL.md; agy plugin install <localpath>. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** antigravity_adapter fake ProcessPort confirma argv install/list/uninstall documentados, plugin name jameskills-<slug>, failure parcial y compensation condicionada a ownership.

**Archivos del incremento:**
- `crates/jameskills-infra/src/agents/antigravity.rs`
- `crates/jameskills-infra/src/agents/mod.rs`
- `crates/jameskills-infra/tests/antigravity_adapter.rs`
- `crates/jameskills-core/src/domain/agent.rs`

**Aceptación:**
- [ ] Scope plugin/user soportado y project standalone Unsupported; no usar .agent del IDE para CLI.
- [ ] Plan muestra comando vendor y destino ~/.gemini/antigravity-cli/plugins/<name> comprobado por versión/plataforma.
- [ ] Artifact stage/receipt permiten compensating uninstall seguro, sin borrar plugin ajeno ni asumir rename transaccional vendor.

**Verificación:** cargo test -p jameskills-infra --locked antigravity_adapter; integración opt-in agy real install/list/uninstall de plugin aislado, comprobar cancel/error y cleanup por receipt.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C011 — Checkpoint tras T031–T033

- [ ] **C011 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- OpenCode/Pi/Antigravity CLI usan rutas/overrides/formatos oficiales y capability gates.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t034"></a>

## T034 — Implementar Grok Build CLI y GROK_HOME

- [ ] **T034 completada y verificada**

**Módulo:** `agent-adapters`. **Dependencias:** T029, T017. **Estado:** pendiente.

**Implementación y funciones:** GrokAdapter::detect, inspect_capabilities, resolve_grok_home, plan_artifact; grok version, grok inspect --json, repo .grok/skills y user GROK_HOME/skills o ~/.grok/skills. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** grok_adapter JSON inspect inválido, CLI diferente, GROK_HOME override/userprofile y unknown feature no se interpretan como soporte universal.

**Archivos del incremento:**
- `crates/jameskills-infra/src/agents/grok.rs`
- `crates/jameskills-infra/src/agents/mod.rs`
- `crates/jameskills-infra/src/platform.rs`
- `crates/jameskills-infra/tests/grok_adapter.rs`

**Aceptación:**
- [ ] Detección confirma Grok Build CLI con contrato oficial, no inventa grok code binario.
- [ ] Scopes/formato/upstream skills se preservan y rutas absolutas resueltas por OS.
- [ ] Capability no acreditada permanece Unsupported con fuente/recheck, mientras skills documentadas funcionan.

**Verificación:** cargo test -p jameskills-infra --locked grok_adapter; smoke opt-in Grok real inspect/descubrimiento de fixture aislado, sin publicar marketplaces.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t035"></a>

## T035 — Planear y aplicar instalaciones con journal

- [ ] **T035 completada y verificada**

**Módulo:** `agent-adapters`. **Dependencias:** T012, T013, T029, T030, T031, T032, T033, T034, T037, T038. **Estado:** pendiente.

**Implementación y funciones:** InstallService::plan_install, apply_install, recover_install; hashes/base revision/ownership, safe staging, journal y commit/rollback; dispatcher de vendor plugin. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** install_transaction colisión ajena, symlink destination, stale plan y crash antes/después rename conservan archivos; vendor fail ejecuta compensación segura.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/install.rs`
- `crates/jameskills-core/src/application/mod.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/src/composition.rs`
- `crates/jameskills-infra/tests/install_transaction.rs`

**Aceptación:**
- [ ] Dry run completo existe antes de todo write y apply revalida plan/version/revisión/destino.
- [ ] Journal fsync/atomic primitives por plataforma permiten recuperación sin afirmar atomicidad vendor inexistente.
- [ ] Receipts registran artifact hash/owner/version/scopes y no datos secretos; cancel no deja instalación declarada completa.

**Verificación:** cargo test -p jameskills-infra --locked install_transaction en ambos OS con failure injection; si dryrun/apply requieren más archivos, dividir T035.a planificación y T035.b journal/apply, conservando deps.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t036"></a>

## T036 — Actualizar y retirar instalaciones propias con seguridad

- [ ] **T036 completada y verificada**

**Módulo:** `agent-adapters`. **Dependencias:** T035, T009. **Estado:** pendiente.

**Implementación y funciones:** plan_upgrade, plan_remove, apply_remove, agents_detect_command, install_plan_command, install_apply_command; receipt/modified destination conflict. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** install_lifecycle archivo modificado manualmente o receipt faltante impide eliminación; update/remove por vendor solo sobre plugin propio comprobado.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/install.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-cli/src/commands.rs`
- `crates/jameskills-infra/tests/install_lifecycle.rs`
- `crates/jameskills-cli/tests/install_command.rs`

**Aceptación:**
- [ ] CLI expone agents detect e install plan/apply/remove con plan ID vigente y JSON.
- [ ] Upgrade conserva cambios ajenos y rollback vuelve al artifact propio anterior.
- [ ] Desinstalar no toca suite canonical/library ni agent settings ajenos; receipt/evidence se actualizan tras verificar.

**Verificación:** cargo test -p jameskills-infra --locked install_lifecycle; cargo test -p jameskills-cli --locked install_command; smoke real opt-in de ciclo para cinco agentes en scopes soportados.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C012 — Checkpoint tras T034–T036

- [ ] **C012 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Grok y lifecycle install transaction/upgrade/remove preservan contenido ajeno en ambos OS.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t037"></a>

## T037 — Crear SQLite y migraciones transaccionales

- [x] **T037 completada y verificada**

**Módulo:** `skill-library`. **Dependencias:** T007, T010, T013. **Estado:** completada.

**Implementación y funciones:** StoragePort, SqliteStore::open, migrate, with_transaction; tablas skill/revision/blob refs/bindings/receipts/snapshot DAG según arquitectura, índices y schema version. StoragePort introduce ahora solo métodos/tipos usados por storage real; capture_snapshot/merge_snapshot se añaden en T059/T061 cuando DTO/providers existen, con firmas finales CONTRACTS. Nunca success placeholder. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** sqlite_migrations fresh DB/upgrade supported, DB locked/corrupt/schema futuro; rollback en failure conserva DB anterior y diagnóstico saneado.

**Archivos del incremento:**
- `crates/jameskills-core/src/ports/storage.rs`
- `crates/jameskills-core/src/ports/mod.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-infra/src/lib.rs`
- `crates/jameskills-infra/tests/sqlite_migrations.rs`

**Descomposición obligatoria y wiring adicional:**
- [x] **T037.dep — Fijar proveedor SQLite** (3 archivos): `crates/jameskills-infra/Cargo.toml`; `Cargo.lock`; `docs/SOURCES.md`. Pin exacto `rusqlite = 0.40.2` con `bundled`, sin SQLite del sistema ni extensiones; registrar fuente/licencia/MSRV.
- Evidencia T037.dep: índice crates.io `yanked=false`; `cargo check` compiló `sqlite3.c` vía cc (MSVC local, objeto generado); CI linux+windows verde con el proveedor. Commit `6c0324c` (squash PR #13, incluye T037.a).
- [x] **T037.a — SQL migrations** (4 archivos): `crates/jameskills-infra/migrations/001_library.sql`; `crates/jameskills-infra/migrations/002_operations.sql`; `crates/jameskills-infra/migrations/003_sync.sql`; `crates/jameskills-infra/tests/sqlite_migrations.rs`. Esquema exacto ARCHITECTURE, user_version y rollback; sqlite_migrations tests se escriben antes del código b.
- Evidencia T037.a: RED 6/6 por archivos ausentes. GREEN 6/6 Windows local (11 tablas exactas, rerun idempotente, FK rechaza revisión huérfana pero permite parent ausente por diseño, archivos sin `PRAGMA user_version`, corrupto rechazado sin pánico, columnas de revisions/deletions fijadas). Remoto PR #13 9/9. Commit `6c0324c` (squash PR #13).
- [x] **T037.b — StoragePort y actor SQLite** (5 archivos): `crates/jameskills-core/src/ports/storage.rs`; `crates/jameskills-core/src/ports/mod.rs`; `crates/jameskills-infra/src/sqlite.rs`; `crates/jameskills-infra/src/lib.rs`; `crates/jameskills-infra/tests/sqlite_migrations.rs`. Single writer, WAL/foreign_keys/busy_timeout 5s. No global connection ni sqlite en UI render.
- Evidencia T037.b: RED E0432 en port y actor. GREEN 12/12 Windows local (pragmas WAL/FK/busy verificados, upgrade v1 con backup y datos, esquema futuro bloqueado read-only sin escrituras, rollback, corrupto rechazado, commit persistente). Core queda libre de tipos rusqlite; el port expone solo `schema_version`/`check_integrity` y el actor `open`/`with_transaction`. Remoto PR #15 9/9. Commit `08f4ddd` (squash PR #15).

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [x] Migrations versionadas hacen backup antes de mutación irreversible y no se ejecutan en render UI.
- [x] Foreign keys/transacciones/constraints preservan IDs y refs sin truncamiento.
- [x] Version futura se abre bloqueada/read-only según spec, nunca downgraded a fuerza.

**Verificación:** cargo test -p jameskills-infra --locked sqlite_migrations con DB temporal en Linux y Windows; registrar journal/WAL y estrategia de locks.

**Evidencia al ejecutar:** T037.dep/a/b completos: SQLite bundled versionado; 11 tablas/migrations; actor single-writer con WAL, FK, busy timeout, backup antes upgrade, rollback e incompatibilidad futura read-only. Windows local 6/6 migrations + 12/12 actor; CI remota PR #13 9/9 y PR #15 9/9. Squashes `6c0324c` y `08f4ddd`.

<a id="t038"></a>

## T038 — Persistir blobs y revisiones inmutables

- [x] **T038 completada y verificada**

**Módulo:** `skill-library`. **Dependencias:** T037, T013, T012. **Estado:** completada.

**Implementación y funciones:** SkillRevision, RevisionId, store_bundle_blob, commit_revision, verify_blob; hash+size+parent causal y commit DB/FS recuperable. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** revision_storage blob existente/corrupto, DB failure tras stage y crash entre blob commit/DB no crean head inválida ni pérdida de revisión previa.

**Archivos del incremento:**
- `crates/jameskills-core/src/domain/library.rs`
- `crates/jameskills-core/src/domain/mod.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-infra/tests/revision_storage.rs`

**Aceptación:**
- [x] Blobs/revisiones inmutables y referenciados por hash; commit acepta expected_heads esperada (base T038, PR #17).
- [x] Orphans locales quedan identificados para recuperación segura; no borrar parent aún usado.
- [x] Reabrir biblioteca verifica consistencia con errores y recuperación, sin sobrescribir contenido.

**Verificación:** cargo test -p jameskills-infra --locked revision_storage con failure injection; comprobar manifest/hash roundtrip frente T013.

**Evidencia al ejecutar:** RED inicial E0432/E0599 para DTO/blob/commit; T038.a RED E0599 para `orphan_blob_hashes`. GREEN 14/14 `revision_storage` y 60/60 infra Windows; hash mismatches y commits sin blob rechazados; huérfanos inventariados/preservados/reintentables; rollback mantiene head anterior; re-open rechaza referencias ausentes/corruptas; parents y tombstone observed-heads deben cubrir las heads vigentes. Workspace core+infra+CLI tests, desktop check, Clippy, fmt y diff check verdes. PR #17 base `c81fa73` 9/9; PR #18 `8389058` con CI Windows/Linux 9/9.

### T038.a — Verificar y recuperar el almacén de blobs

- [x] **T038.a completada y verificada**

**Dependencias:** T038 base (PR #17). **Archivos (4):** `crates/jameskills-infra/src/fs.rs`, `crates/jameskills-infra/src/sqlite.rs`, `crates/jameskills-infra/tests/revision_storage.rs`, `tasks/todo.md`.

**RED primero:** blobs huérfanos identificables sin borrado automático; al reabrir, referencias ausentes/corruptas se informan y bloquean escrituras; reintento de commit puede reutilizar blob verificado. Fallo transaccional no publica revisión ni destruye una revisión anterior.

**Aceptación:** inventario separa blobs referenciados y huérfanos; apertura verifica cada blob referenciado y no limpia bytes automáticamente; recuperación segura es repetible/no destructiva y ninguna revisión parent se borra.

**Verificación:** `cargo test -p jameskills-infra --locked --test revision_storage`; luego suite infra, clippy y fmt. Registrar RED/GREEN y ejecución Windows + CI Linux.

**Evidencia al ejecutar:** RED `cargo test -p jameskills-infra --locked --test revision_storage` falló con E0599 porque faltaba `orphan_blob_hashes`. GREEN 14/14 focused Windows; `cargo test -p jameskills-infra --locked` 60/60; workspace Clippy `-D warnings`, fmt y `git diff --check` limpios. Incluye blob hash mismatch rechazado, huérfano listado/preservado/reutilizado tras rollback, y re-open bloquea referencias ausentes o corruptas. CI Linux/Windows PR #18 9/9.

<a id="t039"></a>

## T039 — Consultar catálogo, búsqueda y paginación

- [ ] **T039 completada y verificada**

**Módulo:** `skill-library`. **Dependencias:** T037, T038.a. **Estado:** pendiente.

**Implementación y funciones:** LibraryService::list_skills, search_skills, load_skill, load_history; filtros/sort/page cursor estables sin cargar blobs completos. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** library_queries dataset grande, query unicode/metacaracteres, deleted/conflicted skills y página vacía no duplican/omiten filas.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/library.rs`
- `crates/jameskills-core/src/application/mod.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-infra/src/composition.rs`
- `crates/jameskills-infra/tests/library_queries.rs`

**Aceptación:**
- [ ] Queries parametrizadas y límites; no SQL de texto de policies/usuario.
- [ ] Paginación estable con IDs y catálogo refleja latest heads sin ocultar conflictos.
- [ ] Leer detalles/historial bajo demanda y exponer immutable view models.

**Verificación:** cargo test -p jameskills-infra --locked library_queries; dataset de referencia se comparte con T071 sin benchmarks artificiales de UI.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C013 — Checkpoint tras T037–T039

- [ ] **C013 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Migrations/blobs/revisions/query conservan consistencia y heads.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t040"></a>

## T040 — Crear y guardar suites desde casos de uso

- [ ] **T040 completada y verificada**

**Módulo:** `skill-library`. **Dependencias:** T039, T011, T014. **Estado:** pendiente.

**Implementación y funciones:** LibraryService::create_skill, save_draft, publish(SaveRevisionRequest); drafts invalid permitidos, autosave500ms, publish validado con expected_heads, no-op idempotente y bump semver tras contenido publicado. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** library_authoring draft inválido se conserva, publish inválido no reemplaza head; no-op idempotente; changed content misma semver falla; dos editores de same heads producen Conflict.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/library.rs`
- `crates/jameskills-core/src/domain/library.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-infra/tests/library_authoring.rs`
- `crates/jameskills-cli/src/commands.rs`

**Aceptación:**
- [ ] Crear/guardar pasa siempre por validador y StoragePort, no writes directas desde editor.
- [ ] Guardar draft tolera errores sin publicar; Publish/Install/Export-suite exige válido/reviewed según operación; semver bump y estados distintos.
- [ ] Servicios quedan en factory y CLI library list/import placeholders se reemplazan donde corresponde, sin success stub.

**Verificación:** cargo test -p jameskills-infra --locked library_authoring; probar crear suite mínima y editar instrucciones/policies conservando historial.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t041"></a>

## T041 — Gestionar assets y referencias como datos

- [ ] **T041 completada y verificada**

**Módulo:** `skill-library`. **Dependencias:** T040, T012, T038. **Estado:** pendiente.

**Implementación y funciones:** add_asset, replace_asset, remove_asset, rename_bundle_path, preview_asset; permisos/limites/hash y nuevo revision. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** library_assets ruta traversal, archivo enorme, referencia rota, rename collision Windows y symlink rechazan; no ejecutar script agregado.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/library.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/tests/library_assets.rs`
- `crates/jameskills-core/src/domain/skill.rs`

**Aceptación:**
- [ ] Mutaciones mantienen inventario/hash y referencias o muestran error explícito.
- [ ] Previews son de formatos soportados como datos; binarios no se ejecutan.
- [ ] Atomic save/revision y undo/cancel no mutan última versión válida.

**Verificación:** cargo test -p jameskills-infra --locked library_assets con fixtures portables; revisar archivo de ejecución agregado nunca lanzó proceso.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t042"></a>

## T042 — Importar suites y resolver IDs/versiones duplicados

- [ ] **T042 completada y verificada**

**Módulo:** `skill-library`. **Dependencias:** T040, T041, T014. **Estado:** pendiente.

**Implementación y funciones:** LibraryService::import_bundle; helpers preview_import/apply_import y library_import_command; scan secrets Gitleaks/quarantine/review explícito, plain SKILL.md crea manifest draft solo instrucciones. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** library_import duplicate exact dedup; same ID distinto contenido/version y simultaneous edit no sobrescriben; malformed archive rollback intacto.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/library.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-infra/tests/library_import.rs`
- `crates/jameskills-cli/src/commands.rs`

**Aceptación:**
- [ ] Preview enumera IDs/version/assets/policies/conflicts y bytes antes de aplicar.
- [ ] Import crea revisiones/transacciones y requiere base state vigente.
- [ ] CLI library import --path usa mismo servicio; scanner ausente permite cuarentena pero bloquea Reviewed/install/cloud. .jskill sin DAG crea raíz concurrente, no parents fabricados.

**Verificación:** cargo test -p jameskills-infra --locked library_import; invocar CLI real import con fixture válida y corrupta en data dir temporal.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C014 — Checkpoint tras T040–T042

- [ ] **C014 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Authoring/assets/import validan, versionan y no sobrescriben concurrent edits.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t043"></a>

## T043 — Exportar suites desde biblioteca y CLI

- [ ] **T043 completada y verificada**

**Módulo:** `skill-library`. **Dependencias:** T042, T009. **Estado:** pendiente.

**Implementación y funciones:** LibraryService::export_bundle(ExportRequest), library_export_command; head/revision explícita, portable .jskill y overwrite preview; CLI --skill UUID --output path --json. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** library_export roundtrip de revisión específica conserva bundle hash; conflicto sin revision selection no elige head arbitraria; destination collision requiere decisión.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/library.rs`
- `crates/jameskills-cli/src/commands.rs`
- `crates/jameskills-cli/src/output.rs`
- `crates/jameskills-infra/tests/library_export.rs`
- `crates/jameskills-cli/tests/library_export_command.rs`

**Aceptación:**
- [ ] Artifact export portable carece DB/OAuth/paths privados y abre con herramientas estándar.
- [ ] Overwrite/cancel/failure preservan archivo existente.
- [ ] CLI library export y GUI usan mismo servicio; formato de archive documentado.

**Verificación:** cargo test -p jameskills-infra --locked library_export; cargo test -p jameskills-cli --locked library_export_command; validate export y comparar hash canonical.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t044"></a>

## T044 — Historial, rollback, fork y tombstones causales

- [ ] **T044 completada y verificada**

**Módulo:** `skill-library`. **Dependencias:** T038, T040, T042. **Estado:** pendiente.

**Implementación y funciones:** LibraryService::delete_skill; helpers list_revisions/restore_revision_as_new/fork_skill/restore_deleted_skill; RevisionKind::Tombstone descendiente de observed heads. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** library_history rollback crea nueva revisión con parent actual; fork nuevo UUID; library_tombstones stale edit no revive borrado y delete vs concurrent edit produce conflicto.

**Archivos del incremento:**
- `crates/jameskills-core/src/domain/library.rs`
- `crates/jameskills-core/src/application/library.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-core/tests/library_history.rs`
- `crates/jameskills-infra/tests/library_tombstones.rs`

**Aceptación:**
- [ ] No reescribir historial ni borrar blobs necesarios para backup/conflict.
- [ ] Soft delete/restore explícitos se modelan en graph y catalog.
- [ ] Acciones requieren expected_heads y son reversibles mediante nueva revision, no reloj last-wins.

**Verificación:** cargo test -p jameskills-core --locked library_history; cargo test -p jameskills-infra --locked library_tombstones; revisar grafo esperado tras delete/recovery.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t045"></a>

## T045 — Vincular repositorios y perfiles a suites

- [ ] **T045 completada y verificada**

**Módulo:** `policy-engine`. **Dependencias:** T039, T044, T025. **Estado:** pendiente.

**Implementación y funciones:** bind_repository, list_bindings, evaluate_binding; configuración local paths/profile/strict con suite revision y fingerprint de entorno. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** repository_bindings repo movido/WSL distinto/base revision antigua no se reutiliza como evidence válida; binding no se exporta en bundle ni backup.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/library.rs`
- `crates/jameskills-core/src/application/policy.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-infra/tests/repository_bindings.rs`
- `crates/jameskills-core/src/domain/library.rs`

**Aceptación:**
- [ ] Bindings locales referencian suite/heads sin alterar canonical portable.
- [ ] Rechecks usan facts vigentes del repo y profile Rust/Node soportado explícito.
- [ ] Binding perdido/offline produce pasos y conserva resultados previos marcados obsoletos.

**Verificación:** cargo test -p jameskills-infra --locked repository_bindings; abrir mismo skill con repos distintos y comparar guidance y provenance.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C015 — Checkpoint tras T043–T045

- [ ] **C015 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Export/history/tombstones/bindings locales preservan portable bytes y causalidad.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t046"></a>

## T046 — Conectar biblioteca GPUI con catálogo real

- [ ] **T046 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T008, T039, T040, T042, T043, T044. **Estado:** pendiente.

**Implementación y funciones:** LibraryView::new/render, handle_search/create/import/export/delete, reduce_library_event; list virtualizada/paginada y selectors de revision. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** library_flow navegar/buscar y aplicar evento stale preserva selección; import conflict y errores muestran panel persistente; acciones invocan servicios reales.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/views/library.rs`
- `crates/jameskills-desktop/src/views/mod.rs`
- `crates/jameskills-desktop/src/state.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-desktop/tests/library_flow.rs`

**Aceptación:**
- [ ] Lista/empty/search/create/import/export/history/delete son acciones reales con estados documentados.
- [ ] No leer disco/SQL en render; page/filter requests coalesced y catalog counters actualizados.
- [ ] Conflicted/deleted items y catálogo vacío permiten recuperar/crear, no CTA sin handler.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked library_flow; smoke GUI con biblioteca temporal real: crear/importar/buscar/exportar/delete/restore, verificar disco y DB.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t047"></a>

## T047 — Conectar editor, assets, políticas e historial

- [ ] **T047 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T046, T041, T044, T015. **Estado:** pendiente.

**Implementación y funciones:** SkillEditorView, DraftState, library.save_draft/publish; autosave500ms, validate, browse_revision/assets. Watcher externo con debounce produce conflicto en dirty draft, no reload silencioso. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** editor_flow guardar texto invalid/current revision conflict y cambiar ruta dirty no pierde draft ni sobrescribe head; asset invalid no queda publicado.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/views/skill_editor.rs`
- `crates/jameskills-desktop/src/views/mod.rs`
- `crates/jameskills-desktop/src/state.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-desktop/tests/editor_flow.rs`

**Aceptación:**
- [ ] Editor muestra instructions/metadata/policies/assets/guidance/history y errores en ubicación.
- [ ] Draft dirty/save/cancel/version/fork tienen ruta de servicio real y conflicto visible.
- [ ] InputState/Subscription retenidos, autosave solo “Guardado” tras persistencia; falla save ofrece retry+export draft emergency; watcher externo no destruye draft.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked editor_flow; GUI real editar suite, asset y requisito, guardar revisión, volver a historial y reabrir app.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t048"></a>

## T048 — Renderizar checks y alcance de enforcement

- [ ] **T048 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T045, T046, T028. **Estado:** pendiente.

**Implementación y funciones:** ChecksView, RequirementCard, run_checks_action, show_check_evidence, choose_binding/profile; summary required/recommended y authority. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** checks_flow Unknown/Blocked/Unsupported nunca dibujan pass badge; cambios repo/suite invalidan evidence; recheck actualiza solo request vigente.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/views/checks.rs`
- `crates/jameskills-desktop/src/components/requirement_card.rs`
- `crates/jameskills-desktop/src/components/mod.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-desktop/tests/checks_flow.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T048.a — RequirementCard y registro de componentes** (3 archivos): `crates/jameskills-desktop/src/components/requirement_card.rs`; `crates/jameskills-desktop/src/components/mod.rs`; `crates/jameskills-desktop/tests/requirement_card.rs`. Tests requirement_card para estados/authority/labels, no color-only.
- [ ] **T048.b — Checks view y bridge** (4 archivos): `crates/jameskills-desktop/src/views/checks.rs`; `crates/jameskills-desktop/src/views/mod.rs`; `crates/jameskills-desktop/src/bridge.rs`; `crates/jameskills-desktop/tests/checks_flow.rs`. Conectar handlers al servicio; panel evidence persistente y run/cancel/retry reales.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Pantalla enseña scope/advisory/hook/CI/host y provenance con timestamp.
- [ ] Filtros/severity/evidence/error/retry funcionan y no ocultan fallos obligatorios.
- [ ] Run tests es acción explícita con progreso/cancel; render/selección no ejecutan tooling.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked checks_flow; verificar repo fixture con cada estado y comparar JSON CLI del mismo service.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C016 — Checkpoint tras T046–T048

- [ ] **C016 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Library/editor/checks UI envían comandos reales y muestran todos estados.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t049"></a>

## T049 — Integrar asistente de requisitos y previews

- [ ] **T049 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T048, T025, T026, T027. **Estado:** pendiente.

**Implementación y funciones:** GuidanceView/OnboardingView, render_guidance_step, preview_step_action, recheck_step; pasos derivados del plan y capability facts. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** guidance_flow usuario pulsa Continuar sin cumplir requisito conserva pendiente; acción bloqueada no ejecuta; recheck después cambio de facts cambia pasos.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/views/onboarding.rs`
- `crates/jameskills-desktop/src/components/operation_preview.rs`
- `crates/jameskills-desktop/src/views/checks.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-desktop/tests/guidance_flow.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T049.a — Preview reusable** (3 archivos): `crates/jameskills-desktop/src/components/operation_preview.rs`; `crates/jameskills-desktop/src/components/mod.rs`; `crates/jameskills-desktop/tests/operation_preview.rs`. Preview render de diff/argv/impact+digest; stale plan deshabilita apply.
- [ ] **T049.b — Onboarding y guía** (5 archivos): `crates/jameskills-desktop/src/views/onboarding.rs`; `crates/jameskills-desktop/src/views/mod.rs`; `crates/jameskills-desktop/src/views/checks.rs`; `crates/jameskills-desktop/src/bridge.rs`; `crates/jameskills-desktop/tests/guidance_flow.rs`. Dynamic guide y recheck; no avanzar por confirmación manual sin condición de success.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Asistente adapta OS/stack/tools/auth/permissions y enlaza fuentes oficiales relevantes.
- [ ] Preview diff/argv/consecuencia precede acción invasiva y revalidación evita stale apply.
- [ ] Accesos faltantes dan guía concreta y permiten biblioteca offline; no popups de aprobación de fase.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked guidance_flow; recorrido real Git missing/path custom/README absent/host permission missing y recheck.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t050"></a>

## T050 — Integrar detección e instalación de cinco agentes

- [ ] **T050 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T036, T046, T049. **Estado:** pendiente.

**Implementación y funciones:** AgentsView con install.detect_agents/plan_install/apply_install/remove_installation; handlers scope/revision/preview/digest y artifact ownership; Upgrade capability gated. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** install_flow capability Unsupported desactiva acción con razón; colisión/stale plan no invoca apply; fail muestra recovery/receipt y no installed badge.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/views/agents.rs`
- `crates/jameskills-desktop/src/views/mod.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-desktop/src/components/operation_preview.rs`
- `crates/jameskills-desktop/tests/install_flow.rs`

**Aceptación:**
- [ ] Cinco tarjetas/perfiles muestran CLI/version/scopes/sources/destination real y refresh.
- [ ] Plan informa changes/check phase y ownership; Reviewed obligatorio, Pre-install strict según policy, pre-release no bloquea install sin declaración; runtime discovery no disponible se muestra NeedUserVerification.
- [ ] Install strict utiliza checks requeridos vigentes y no acepta Unknown; receipts se muestran solo tras verify.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked install_flow; GUI real cinco perfiles, con binarios ausentes y disponibles; proyecto Antigravity Unsupported explícito, CLI plugin user validado.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t051"></a>

## T051 — Implementar header binario y KDF limitados

- [ ] **T051 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T007, T012, T038. **Estado:** pendiente.

**Implementación y funciones:** BackupHeaderV1::parse/encode, derive_wrapping_key, validate_crypto_limits; header176 bytes y WrapAAD/PayloadAAD exactos de SPEC-cloud-sync. Header: JSKSBK01, version u16be, cipher/kdf u8, m/t/p u32be, salt16, vaultUUID16, snapshotUUID16, wrapnonce24, payloadnonce24, wrappedmaster48, ciphertext_len u64be. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** crypto_header offsets/endian/vectors fijos, magic/version/cipher/kdf/length alterados, KDF hostile y truncated headers fallan antes de KDF o alloc gigante.

**Archivos del incremento:**
- `crates/jameskills-core/src/ports/crypto.rs`
- `crates/jameskills-core/src/ports/mod.rs`
- `crates/jameskills-infra/src/crypto.rs`
- `crates/jameskills-infra/src/lib.rs`
- `crates/jameskills-infra/tests/crypto_header.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T051.a — DTOs sync y SecretInput controlado** (5 archivos): `crates/jameskills-core/src/domain/sync.rs`; `crates/jameskills-core/src/domain/mod.rs`; `crates/jameskills-core/src/ports/secrets.rs`; `crates/jameskills-core/src/ports/mod.rs`; `crates/jameskills-core/tests/secret_input.rs`. SecretInput Debug redacted/no Serialize/zeroize; tipos payload/IDs según contrato, no fake snapshot validation.
- [ ] **T051.b — CryptoPort/header/KDF** (5 archivos): `crates/jameskills-core/src/ports/crypto.rs`; `crates/jameskills-core/src/ports/mod.rs`; `crates/jameskills-infra/src/crypto.rs`; `crates/jameskills-infra/src/lib.rs`; `crates/jameskills-infra/tests/crypto_header.rs`. Header parse+limits antes derivación; vectors/tamper y raw byte offsets exactos. CryptoPort síncrono CPU usa spawn_blocking fuera core.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Header JSKSBK01 version1: offsets y AAD verificados; params fijos m65536/t3/p1/salt16.
- [ ] Límites se validan antes Argon2id, IO/decompresión y no dependen de metadata de atacante.
- [ ] No serde JSON para header crypto, no own cipher/KDF y librerías fijadas/revisadas.

**Verificación:** cargo test -p jameskills-infra --locked crypto_header; contrastar byte-vector de SECURITY con roundtrip y tamper tests, sin cuentas remotas.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C017 — Checkpoint tras T049–T051

- [ ] **C017 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Guía/agents UI y header176/KDF gates probados; secrets no UI state persistente.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t052"></a>

## T052 — Cifrar y autenticar snapshots completos

- [ ] **T052 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T051, T014, T038. **Estado:** pendiente.

**Implementación y funciones:** CryptoProvider::create_vault/unlock_vault/open_with_vault/seal/open según CryptoPort final. UnlockedVault privado contiene vaultID/master32/wrappingkey32/salt16/params; KDF produce wrappingkey, passphrase zeroized después. Nonces24 fresh, wrapperAAD por snapshot, CSPRNG failure aborta. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** crypto_snapshot wrong passphrase y tamper de cada field/wrappedkey/payload/RNG fail; cifrar dos snapshots usa nonces frescos y distinto ciphertext; retry mismo snapshot conserva encrypted bytes originales. Cross-vault/salt/keyslot mismatch rechazados; open_with_vault verifica IDs/auth y KDF no se repite cuando cache material coincide.

**Archivos del incremento:**
- `crates/jameskills-infra/src/crypto.rs`
- `crates/jameskills-core/src/ports/crypto.rs`
- `crates/jameskills-infra/tests/crypto_snapshot.rs`
- `crates/jameskills-infra/Cargo.toml`

**Aceptación:**
- [ ] WrapAAD=header[0..120]+header[168..176]; PayloadAAD=header176 y master/key-slot vault según SPEC-cloud-sync; rotation crea vault+master nueva.
- [ ] Passphrase/keys viven mínimo y never log/serialize en DB; nonce24 separado para wrap/payload.
- [ ] Payload auth failure uniforme/saneado y no plaintext temp persistente antes de validar flujo.

**Verificación:** cargo test -p jameskills-infra --locked crypto_snapshot; vectores/roundtrips con crypto library real, límite256MiB y memory bound.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t053"></a>

## T053 — Gestionar keyring, sesión de vault y lock

- [ ] **T053 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T052, T004. **Estado:** pendiente.

**Implementación y funciones:** SecretStorePort, KeyringSecretStore, SyncService::unlock y VaultSession::lock; cache opt-in material versionado vaultID/master32/wrappingkey32/salt16/params, nunca passphrase. SecretInput zeroize + Debug REDACTED. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** keyring_session vault locked/unavailable/write denied produce capability y nunca plaintext fallback; lock cancela jobs seguros y limpia key material. Cache contiene master+wrappingkey+salt+vaultID versionados; no password. Material de vault/salt/keyslot distinto falla; locking cero ambos keys y passphrase ya descartada.

**Archivos del incremento:**
- `crates/jameskills-core/src/ports/secrets.rs`
- `crates/jameskills-core/src/ports/mod.rs`
- `crates/jameskills-infra/src/keyring.rs`
- `crates/jameskills-infra/src/lib.rs`
- `crates/jameskills-infra/tests/keyring_session.rs`

**Aceptación:**
- [ ] Tokens y optional cache key material solo keyring opt-in revocable; passphrase nunca persistida ni cacheada. Cache scoped vault/keyslot y constructor controlado.
- [ ] Keyring ausente bloquea conexión cloud persistente; biblioteca offline y backup cifrado manual con passphrase transitoria siguen disponibles.
- [ ] Lock/shutdown limpia llave/sesión; no promete cifrado de fuentes locales. Disconnect limpia OAuth local y revocación es acción aparte; no borra backup.

**Verificación:** cargo test -p jameskills-infra --locked keyring_session con fake; prueba real read/write/delete de dato descartable keyring por OS, sin token real en evidencia.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t054"></a>

## T054 — Asistente de provisioning OAuth Desktop

- [ ] **T054 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T025, T053. **Estado:** pendiente.

**Implementación y funciones:** OAuthClientConfig::validate, plan_google_provisioning, probe_oauth_configuration; client_id público tipo desktop, drive.appdata y loopback oficial. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** oauth_provisioning client_id absent/wrong type/settings invalid no deja conectar; un tick manual no confirma API/consent/client válidos.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/guidance.rs`
- `crates/jameskills-core/src/domain/guidance.rs`
- `crates/jameskills-infra/src/google/oauth.rs`
- `crates/jameskills-infra/src/google/mod.rs`
- `crates/jameskills-infra/tests/oauth_provisioning.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T054.a — Config y módulo OAuth** (4 archivos): `crates/jameskills-infra/src/google/oauth.rs`; `crates/jameskills-infra/src/google/mod.rs`; `crates/jameskills-infra/src/lib.rs`; `crates/jameskills-infra/tests/oauth_provisioning.rs`. Config pública validada y registry google; no credential secret server hardcode.
- [ ] **T054.b — Guía de provisioning** (3 archivos): `crates/jameskills-core/src/application/guidance.rs`; `crates/jameskills-core/src/domain/guidance.rs`; `crates/jameskills-core/tests/google_guidance.rs`. DAG console/consent/testusers/Drive/API/desktopclient/recheck; fuente oficial.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Guía identifica proyecto Cloud Console, habilitar Drive API, client Desktop, consent/test users y scope mínimo.
- [ ] Client ID público configurable; no exige distribuir un client_secret de servidor como secreto nativo.
- [ ] Setup muestra qué puede verificar y qué requiere Console/browser; puede recheck y continuar offline.

**Verificación:** cargo test -p jameskills-infra --locked oauth_provisioning; docs oficiales exactas, plantilla config pública y gates; no crear proyecto Google por llamada no autorizada.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C018 — Checkpoint tras T052–T054

- [ ] **C018 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Cifrado/keyring/provisioning tienen vectores, fallos seguros y guía real.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t055"></a>

## T055 — Implementar autorización OAuth PKCE loopback

- [ ] **T055 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T054, T053. **Estado:** pendiente.

**Implementación y funciones:** begin_authorization, start_loopback_receiver, exchange_code; state/PKCE-S256, system browser, listener 127.0.0.1 port efímero timeout. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** oauth_pkce state mismatch, duplicate callback, wrong host/path, cancelled browser, expired auth y incorrect verifier se rechazan.

**Archivos del incremento:**
- `crates/jameskills-infra/src/google/oauth.rs`
- `crates/jameskills-infra/tests/oauth_pkce.rs`
- `crates/jameskills-infra/Cargo.toml`
- `crates/jameskills-core/src/ports/remote.rs`

**Aceptación:**
- [ ] OAuth solo scope drive.appdata, redirect loopback/documentado; no embedded webview ni auth por password.
- [ ] Listener acotado cierra al terminar y callback no logs code/token.
- [ ] Token exchange TLS endpoint oficial, timeout/retry seguro y errors typed sin secret leakage.

**Verificación:** cargo test -p jameskills-infra --locked oauth_pkce con HTTP/clock fake; contrato opt-in real inicia browser/cancela sin writes Drive antes de autorización válida.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t056"></a>

## T056 — Persistir refresh tokens y manejar reauth/revoke

- [ ] **T056 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T055, T053. **Estado:** pendiente.

**Implementación y funciones:** refresh_access_token, revoke_connection, bind_account_session, OAuthState; concurrencia single-flight refresh y scopes exactos. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** oauth_lifecycle invalid_grant/401/403, revocación, refresh racing y account binding switch no mezclan tokens/vaults ni loop infinito.

**Archivos del incremento:**
- `crates/jameskills-infra/src/google/oauth.rs`
- `crates/jameskills-infra/src/keyring.rs`
- `crates/jameskills-infra/tests/oauth_lifecycle.rs`
- `crates/jameskills-core/src/domain/sync.rs`
- `crates/jameskills-core/src/domain/mod.rs`

**Aceptación:**
- [ ] Refresh token en keyring/access RAM; SyncService::disconnect cancela y limpia binding; revocar ofrece acción separada, nunca borrar backup.
- [ ] UI etiqueta asociación local, no afirma email obtenido con solo drive.appdata.
- [ ] Reauth preserva biblioteca/vault local y backups; error de cuenta distinto se decide mediante discovery de vault, no email supuesto.

**Verificación:** cargo test -p jameskills-infra --locked oauth_lifecycle; contrato real opt-in connect/refresh/disconnect con cuenta prueba y logs redacted.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t057"></a>

## T057 — Implementar cliente Drive appDataFolder

- [ ] **T057 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T056, T022. **Estado:** pendiente.

**Implementación y funciones:** DriveSnapshotClient::list_files/upload/download, DriveFileMetadata; APIv3 spaces=appDataFolder, parent appDataFolder, fields mínimos/pageToken. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** drive_transport múltiples páginas, duplicate ID, truncation, 401/403/429/5xx/cancel; bounded download y retry jitter fake clock no duplican éxito.

**Archivos del incremento:**
- `crates/jameskills-infra/src/google/drive.rs`
- `crates/jameskills-infra/src/google/mod.rs`
- `crates/jameskills-core/src/ports/remote.rs`
- `crates/jameskills-infra/tests/drive_transport.rs`
- `crates/jameskills-infra/Cargo.toml`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T057.a — RemoteSnapshotPort** (3 archivos): `crates/jameskills-core/src/ports/remote.rs`; `crates/jameskills-core/src/ports/mod.rs`; `crates/jameskills-core/tests/remote_contract.rs`. Object-safe async list_page/download/upload; typed bounded file IDs/data; sin HTTP/reqwest en core.
- [ ] **T057.b — Drive transport** (4 archivos): `crates/jameskills-infra/src/google/drive.rs`; `crates/jameskills-infra/src/google/mod.rs`; `crates/jameskills-infra/tests/drive_transport.rs`; `crates/jameskills-infra/Cargo.toml`. Real APIv3 appDataFolder; retry ambiguous re-list/download/compare; no reseal al reintentar UUID.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] List recorre ALL pages y upload crea objeto inmutable, sin head mutable ni CAS inventado.
- [ ] Filename/metadata no contienen skill names ni datos personales; endpoint no configurable por suite.
- [ ] Timeout/backoff/retry-after/size/cancellation verificados y cuenta/prefix no cruzados.

**Verificación:** cargo test -p jameskills-infra --locked drive_transport; contrato opt-in list/upload/download ciphertext descartable en appDataFolder y cleanup explícito del objeto de prueba autorizado.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C019 — Checkpoint tras T055–T057

- [ ] **C019 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- OAuth lifecycle y Drive paginado cuentan con tests y contrato opt-in distinguido.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t058"></a>

## T058 — Modelar snapshots, DAG y vault discovery

- [ ] **T058 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T056, T051, T014, T044. **Estado:** pendiente.

**Implementación y funciones:** SnapshotId, VaultId, SnapshotPayload, parse_snapshot, validate_snapshot_graph, discover_vaults; ZIP snapshot.json+bundles/<hash>.jskill y límite agregado. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** snapshot_graph cycles/missing parents/two roots vaults/same UUID different hash; snapshot_payload known nesting only, hash mismatch y secrets/drafts/receipts prohibited.

**Archivos del incremento:**
- `crates/jameskills-core/src/domain/sync.rs`
- `crates/jameskills-core/tests/snapshot_graph.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/tests/snapshot_payload.rs`
- `crates/jameskills-core/src/domain/library.rs`

**Aceptación:**
- [ ] Snapshot completo retiene revisiones/blobs/tombstones y parents causales, no solo current heads.
- [ ] Vaults concurrent init se ofrecen para selección explícita, sin fusionar llaves/cuentas automáticamente.
- [ ] Headers/metadata remotos se verifican contra payload IDs/hash; datos locales paths/drafts/receipts jamás exportados.

**Verificación:** cargo test -p jameskills-core --locked snapshot_graph; cargo test -p jameskills-infra --locked snapshot_payload; adversarial payloads no llegan al commit local.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t059"></a>

## T059 — Capturar snapshot consistente de biblioteca

- [ ] **T059 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T058, T038, T044, T052. **Estado:** pendiente.

**Implementación y funciones:** SyncService::sync_once captura vía StoragePort::capture_snapshot; helpers serialize_snapshot_payload/seal_snapshot con generation consistente y blob hashes. Gitleaks pre-upload/no secrets gate no eludible por ignore. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** snapshot_capture save concurrent durante capture no genera mix inválido; missing/corrupt blob no produce backup completo falso. Detectado secret o scanner requerido ausente no produce cloud write; draft/evidence/receipts/tokens excluidos.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/sync.rs`
- `crates/jameskills-core/src/application/mod.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/tests/snapshot_capture.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T059.a — Captura tipada en StoragePort** (3 archivos): `crates/jameskills-core/src/ports/storage.rs`; `crates/jameskills-core/src/domain/sync.rs`; `crates/jameskills-core/tests/snapshot_capture_contract.rs`. Añadir capture_snapshot()->SnapshotPayload solo ahora, tras DTO T051.a y RevisionRecord T013.a. Snapshot coherente/generation/secrets exclusion tests, no success mock en provider.
- [ ] **T059.b — Captura/crypto/usecase real** (5 archivos): `crates/jameskills-core/src/application/sync.rs`; `crates/jameskills-core/src/application/mod.rs`; `crates/jameskills-infra/src/sqlite.rs`; `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/snapshot_capture.rs`. Impl SQLite capture y service snapshot/seal; tests con SQLite/FS/crypto reales, lock generation y backup pre-upload scan.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Snapshot incluye toda revisión retenida y dependencia causal con generación local determinada.
- [ ] Lectura/stream/ZIP bounded y ciphertext cache paths no contienen plaintext bundle.
- [ ] Cambio concurrente mantiene Pending; secret scanning gate bloquea upload y facilita remediar archivo, sin registrar contenido.

**Verificación:** cargo test -p jameskills-infra --locked snapshot_capture; decrypt snapshot fixture y comparar library graph/blobs, verificar ausencia de secrets/paths/receipts.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t060"></a>

## T060 — Subir snapshots inmutables con journal de sync

- [ ] **T060 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T057, T059, T053. **Estado:** pendiente.

**Implementación y funciones:** sync_upload, resume_upload, record_remote_receipt; UUID/ciphertext hash, journal generation y remote_file_id por account/vault. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** sync_upload timeout después create, retry duplicate remoto y crash antes persistir receipt convergen por snapshot UUID+hash; no sobrescriben otro archivo.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/sync.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-infra/src/google/drive.rs`
- `crates/jameskills-infra/src/composition.rs`
- `crates/jameskills-infra/tests/sync_upload.rs`

**Aceptación:**
- [ ] Upload idempotencia lógica por immutable object y receipts, acepta duplicados equivalentes sin pérdida.
- [ ] No borrar/reemplazar snapshot anterior ni GC automático; account binding validado.
- [ ] Cambio local durante upload conserva pending y no pierde receipt al cerrar vista.

**Verificación:** cargo test -p jameskills-infra --locked sync_upload con HTTP fake y fault injection; integración Drive real opt-in comprueba objeto cifrado recuperable.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C020 — Checkpoint tras T058–T060

- [ ] **C020 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Payload completo, snapshot consistente y upload inmutable conservan ancestry y receipts.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t061"></a>

## T061 — Descargar, unir y reconciliar snapshots

- [ ] **T061 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T060, T058. **Estado:** pendiente.

**Implementación y funciones:** sync_download, merge_snapshot_graph, reconcile_library_heads, compute_sync_state; dedup UUID/hash y union de revisions/tombstones. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** sync_union two devices offline, lost page, duplicate files, clocks skewed, divergent skill versions y deletion vs stale edit conservan ambas ramas.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/sync.rs`
- `crates/jameskills-core/src/domain/sync.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/tests/sync_union.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T061.a — Commit de merge tipado** (3 archivos): `crates/jameskills-core/src/ports/storage.rs`; `crates/jameskills-core/src/domain/sync.rs`; `crates/jameskills-core/tests/merge_contract.rs`. Añadir merge_snapshot(MergePlan)->MergeResult cuando tipos de dominio graph/revisions ya existen. expected generation/heads y validate-before-commit.
- [ ] **T061.b — Descarga/reconciliación real** (5 archivos): `crates/jameskills-core/src/application/sync.rs`; `crates/jameskills-core/src/domain/sync.rs`; `crates/jameskills-infra/src/sqlite.rs`; `crates/jameskills-infra/src/fs.rs`; `crates/jameskills-infra/tests/sync_union.rs`. SQL transaction/FS staging+domain merge completo; fakes HTTP etiquetados, no mock de storage/crypto.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Pull valida auth/hash/schema/graph en quarantine antes de transaction.
- [ ] Union no decide por timestamp ni usa last writer; head única solo por ancestralidad.
- [ ] Parents pendientes se reportan y los corruptos no dañan library; relist no promete atomic snapshot del Drive folder.

**Verificación:** cargo test -p jameskills-infra --locked sync_union; fixture determinista dos perfiles/drive fake y contrato real opt-in dos equipos/account vault común.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t062"></a>

## T062 — Resolver conflictos mediante revisión explícita

- [ ] **T062 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T061, T040. **Estado:** pendiente.

**Implementación y funciones:** domain::resolve_heads y LibraryService::publish con parents todos heads; helper plan_conflict_resolution para choose/merge/fork/delete y expected_heads. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** conflict_resolution stale heads/third device new branch rechaza plan; choose/merge/delete preserve ancestry y assets; conflictos no se cierran por reloj.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/sync.rs`
- `crates/jameskills-core/src/domain/sync.rs`
- `crates/jameskills-core/src/application/library.rs`
- `crates/jameskills-core/tests/conflict_resolution.rs`
- `crates/jameskills-infra/tests/conflict_commit.rs`

**Aceptación:**
- [ ] Diff identifica instrucciones/metadata/policies/assets y acciones preservan copies necesarias.
- [ ] Resolución crea nueva revision causal y snapshot pending; no borra branches históricas.
- [ ] Conflicto de vault/key diferente se resuelve eligiendo vault o import explícito, no mezclando ciphertext.

**Verificación:** cargo test -p jameskills-core --locked conflict_resolution; cargo test -p jameskills-infra --locked conflict_commit; validar graph converge tras sync posterior.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t063"></a>

## T063 — Restaurar con preview, recovery y transacción

- [ ] **T063 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T061, T062, T042. **Estado:** pendiente.

**Implementación y funciones:** preview_restore, apply_restore, recover_restore; quarantine auth/schema/hash/graph, diff vigente, export recovery actual y journal. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** restore_transaction wrong passphrase/corrupt/truncated/huge snapshot no writes; crash mitad commit/restart conserva última library consistente.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/sync.rs`
- `crates/jameskills-infra/src/fs.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `crates/jameskills-infra/tests/restore_transaction.rs`
- `crates/jameskills-core/src/domain/sync.rs`

**Aceptación:**
- [ ] Preview enumera cambios/conflicts/deletes y expected generation evita stale apply.
- [ ] Backup recovery local antes de restore; commit no restaura installations/settings locales.
- [ ] Restore clean profile con passphrase verifica toda revisión/blob sin tener keyring/tokens del equipo origen.

**Verificación:** cargo test -p jameskills-infra --locked restore_transaction con faults en cada checkpoint y verificación antes/después; GUI wiring en T067.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C021 — Checkpoint tras T061–T063

- [ ] **C021 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Union/conflicts/restore rechazan corrupción y preservan datos con failure injection.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t064"></a>

## T064 — Exportar backup portable, recuperar y rotar passphrase

- [ ] **T064 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T063, T009, T052, T053. **Estado:** pendiente.

**Implementación y funciones:** Backup local helpers + CLI export/restore; SyncService::plan_remote_reset/apply_remote_reset y rotación nuevo vault/key. Passphrase oculta fuera argv; confirm-digest de preview vigente. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** backup_recovery perfil limpio sin keyring recupera con passphrase+envelope; passphrase nueva crea backups nuevos y antiguas copias siguen requerir la vieja. Recuperar tras perder keyring usa password+envelope, crea UnlockedVault con wrappingkey para seal nuevos snapshots; cached master solo no cuenta como sesión completa.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/sync.rs`
- `crates/jameskills-cli/src/commands.rs`
- `crates/jameskills-infra/src/crypto.rs`
- `crates/jameskills-infra/tests/backup_recovery.rs`
- `crates/jameskills-cli/tests/backup_command.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T064.a — Export/recovery/CLI** (5 archivos): `crates/jameskills-core/src/application/sync.rs`; `crates/jameskills-cli/src/commands.rs`; `crates/jameskills-infra/src/crypto.rs`; `crates/jameskills-infra/tests/backup_recovery.rs`; `crates/jameskills-cli/tests/backup_command.rs`. Manual encrypted export y restore preview/apply --confirm-digest; secret TTY. backup_command usa binario real.
- [ ] **T064.b — Rotación y reset remoto** (5 archivos): `crates/jameskills-core/src/application/sync.rs`; `crates/jameskills-core/src/ports/remote.rs`; `crates/jameskills-infra/src/google/drive.rs`; `crates/jameskills-infra/tests/remote_reset.rs`; `crates/jameskills-core/tests/vault_rotation.rs`. RemoteSnapshotAdminPort::delete_file; reset scoped current account/vault/known IDs tras recovery export verificado. AppPlan expiry/digest/expected-generation. Delete permanente, no trash/GC silencioso.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] Formato local/Drive mismo envelope portable con autenticación y límite.
- [ ] Rotación crea vault+master nuevos y reencrypt full snapshot; export antiguo sigue password antigua. Reset remoto permanente exige export encrypted válido y plan scoped file IDs/current vault.
- [ ] CLI export/restore preview/apply tienen plan ID y no guardan secret en history/log/env; no skip preview mediante success stub.

**Verificación:** cargo test -p jameskills-infra --locked backup_recovery; cargo test -p jameskills-cli --locked backup_command; recuperación manual en segunda instalación limpia Linux/Windows.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t065"></a>

## T065 — Programar sync durante app activa y cerrar limpio

- [ ] **T065 completada y verificada**

**Módulo:** `cloud-sync`. **Dependencias:** T060, T061, T053, T064. **Estado:** pendiente.

**Implementación y funciones:** SyncService::sync_once + helpers schedule/pause/shutdown; interval>=5min, debounce30s, single-flight account/vault y CLI sync status/run --json. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** sync_scheduler tick offline/locked/disconnected no dispara writes; concurrent save y sync/cancel/shutdown generan estado durable no success falso.

**Archivos del incremento:**
- `crates/jameskills-core/src/application/sync.rs`
- `crates/jameskills-infra/src/composition.rs`
- `crates/jameskills-cli/src/commands.rs`
- `crates/jameskills-infra/tests/sync_scheduler.rs`
- `crates/jameskills-cli/tests/sync_command.rs`

**Aceptación:**
- [ ] Sync periódico solo app activa y opción configurable; no daemon/task system no solicitado.
- [ ] Estado pending/syncing/synced/conflict/locked/error/reauth typed con generaciones.
- [ ] Shutdown concluye commit o journal recuperable y requests limitados; CLI usa mismo service.

**Verificación:** cargo test -p jameskills-infra --locked sync_scheduler; cargo test -p jameskills-cli --locked sync_command; clock fake avanza timers sin sleeps largos y CI no toca Drive real.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t066"></a>

## T066 — Integrar configuración Google, unlock y vault selection

- [ ] **T066 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T049, T056, T058, T065. **Estado:** pendiente.

**Implementación y funciones:** SyncSetupView, connect_google_action, unlock_vault_action, choose_vault_action, disconnect_action; SecretInput y lifecycle sensible. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** sync_setup_flow Google config absent, keyring unavailable, two vaults, wrong passphrase y callback cancel conservan offline library y secret fields vacíos al lock.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/views/sync.rs`
- `crates/jameskills-desktop/src/views/settings.rs`
- `crates/jameskills-desktop/src/views/mod.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-desktop/tests/sync_setup_flow.rs`

**Aceptación:**
- [ ] UI muestra provisioning/recheck/browser linking real y scope único sin email inventado.
- [ ] Vault selection explícita, opt-in cache llave y aviso de recuperación portable.
- [ ] No secret en UiEvent/debug/draft/log; keyring ausente Blocked cloud persistente con manual encrypted export/offline disponibles; etiqueta local no email.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked sync_setup_flow; recorrido real Google test account/browser y keyring por OS, confirmar contenido cifrado remoto.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C022 — Checkpoint tras T064–T066

- [ ] **C022 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Recovery portable, scheduler activo y setup GUI mantienen cuenta/vault/secrets separados.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t067"></a>

## T067 — Integrar progreso, conflictos y restauración GUI

- [ ] **T067 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T066, T062, T063, T064. **Estado:** pendiente.

**Implementación y funciones:** BackupView handlers sync.sync_once, library.publish para conflict resolution, sync.preview_restore/apply_restore; local export/recovery y remote reset plan/aprobación con digest. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** sync_flow restore stale plan/corrupt backup no aplica; choose branch crea nueva revision; cancel después commit muestra completion real y refresca.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/views/sync.rs`
- `crates/jameskills-desktop/src/components/conflict_dialog.rs`
- `crates/jameskills-desktop/src/components/mod.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-desktop/tests/sync_flow.rs`

**Aceptación:**
- [ ] Estado/progreso/last evidence/account local label/vault y retry persisten en panel.
- [ ] Diff/restore/recovery/wrong-passphrase/conflict no se reducen a toast ni accionan pérdida por reloj.
- [ ] Upload/download/restore reales usan servicios y UI refleja generation pending después save concurrent.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked sync_flow; dos perfiles reales offline divergent→sync→conflict→resolve→restore limpio, screenshots y hashes.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t068"></a>

## T068 — Completar settings y health de capacidades

- [ ] **T068 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T050, T066, T067, T026. **Estado:** pendiente.

**Implementación y funciones:** SettingsView apply_settings/probe_health/set_approved_tool_path; public config, theme/language/schedule/log retention; paths lectura + Abrir carpeta. Override de data root solo startup validado, sin live relocation. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** settings_flow tool path cambiado durante instalación invalida preview; WSL/arquitectura incompatible no mezcla paths; invalid schedule no persiste.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/views/settings.rs`
- `crates/jameskills-desktop/src/state.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-infra/src/platform.rs`
- `crates/jameskills-desktop/tests/settings_flow.rs`

**Aceptación:**
- [ ] Settings públicos se validan y guardan sin tokens/secretos; editar config no muta library.
- [ ] Health muestra origen/version/source/recheck y pasos específicos por OS.
- [ ] Español/mensajes ID; paths de app lectura, abrir carpeta con system opener validado; guía export/restore hacia perfil nuevo cerrado; no migrar DB abierta.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked settings_flow; reiniciar app confirma prefs y biblioteca offline independiente de Drive.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t069"></a>

## T069 — Cerrar concurrencia, cancelación y completions obsoletas

- [ ] **T069 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T047, T050, T067, T068. **Estado:** pendiente.

**Implementación y funciones:** BoundedBridge, coalesce_search, cancel_job, apply_event, shutdown_runtime; queue64, semáforos y activity durable por operación. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** async_lifecycle búsqueda rápida/ruta cerrada/save concurrent/request antiguo no actualiza vista errónea ni pierde receipt de mutación completada.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-desktop/src/state.rs`
- `crates/jameskills-desktop/src/composition.rs`
- `crates/jameskills-desktop/src/components/status_bar.rs`
- `crates/jameskills-desktop/tests/async_lifecycle.rs`

**Descomposición obligatoria y wiring adicional:**
- [ ] **T069.a — Status/activity componente** (3 archivos): `crates/jameskills-desktop/src/components/status_bar.rs`; `crates/jameskills-desktop/src/components/mod.rs`; `crates/jameskills-desktop/tests/activity_model.rs`. Activity registra receipt aunque cambie la route; tests status/error/committed cancel.
- [ ] **T069.b — Lifecycle async real** (4 archivos): `crates/jameskills-desktop/src/bridge.rs`; `crates/jameskills-desktop/src/state.rs`; `crates/jameskills-desktop/src/composition.rs`; `crates/jameskills-desktop/tests/async_lifecycle.rs`. Bounded queues/semáforos/WeakEntity/shutdown; no pérdida de mutations completadas.

Cerrar cada subtarea con prueba roja/verde y commit/evidencia. El listado anterior del padre es orientativo; esta descomposición contiene el presupuesto/wiring real. Las subtareas siguientes dependen de la anterior.

**Aceptación:**
- [ ] No blocking IO/mutex en GPUI render ni tasks infinitas; servicios construidos una vez.
- [ ] Cancel antes commit aborta y después commit muestra resultado real; bounded queue evita growth.
- [ ] SQLite/KDF/decompresión en worker adecuado y UI update vía WeakEntity/cx.notify/subscription retenida.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked async_lifecycle; stress GUI switching/running/cancel/close/reopen con journal y actividad comprobados.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C023 — Checkpoint tras T067–T069

- [ ] **C023 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Sync UI/settings/async lifecycle manejan errors/cancel/stale y commits durables.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

<a id="t070"></a>

## T070 — Completar teclado, foco, accesibilidad y tema

- [ ] **T070 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T069. **Estado:** pendiente.

**Implementación y funciones:** register_keybindings, focus_route, restore_dialog_focus, render_status_semantics; kit tokens, labels, icons semánticos y no color-only. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** keyboard_navigation tab order/modal escape/focus return/shortcuts validos; estados error/unknown accesibles y contrast themes cumplen objetivos GUI.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/theme.rs`
- `crates/jameskills-desktop/src/views/shell.rs`
- `crates/jameskills-desktop/src/components/requirement_card.rs`
- `crates/jameskills-desktop/src/routes.rs`
- `crates/jameskills-desktop/tests/keyboard_navigation.rs`

**Aceptación:**
- [ ] Cada flow obligatorio funciona con teclado y foco visible; modales no trap indefinidamente.
- [ ] Light/dark y escala DPI Windows/Linux no cortan controles/texto/íconos.
- [ ] Semántica/accessibility bridge GPUI soportada se comprueba; gaps upstream se documentan con workaround verificable.

**Verificación:** cargo test -p jameskills-desktop --features test-support --locked keyboard_navigation; auditoría nativa con teclado/lector disponible/DPI125%200%, screenshots y ratios según GUI/TESTING.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t071"></a>

## T071 — Medir presupuestos de rendimiento y optimizar cuellos reales

- [ ] **T071 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T039, T069, T070. **Estado:** pendiente.

**Implementación y funciones:** generate_reference_catalog, measure_native_scenario; dataset1000 skills32KiB, page50/debounce150ms; profiling release antes de cambio. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** Primero medir inicio, filtro, interacción/frames/RSS/validation; si objetivo incumplido escribir regression test/bench del cuello medido, no microbenchmark artificial.

**Archivos del incremento:**
- `crates/jameskills-desktop/src/views/library.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-infra/src/sqlite.rs`
- `scripts/benchmark-native.sh`
- `docs/PERFORMANCE-EVIDENCE.md`

**Aceptación:**
- [ ] Registrar hardware/OS/build release/dataset/p95 y comparar R01/R02 presupuestos, sin afirmar resultados no medidos.
- [ ] Lista virtualizada/cancel coalesce y memory bounded verificados; IO/crypto no render.
- [ ] Optimización conserva pruebas/funcionalidad y benchmarks reproducibles; Windows command equivalente documentado.

**Verificación:** cargo build -p jameskills-desktop --release --locked; bash scripts/benchmark-native.sh --dataset 1000. Windows ejecutar escenario equivalente descrito; core validation100 policies y 100MiB backup incluidos.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

<a id="t072"></a>

## T072 — Instrumentar diagnósticos locales sin contenido privado

- [ ] **T072 completada y verificada**

**Módulo:** `desktop-app`. **Dependencias:** T065, T069, T071. **Estado:** pendiente.

**Implementación y funciones:** init_diagnostics, redact_event, export_diagnostic_report; logs rotados bounded/opt-in export sin telemetría remota automática. Las APIs públicas siguen docs/CONTRACTS.md; nombres adicionales son helpers privados.

**Red primero:** diagnostic_redaction token/passphrase/auth code/skill content/paths/JSON unexpected no aparecen en logs; synthetic secrets sobreviven solo fixture en memoria.

**Archivos del incremento:**
- `crates/jameskills-infra/src/composition.rs`
- `crates/jameskills-desktop/src/bridge.rs`
- `crates/jameskills-cli/src/output.rs`
- `crates/jameskills-infra/tests/diagnostic_redaction.rs`
- `docs/DIAGNOSTICS.md`

**Aceptación:**
- [ ] Operation IDs/status/errors/performance útiles sin nombres/contenidos ni credential payload.
- [ ] Export ofrece preview/ubicación y no envía reporte por red.
- [ ] Debug/release/crash/error outputs mantienen same redaction y retention/límite documentados.

**Verificación:** cargo test -p jameskills-infra --locked diagnostic_redaction; generar cada error OAuth/crypto/process/sync y revisar output CLI/GUI/log artifact saneado.

**Evidencia al ejecutar:** pendiente. Registrar test rojo (comando/fallo esperado), verde (comando/n.º tests), build/manual, OS, commit y bloqueo saneado.

## C024 — Checkpoint tras T070–T072

- [ ] **C024 verificado**

- Ejecutar pruebas enfocadas y suite acumulada core/infra/CLI; desktop build/tests cuando su entorno esté disponible. Fmt/clippy aplicables sin esconder target fallido.
- Teclado/accessibility, performance p95 y diagnóstico privado tienen evidencia real.
- Revisar wiring/errores/secret handling/archivos tocados. Actualizar `tasks/RESUME.md` con próxima tarea elegible, evidencia y bloqueos. No requiere aprobación humana de fase.

**Evidencia:** pendiente. Un checkpoint con requisito nativo/account pendiente permanece sin marcar; seguir tareas independientes cuando el DAG lo permite.

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
