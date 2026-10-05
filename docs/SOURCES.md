# Fuentes y decisiones verificadas — JameSkills

Consulta inicial: 2026-10-02 UTC. Verificación de bootstrap T001: 2026-10-02 UTC en Linux x86_64. Estas fuentes describen contratos externos; la especificación de JameSkills es una propuesta propia. Aún no se ha compilado el workspace ni se ha abierto una ventana de aplicación.

## Plataforma nativa

| Fuente oficial consultada | Evidencia | Decisión |
|---|---|---|
| https://crates.io/api/v1/crates/gpui-kit | API: 0.7.0 publicada 2026-09-28, no retirada; 0.6.5 aparece retirada | Fijar gpui-kit = "=0.7.0"; guardar Cargo.lock al bootstrap |
| https://github.com/longbridge/gpui-kit/releases/tag/v0.7.0 | Release de la misma versión | Usar fuentes del tag; no mezclar documentación de main con APIs 0.6 |
| https://github.com/longbridge/gpui-kit/blob/v0.7.0/crates/kit/Cargo.toml | Default features component y assets; test-support opcional | Un solo facade de GUI; pruebas UI activan test-support |
| https://github.com/longbridge/gpui-kit/blob/v0.7.0/crates/kit/src/lib.rs | application(), init(), open_window(); open_window añade base::Root | Inicializar Kit una vez y crear contenido sin un segundo Root |
| https://github.com/longbridge/gpui-kit/blob/v0.7.0/Cargo.toml | GPUI snapshots fijados =0.3.7 y edición 2024 | Conservar alineación del facade; no añadir un GPUI independiente |
| https://gpui-kit.com/docs/installation | Windows 10+, MSVC/VS2022+CMake; Ubuntu24.04 packages; Vulkan+sesión gráfica; la página indica baseline Rust1.92 | La página refleja un baseline insuficiente para source v0.7.0; T001 verificó compilación con Rust1.95 por `cold_path` |
| https://gpui-kit.com/docs/assets/ | Assets debe registrarse; catálogo e iconos separados de componentes | Usar Assets del Kit y verificar nombres del catálogo fijado |
| https://github.com/longbridge/gpui-kit/tree/v0.7.0/examples/ai_recipes | Recetas compilables y tests retained state | Modelo de entidades/subscriptions y UI tests desde fuentes fijadas |

Directorios nativos: crate `directories = "=6.0.0"`, MIT OR Apache-2.0,
[`BaseDirs`](https://docs.rs/directories/6.0.0/directories/struct.BaseDirs.html)
usa XDG y la home estándar en Linux y Known Folder API en Windows. Se eligió
este proveedor para `config_dir`, `data_local_dir`, `cache_dir`; JameSkills solo
agrega su nombre de aplicación, no construye `C:\\Users` ni concatena `$HOME`.

Tipos compartidos T007: pines exactos `serde = 1.0.229` (derive y DTOs),
`thiserror = 2.0.21` (errores tipados), `uuid = 1.26.1` (IDs UUID v4),
`sha2 = 0.10.9` (digest SHA-256), `unicode-normalization = 0.1.25` (NFC), y
`chrono = 0.4.45` (reloj UTC del host). Referencias versionadas:
[`serde`](https://docs.rs/serde/1.0.229/serde/),
[`thiserror`](https://docs.rs/thiserror/2.0.21/thiserror/),
[`uuid`](https://docs.rs/uuid/1.26.1/uuid/),
[`sha2`](https://docs.rs/sha2/0.10.9/sha2/),
[`unicode-normalization`](https://docs.rs/unicode-normalization/0.1.25/unicode_normalization/),
[`chrono`](https://docs.rs/chrono/0.4.45/chrono/). Cargo.lock fija los checksums
resueltos. `PortablePath` exige NFC antes de aceptar, no normaliza bytes importados.

CLI T009: `clap = 4.5.60` (MIT OR Apache-2.0; derive parser/help con MSRV1.74)
y `serde_json = 1.0.149` (MIT OR Apache-2.0; wrapper JSON con schema explícito).
Referencias versionadas: [`clap`](https://docs.rs/clap/4.5.60/clap/),
[`serde_json`](https://docs.rs/serde_json/1.0.149/serde_json/). Las versiones
exactas y checksums están registradas en Cargo manifests y lockfile.

Skill format T010: `toml = 1.1.6` (TOML1.1, MIT OR Apache-2.0),
`semver = 1.0.28` (MIT OR Apache-2.0) y `serde-saphyr = 1.3.0`
(YAML1.2 Serde parser, MIT OR Apache-2.0, MSRV1.89). Fuentes versionadas:
[`toml`](https://docs.rs/toml/1.1.6+spec-1.1.0/toml/),
[`semver`](https://docs.rs/semver/1.0.28/semver/),
[`serde-saphyr`](https://docs.rs/serde-saphyr/1.3.0/serde_saphyr/). Se fijan
budgets de profundidad/eventos/scalars y cero aliases/anchors; Options rechaza
tags custom y merge keys, exige llaves duplicadas como error y desactiva snippets
en errores. El TOML usa structs Serde `deny_unknown_fields` y [extensions]
string-map explícito.

Portable path collision T012: `icu_casemap = 2.3.0` (Unicode-3.0 license,
MSRV1.88). `CaseMapper::new().fold_string` uses full locale-independent Unicode
case folding; normalize its result to NFC before comparing path keys. Fuente de
API versionada: [`icu_casemap::CaseMapper`](https://docs.rs/icu_casemap/2.3.0/icu_casemap/struct.CaseMapper.html).

Storage T037: `rusqlite = 0.40.2` (MIT), `default-features = false` con
`bundled`: SQLite se compila desde su fuente con cc, sin SQLite del sistema
ni extensiones cargables. Índice crates.io: `yanked=false`; `rust-version`
no declarado en el índice, compatibilidad con 1.95 probada por compilación
local (MSVC, objeto `sqlite3.o` generado) y CI. Referencia versionada:
[`rusqlite`](https://docs.rs/rusqlite/0.40.2/rusqlite/). El toolchain C lo
aporta VS2022 en Windows y cc en Linux; `load_extension` queda desactivado.

Process T016: `async-trait = 0.1.92` (MIT OR Apache-2.0, MSRV1.71) define el
port async sin runtime en core; `tokio = 1.53.1` (MIT, MSRV1.71) con `rt`
ejecuta IO bloqueante con `spawn_blocking`; `command-group = 5.0.1` (Apache-2.0
OR MIT, MSRV1.68) cancela grupos POSIX/Windows. Los tres crates.io APIs reportan
`yanked=false`; Tokio ya estaba resuelto en Cargo.lock por GPUI.
Su API `group_spawn` crea process groups Unix y Job Objects Windows;
`GroupChild::kill` termina el grupo. `wait_with_output` lee stdout antes que
stderr en Windows y puede bloquear, por lo que el adaptador drenará ambas pipes
concurrentemente.
Fuentes de versión/license/MSRV: [async-trait crates.io](https://crates.io/api/v1/crates/async-trait/0.1.92),
[`async-trait`](https://docs.rs/async-trait/0.1.92/async_trait/),
[`tokio` crates.io](https://crates.io/api/v1/crates/tokio/1.53.1),
[`Tokio spawn_blocking`](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html),
[`command-group` crates.io](https://crates.io/api/v1/crates/command-group/5.0.1),
[`command-group CommandGroup`](https://docs.rs/command-group/5.0.1/command_group/stdlib/trait.CommandGroup.html),
[`command-group GroupChild`](https://docs.rs/command-group/5.0.1/command_group/struct.GroupChild.html).

Comprobación local T001: `cargo info gpui-kit@0.7.0` descargó versión 0.7.0,
licencia Apache-2.0 y reportó `rust-version: unknown`. El índice sparse local de
Cargo indica 0.7.0 `yanked=false` y 0.6.5 `yanked=true`. El crate normalizado
fija `gpui = "=0.3.7"`, edición 2024, y su `src/lib.rs` exporta
`application`, `init`, `open_window`, `assets` y `component`; `open_window`
envuelve el contenido con `base::Root`. Tag v0.7.0 resuelto a
`0c830f4d257e69fdd17200650533ab4ca9a40cc0`. Evidencia detallada y límites de
plataforma en `docs/PLATFORM-EVIDENCE.md`. La prueba inicial de toolchain cambió
el baseline de 1.92 a 1.95. Rust oficial documenta
[`slice::as_array`](https://doc.rust-lang.org/1.95.0/std/primitive.slice.html#method.as_array)
estable desde 1.93.0 y
[`std::hint::cold_path`](https://doc.rust-lang.org/1.95.0/std/hint/fn.cold_path.html)
desde 1.95.0; `gpui-pre 0.3.7` llama a esta última. Pruebas de compilación
negativas en 1.92 y 1.94, positiva de `jameskills-desktop` en 1.95 con el modo
dlopen upstream de fontconfig están detalladas en `docs/PLATFORM-EVIDENCE.md`.

Revisión 2026-10-04: la página actual de installation ofrece v0.7.0 y cita Rust 1.92+ para su grafo vigente; el README publicado en el tag v0.7.0 todavía muestra `gpui-kit = "0.6"`, por lo que ese ejemplo no sirve para seleccionar la versión del producto. Para JameSkills prevalecen la dependencia exacta `gpui-kit = "=0.7.0"`, el Cargo.lock y la evidencia de compilación del snapshot `gpui-pre 0.3.7`: el workspace requiere Rust 1.95 por `std::hint::cold_path`, aunque la página de instalación cite 1.92. Las URLs de documentación `/versions/v0.7.0/docs/...` previamente consultadas devolvieron 404; firmas/APIs se verifican contra el tag y los crates versionados, no contra ejemplos ambiguos.

Cargo 1.95: [`cargo test --locked`](https://doc.rust-lang.org/1.95.0/cargo/commands/cargo-test.html) asegura que no cambie la resolución existente del lockfile; [`cargo clippy`](https://doc.rust-lang.org/1.95.0/cargo/commands/cargo-clippy.html) es un subcomando externo distribuido como componente del toolchain. CI y los comandos locales usan Rust/Cargo fijados en 1.95.0; el modo `--locked` no sustituye la ejecución de tests, lint ni build.

Runner de suites T020.c: Cargo 1.95 [`cargo metadata`](https://doc.rust-lang.org/1.95.0/cargo/commands/cargo-metadata.html) recomienda `--format-version 1`, define `workspace_members` y `packages[].targets[].test`, y `--no-deps` omite dependencias. El runner usa metadata `--no-deps --format-version 1 --locked --offline` como inspección declarativa y no invoca `cargo test` durante esa inspección. La ejecución aprobada usa [`cargo test`](https://doc.rust-lang.org/1.95.0/cargo/commands/cargo-test.html) con `--workspace --locked`: Cargo documenta que selecciona miembros del workspace y que `--locked` rechaza cambios a `Cargo.lock`; exit 0 es éxito de Cargo y exit 101 es fallo de Cargo. Un exit no nulo por sí solo no prueba que una prueba individual fallara ni distingue fallo de compilación de fallo del harness.

Licencias: software y ejemplos Kit Apache-2.0; prosa/ilustraciones originales de docs bajo CC BY4.0 según sitio. Este dossier resume requisitos, no copia ilustraciones. Conservar atribuciones de Lucide/Isocons y dependencias en THIRD-PARTY-NOTICES.

## Formato y agentes

| Fuente | Contrato observado |
|---|---|
| https://agentskills.io/specification | SKILL.md YAML + Markdown; name 1–64, minúsculas/dígitos/guiones sin extremos ni dobles; descripción 1–1024; metadata string map; carpetas libres |
| https://developers.openai.com/codex/skills | Repo .agents/skills entre cwd y root; usuario ~/.agents/skills; agents/openai.yaml opcional; carga progresiva |
| https://opencode.ai/docs/skills/ | Repo .opencode/skills; usuario ~/.config/opencode/skills; reconoce .agents; permisos pueden impedir carga |
| https://pi.dev/docs/latest/skills | Directorio SKILL.md portable; .pi/skills y .agents; /skill:name y /reload |
| https://pi.dev/docs/latest/configuration | PI_CODING_AGENT_DIR cambia ~/.pi/agent; skill-dir <agent-dir>/skills |
| https://antigravity.google/docs/cli/plugins | CLI agy plugin install/list/uninstall; plugin.json y skills/<name>/SKILL.md; perfil ~/.gemini/antigravity-cli/plugins |
| https://antigravity.google/docs/cli/install | Ejecutable agy; Windows user-local AppData/Local/agy/bin; Linux ~/.local/bin |
| https://docs.x.ai/build/cli/reference | grok version y grok inspect --json; no asumir --version |
| https://docs.x.ai/build/features/skills-plugins-marketplaces | .grok/skills, ~/.grok/skills y plugins; SKILL.md |
| https://docs.x.ai/build/settings | GROK_HOME cambia perfil; Windows usa USERPROFILE/.grok |

Versiones de agentes no fijadas aquí: no hay CLIs instaladas para confirmar un rango. Cada perfil debe registrar versión observada, fixture de discovery y fuentes; instalación se habilita al superar contrato local. La existencia de docs confirma producto/formato, no cualquier versión futura ni cualquier hook. Grok Code del brief se identifica con el CLI oficial Grok Build; no confundir un modelo xAI con un ejecutable alternativo de terceros.

## Drive, recuperación y autenticación

- https://developers.google.com/workspace/drive/api/guides/appdata
  Carpeta exclusiva y oculta appDataFolder; scope drive.appdata no sensible; parents=["appDataFolder"] al crear y spaces=appDataFolder al listar. No compartir/mover/mandar a papelera sus archivos. La carpeta se puede borrar desde administración de aplicaciones de Drive.
- https://developers.google.com/workspace/drive/api/reference/rest/v3/files/list
  Listado paginado con nextPageToken y fields explícitos. Confirmar durante integración parámetros de query/fields usados.
- https://developers.google.com/workspace/drive/api/reference/rest/v3/files/create
  Upload multipart; snapshot inmutable, sin asumir nombres únicos.
- https://developers.google.com/identity/protocols/oauth2/native-app
  Cliente Desktop público, navegador externo, PKCE, callback loopback, refresh tokens; credenciales y API se provisionan en Google Console.
- https://www.rfc-editor.org/rfc/rfc8252.html
  PKCE para clientes nativos y sin secreto confidencial distribuido.
- https://2fas.com/support/2fas-auth-mobile-app/how-does-google-drive-synchronization-work
  Inspiración: carpeta oculta, cuenta elegida, restore tras reinstalar, export adicional; contraseña de backup opcional en 2FAS.
- https://2fas.com/support/2fas-auth-security-privacy/can-i-access-my-google-drive-backup-file-if-i-dont-remember-the-password
  Pérdida de contraseña impide recuperar copia protegida.
- https://github.com/twofas/2fas-android
  Repositorio GPL3. Estudiar arquitectura no concede relicenciar código: JameSkills implementa protocolo propio sin copiar fuente GPL.

Drive no provee aquí un CAS verificado para un HEAD mutable. Se usa DAG de snapshots completos inmutables. No afirmar atomicidad multiarchivo ni sincronización exactamente una vez.

## Herramientas estándar y contratos a consultar al implementar

- Conventional Commits: https://www.conventionalcommits.org/en/v1.0.0/
- GitHub rulesets: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets
- REST rulesets: https://docs.github.com/en/rest/repos/rules
- Git hooks: https://git-scm.com/docs/githooks
- Git ignore probe: https://git-scm.com/docs/git-check-ignore
- Commitlint: https://commitlint.js.org/reference/cli.html
- Gitleaks: https://github.com/gitleaks/gitleaks
- cargo-deny: https://embarkstudios.github.io/cargo-deny/
- cargo-audit: https://rustsec.org/docs/
- RustCrypto AEAD: https://docs.rs/chacha20poly1305/
- Argon2: https://docs.rs/argon2/
- Keyring: https://docs.rs/keyring/
- GPUI headless recipes y APIs: fuente v0.7.0 indicada arriba.
- MSRV/toolchain: https://doc.rust-lang.org/cargo/reference/rust-version.html
- GitHub Actions permissions: https://docs.github.com/en/actions/security-for-github-actions/security-guides/automatic-token-authentication

## Guías oficiales de instalación para el registry de tools

Los IDs siguientes son app-owned y se usan como fuentes inertes de guía. Los
enlaces de probe/documentación describen comandos versionados; no autorizan a
descargar, instalar ni ejecutar herramientas automáticamente.

| Tool | Windows | Linux | Probe/documentación del comando |
|---|---|---|---|
| Git | [git-install-windows](https://git-scm.com/install/windows) | [git-install-linux](https://git-scm.com/install/linux) | [Git SCM](https://git-scm.com/docs/git) |
| Node.js | [node-install-windows](https://nodejs.org/en/download) | [node-install-linux](https://nodejs.org/en/download) | [npm: instalar y comprobar node/npm](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) |
| npm | [npm-install-windows](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) | [npm-install-linux](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) | [npm CLI](https://docs.npmjs.com/cli/v12/commands/npm) |
| Rust (`cargo`, `rustc`) | [rust-install-windows](https://rust-lang.github.io/rustup/installation/windows-msvc.html) | [rust-install-linux](https://rust-lang.github.io/rustup/installation/other.html) | [rustup](https://rustup.rs/) |
| GitHub CLI | [gh-install-windows](https://github.com/cli/cli/blob/trunk/docs/install_windows.md) | [gh-install-linux](https://github.com/cli/cli/blob/trunk/docs/install_linux.md) | [GitHub CLI](https://cli.github.com/) |
| Gitleaks | [gitleaks-install-windows](https://github.com/gitleaks/gitleaks#installing) | [gitleaks-install-linux](https://github.com/gitleaks/gitleaks#installing) | [Gitleaks releases](https://github.com/gitleaks/gitleaks/releases) |
| Commitlint | [commitlint-install-windows](https://commitlint.js.org/guides/getting-started.html) | [commitlint-install-linux](https://commitlint.js.org/guides/getting-started.html) | [Commitlint CLI](https://commitlint.js.org/reference/cli.html) |
| cargo-audit | [cargo-audit-install-windows](https://github.com/rustsec/rustsec/tree/main/cargo-audit) | [cargo-audit-install-linux](https://github.com/rustsec/rustsec/tree/main/cargo-audit) | [RustSec](https://rustsec.org/) |
| cargo-deny | [cargo-deny-install-windows](https://embarkstudios.github.io/cargo-deny/) | [cargo-deny-install-linux](https://embarkstudios.github.io/cargo-deny/) | [cargo-deny CLI](https://embarkstudios.github.io/cargo-deny/cli/index.html) |

Gitleaks v8.30.1 CLI contract: [README at tag v8.30.1](https://github.com/gitleaks/gitleaks/blob/v8.30.1/README.md) documents `dir`, `--config` precedence over repository `.gitleaks.toml`, default `useDefault` rules, redaction, JSON output, and exit-code override. Only 8.30.1 is currently accepted: it is the exact tag whose CLI and JSON fixture were reviewed; all other versions remain Blocked until separately evidenced. The [tagged `cmd/root.go`](https://github.com/gitleaks/gitleaks/blob/v8.30.1/cmd/root.go) also shows that `.gitleaksignore` is loaded from the scan source independently of `--config`; therefore the check blocks when that file exists rather than treating its suppressions as trusted. The repository test fixture is sanitized; findings values are never returned or logged.

Commitlint T020 source/API: [`@commitlint/cli` v21.2.2](https://github.com/conventional-changelog/commitlint/tree/v21.2.2/%40commitlint/cli) implements `--default-config` with built-in `@commitlint/config-conventional` and `--edit <file>` input. Its [`cli.ts`](https://github.com/conventional-changelog/commitlint/blob/v21.2.2/%40commitlint/cli/src/cli.ts) source confirms config-file discovery and exit behavior. JameSkills pins this driver to 21.2.2; the checker must run with an app-owned private cwd so repository Commitlint configs are not loaded. The official current CLI page reports 21.2.3, which is outside the verified pin and remains Blocked.

Commitlint v21.2.2 [`package.json`](https://github.com/conventional-changelog/commitlint/blob/v21.2.2/%40commitlint/cli/package.json) declares Node `>=22.12.0`. The app-owned general Node discovery range includes stable Node 18–24 (`<25`); the Commitlint driver separately enforces the package's Node minimum. The [Node release schedule](https://github.com/nodejs/Release/blob/main/schedule.json) records Node 24 as LTS through 2028-04-30; Node 25 is excluded until separately reviewed.

The Commitlint v21.2.2 [`load-config.ts`](https://github.com/conventional-changelog/commitlint/blob/v21.2.2/%40commitlint/load/src/utils/load-config.ts) uses `explorer.load(explicitPath)` when `--config` is supplied, rather than cosmiconfig search from cwd. Its [`get-edit-commit.ts`](https://github.com/conventional-changelog/commitlint/blob/v21.2.2/%40commitlint/read/src/get-edit-commit.ts) resolves the Git top-level from the CLI cwd before reading `--edit`. Therefore the Node driver keeps the OS process cwd private, passes the approved repository root as Commitlint `--cwd`, and supplies an absolute app-owned JSON config path; it does not execute repository Commitlint config files.

La guía de npm es fuente primaria para comprobar versiones mediante `node -v` y
`npm -v`; Commitlint documenta `--version`; Gitleaks documenta `version` y
`--version`. Para los subcomandos Cargo, sus README oficiales documentan su
instalación como Cargo subcommands. Los resultados de versión siguen siendo
Candidate hasta verificar otras capacidades.

### Parser Markdown para README checks

- `markdown = "=1.0.0"`: [documentación versionada](https://docs.rs/markdown/1.0.0/markdown/)
  y [`to_mdast`](https://docs.rs/markdown/1.0.0/markdown/fn.to_mdast.html).
  Devuelve AST CommonMark desde `&str`; los checks examinan headings y contenido
  visible, sin renderizar HTML ni ejecutar nodos.

Las fuentes de esta última sección son referencias de implementación, no todas APIs específicas fueron ejecutadas aquí. T001 registra versiones exactas; cada driver guarda help/source y fixtures antes de declarar compatibilidad. Si cambia un contrato, actualizar docs + tarea afectada + tests; no improvisar flags.
