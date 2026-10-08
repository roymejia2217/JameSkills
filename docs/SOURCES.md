# Fuentes y decisiones verificadas — JameSkills

Consulta inicial: 2026-10-02 UTC. Verificación de bootstrap T001: 2026-10-02 UTC en Linux x86_64. Estas fuentes describen contratos externos; la especificación de JameSkills es una propuesta propia. Checkpoints C014/C015 verificaron compilación y pruebas GPUI headless en Windows MSVC; no se afirma smoke visual nativo.

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
| https://crates.io/api/v1/crates/rfd/0.17.2 | rfd 0.17.2, MIT, edition 2021, yanked=false, checksum `20dafead71c16a34e1ff357ddefc8afc11e7d51d6d2b9fbd07eaa48e3e540220`; índice no declara MSRV | Pin exacto; compatibilidad se verifica con Rust1.95 en Windows/Linux |
| https://github.com/PolyMeilex/rfd/tree/0.17.2 | Tag 0.17.2 del selector nativo de archivos | Usar API AsyncFileDialog, nunca invocar shell/tool para abrir picker |
| https://docs.rs/rfd/0.17.2/rfd/struct.AsyncFileDialog.html | API versionada: `pick_file`, `pick_folder`, `save_file`, `set_file_name`, filtros; Windows/Linux/macOS | Acciones de UI esperan selección/cancel sin bloquear render |
| https://docs.rs/rfd/0.17.2/rfd/struct.FileHandle.html | `FileHandle::path()` devuelve ruta nativa en desktop | Pasar la ruta al preview/apply tipado; no abrir contenido del picker en el renderer |
| https://docs.rs/rfd/0.17.2/rfd/ | Default features `xdg-portal` + `wayland`; Linux portal usa backend del desktop, `libdbus`/Zenity como fallback según runtime | Linux package/runtime documenta portal y Zenity; no sustituir por GTK ni mock web |

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

MSVC child-process environment: Microsoft Learn [Build Tools from the command line](https://learn.microsoft.com/en-us/cpp/build/building-on-the-command-line?view=msvc-170) states that the command-line toolchain requires environment variables for executable, include, library, and SDK paths; it specifically lists `PATH`, `TMP`, `INCLUDE`, `LIB`, and `LIBPATH`, and recommends the installed developer command file because values vary by target and installation. The [CL environment-variable reference](https://learn.microsoft.com/en-us/cpp/build/reference/cl-environment-variables?view=msvc-170) documents `INCLUDE`/`LIBPATH` and confirms `CL`/`_CL_` inject compiler arguments. JameSkills therefore permits only named toolchain path variables in `ApprovedEnv`, never `CL` or `_CL_`.

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
| https://developers.openai.com/codex/build-skills | Repo `.agents/skills` desde CWD hasta el repo root; user `$HOME/.agents/skills`; `agents/openai.yaml` opcional; Codex CLI sigue symlinks al descubrir skills |
| https://developers.openai.com/codex/cli | Standalone installer Windows/macOS/Linux; `npm install -g @openai/codex` como alternativa; no fija una versión CLI en esta página |
| https://github.com/openai/codex/blob/main/codex-rs/cli/src/main.rs | Source mutable `main` declara Clap `version` y `bin_name="codex"`; respalda existencia del flag `--version`, no fija formato/versión de un release |
| https://opencode.ai/docs/skills/ | Repo .opencode/skills; usuario ~/.config/opencode/skills; reconoce .agents; permisos pueden impedir carga |
| https://opencode.ai/docs/config/ | Global config `~/.config/opencode`; documenta `OPENCODE_CONFIG_DIR` como custom directory |
| https://opencode.ai/docs/cli/ | Global `--version`/`-v`; referencia de comandos y env vars |
| https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/config/paths.ts | Source `dev` mutable: `OPENCODE_CONFIG_DIR` y `.opencode` directories entran en `Config.directories` |
| https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/skill/index.ts | Source `dev` mutable: busca `skills/**/SKILL.md` en config dirs y ancestros del worktree; upstream sigue symlinks |
| https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/index.ts | Source `dev` mutable: yargs define `--version` usando `InstallationVersion` |
| https://github.com/anomalyco/opencode/blob/dev/packages/core/src/global.ts | Source `dev` mutable: config root usa `xdg-basedir`; `OPENCODE_CONFIG_DIR` puede sustituir `Global.Path.config` |
| https://pi.dev/docs/latest/skills | Directorio SKILL.md portable; .pi/skills y .agents; /skill:name y /reload |
| https://pi.dev/docs/latest/configuration | PI_CODING_AGENT_DIR cambia ~/.pi/agent; skill-dir <agent-dir>/skills |
| https://pi.dev/docs/latest/cli | `pi --version` muestra la versión; CLI documenta `--version` y `-v` |
| https://pi.dev/docs/latest/environment-variables | `PI_CODING_AGENT_DIR` override del config dir; variables process-level |
| https://github.com/earendil-works/pi/blob/v1.0.4/packages/coding-agent/src/config.ts | Tag v1.0.4: `PI_CODING_AGENT_DIR`, default `.pi/agent`, `VERSION` desde package.json |
| https://github.com/earendil-works/pi/blob/v1.0.4/packages/coding-agent/src/main.ts | Tag v1.0.4: `pi --version` imprime `VERSION` raw antes de iniciar sesión |
| https://github.com/earendil-works/pi/blob/v1.0.4/packages/coding-agent/package.json | Tag v1.0.4: package `@earendil-works/pi-coding-agent`, version 1.0.4, bin `pi`, Node >=22.19 |
| https://github.com/earendil-works/pi/releases/tag/v1.0.4 | Release estable publicada 2026-10-05; incluye `pi-windows-x64.zip` con SHA-256 reportado por GitHub API `6bdbfb7bac252eea36a0095e4b741c9d5784d5ba99146e2d76e9246dee409b58` |
| https://antigravity.google/docs/plugins?tab=cli | CLI `agy plugin install/list/uninstall`; `plugin.json` + `skills/<name>/SKILL.md`; CLI profile `~/.gemini/antigravity-cli/plugins/<name>`; schema example uses `$schema` but formal schema omits it while closing `additionalProperties` |
| https://antigravity.google/docs/cli/install | Ejecutable agy; Windows user-local AppData/Local/agy/bin; Linux ~/.local/bin |
| https://docs.x.ai/build/cli/reference | grok version y grok inspect --json; no asumir --version |
| https://docs.x.ai/build/features/skills-plugins-marketplaces | .grok/skills, ~/.grok/skills y plugins; SKILL.md |
| https://docs.x.ai/build/settings | GROK_HOME cambia perfil; Windows usa USERPROFILE/.grok |

No hay rangos de versión CLI ni fixture de discovery ejecutados en este host. Cada perfil debe observar el executable seleccionado y probar discovery antes de marcar una capability Supported. Pi tiene un source fixture pinned v1.0.4 cuyo `--version` imprime `1.0.4`; eso no acredita que la CLI instalada en este host sea Pi. La página actual de Codex no fija su CLI version; el source upstream consultado pertenece a `main` mutable. El usuario informa que OpenCode y `agy` están instalados, pero sus versiones no se han observado en esta sesión. Grok Code del brief se identifica con el CLI oficial Grok Build; no confundir un modelo xAI con un ejecutable alternativo de terceros.

En Windows, el instalador standalone de Codex es el camino preferido para un executable nativo aprobado. La instalación npm también es oficial, pero puede exponer `codex.cmd`; JameSkills lo clasifica Candidate y no ejecuta shims hasta resolver un Node/entrypoint con identidad verificada.

### IDs de evidencia del registry de agentes

Estos IDs son aliases estables hacia las URLs de la tabla anterior; no significan que se haya probado una versión de CLI. La fecha en `CapabilityEvidence` registra la revisión local del perfil; mientras `tested_version`/fixture estén ausentes, la capability permanece `NeedsVerification` o `Unsupported`.

| Source ID | URL oficial asociada |
|---|---|
| `codex-skills` | https://developers.openai.com/codex/build-skills |
| `codex-cli-install` | https://developers.openai.com/codex/cli |
| `codex-cli-source` | https://github.com/openai/codex/blob/main/codex-rs/cli/src/main.rs |
| `opencode-skills` | https://opencode.ai/docs/skills/ |
| `opencode-cli-docs` | https://opencode.ai/docs/cli/ |
| `opencode-config-docs` | https://opencode.ai/docs/config/ |
| `opencode-config-source` | https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/config/paths.ts |
| `opencode-skills-source` | https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/skill/index.ts |
| `opencode-cli-source` | https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/index.ts |
| `opencode-global-source` | https://github.com/anomalyco/opencode/blob/dev/packages/core/src/global.ts |
| `pi-skills` | https://pi.dev/docs/latest/skills |
| `pi-cli-docs` | https://pi.dev/docs/latest/cli |
| `pi-agent-dir` | https://pi.dev/docs/latest/environment-variables |
| `pi-cli-source` | https://github.com/earendil-works/pi/blob/v1.0.4/packages/coding-agent/src/main.ts |
| `pi-cli-package-v1.0.4` | https://github.com/earendil-works/pi/blob/v1.0.4/packages/coding-agent/package.json |
| `pi-cli-release-v1.0.4` | https://github.com/earendil-works/pi/releases/tag/v1.0.4 |
| `antigravity-cli-plugins` | https://antigravity.google/docs/plugins?tab=cli |
| `antigravity-plugin-schema` | https://antigravity.google/docs/plugins?tab=cli |
| `antigravity-cli-install` | https://antigravity.google/docs/cli/install |
| `grok-cli-reference` | https://docs.x.ai/build/cli/reference |

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

- cap-std filesystem capabilities, pinned crate source `4.0.3`: https://docs.rs/cap-std/4.0.3/cap_std/fs/struct.Dir.html
  `Dir::open_ambient_dir` establishes the explicitly selected root; subsequent
  open/create/hard-link/remove operations accept paths relative to that handle.
  This is a containment boundary, not a promise that every internal symlink is
  rejected; callers still inspect each component and preserve create-only rules.
- Conventional Commits: https://www.conventionalcommits.org/en/v1.0.0/
- GitHub rulesets: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets
- REST rulesets: https://docs.github.com/en/rest/repos/rules
- Git hooks: https://git-scm.com/docs/githooks
- Git remotes: https://git-scm.com/docs/git-remote
- Git ignore probe: https://git-scm.com/docs/git-check-ignore
- Git repository-root probe: https://git-scm.com/docs/git-rev-parse
- Commitlint: https://commitlint.js.org/reference/cli.html
- Gitleaks: https://github.com/gitleaks/gitleaks
- cargo-deny: https://embarkstudios.github.io/cargo-deny/
- cargo-audit: https://rustsec.org/docs/
- RustCrypto AEAD: https://docs.rs/chacha20poly1305/
- Argon2: https://docs.rs/argon2/
- Argon2 KDF specification/vector: https://www.rfc-editor.org/rfc/rfc9106.txt (Argon2 version 0x13 and published Argon2id test vector, §5.3).
- Crypto T051 pins: `argon2 = 0.5.3` (MIT OR Apache-2.0, MSRV1.65, crates.io `yanked=false`, checksum `3c3610892ee6e0cbce8ae2700349fcf8f98adb0dbfbee85aec3c9179d29cc072`), `chacha20poly1305 = 0.10.1` (Apache-2.0 OR MIT, docs declare Rust1.56+, `yanked=false`, checksum `10cd79432192d1c0f4e1a0fef9527696cc039165d729fb41b3f4f4f354c2dc35`) and `zeroize = 1.9.0` (source above). Argon2 disables default PHC/password-hash/rand features and enables `zeroize`; provider supplies/zeroizes caller-owned blocks using explicit-memory API, with `alloc` disabled to avoid library-owned KDF memory. Chacha uses `alloc` + `getrandom` for fallible OS `OsRng` nonce generation. RustCrypto docs confirm `Params::new`, `Algorithm::Argon2id`, version v19, `hash_password_into_with_memory`, and XChaCha20Poly1305/XNonce 24-byte nonces. RFC9106 provides a separate Argon2id test vector. XChaCha follows SPEC-cloud-sync despite docs noting no final IETF standard; ciphertext AAD/offsets are JameSkills protocol, not crate defaults.
- Snapshot ZIP T052: `zip = 8.5.1`, MIT, MSRV1.88, crates.io `yanked=false`, checksum `dcab981e19633ebcf0b001ddd37dd802996098bc1864f90b7c5d970ce76c1d59`; versioned docs confirm `ZipWriter`, `ZipArchive`, Stored entries, and ZIP64. Pin disables all default features (AES/password encryption and compression codecs); only app-generated, bounded Stored entries are written/read. Paths/names, entry count, aggregate bytes and actual reads remain app-validated; no arbitrary extraction helper or filesystem path API is used.
- Secret memory clearing T051: [`zeroize 1.9.0`](https://docs.rs/zeroize/1.9.0/zeroize/) (MIT OR Apache-2.0, MSRV1.85). Its `Zeroize` implementation uses volatile writes/fences to avoid compiler elision; `ZeroizeOnDrop` is only a marker, so `SecretInput` implements Drop that calls `zeroize()` explicitly. The crate has no required dependencies; only core owns the passphrase bytes, with no serde/Debug exposure.
- Keyring: https://docs.rs/keyring/4.2.0/keyring/v1/ and https://docs.rs/keyring/4.2.0/keyring/v1/struct.Entry.html. Pin `keyring=4.2.0` (MIT OR Apache-2.0, MSRV1.88.0, crates.io release dated 2026-08-29, `yanked=false`, checksum `2270074a3d26bcac93c1dc5d2845eb4c089e8d761ccf6e0ea266a16004640627`); crate defaults are disabled and only `v1` is enabled. Its v1 `Entry` uses Windows Credential Manager and Secret Service on Linux; `store_status`, `set_secret`, `get_secret`, and `delete_credential` are the reviewed calls. Keyring I/O is synchronous and must run outside UI/render; no mock/default storage fallback is accepted for production.
- GPUI headless recipes y APIs: fuente v0.7.0 indicada arriba.
- MSRV/toolchain: https://doc.rust-lang.org/cargo/reference/rust-version.html
- GitHub Actions permissions: https://docs.github.com/en/actions/security-for-github-actions/security-guides/automatic-token-authentication
- GitHub Actions workflow syntax: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax
- GitHub Actions job matrices: https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/run-job-variations
- GitHub protected branches/status checks: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches
- GitHub ruleset rules: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets

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

Git's [`githooks` manual](https://git-scm.com/docs/githooks) specifies that hooks without the executable bit are ignored, `core.hooksPath` selects the hooks directory, and `commit-msg` may be bypassed with `--no-verify`. The [Husky v9 setup guide](https://typicode.github.io/husky/get-started.html) additionally documents disabling hooks with `HUSKY=0`. Therefore an installed hook is local feedback, not a security or merge boundary; inspecting config/file presence cannot prove invocation.

Commitlint's [local setup guide](https://commitlint.js.org/guides/local-setup.html) recommends a Husky `commit-msg` hook and explicitly says local linting can be tinkered with; its [CI setup guide](https://commitlint.js.org/guides/ci-setup.html) demonstrates validating commits in push/PR workflows. The live [`nodejs/node` commit-lint workflow](https://github.com/nodejs/node/blob/main/.github/workflows/commit-lint.yml) validates the PR's first commit message in a remote workflow and pins its validator release and action by SHA. Its [workflow directory](https://github.com/nodejs/node/tree/main/.github/workflows) separately contains Linux, Windows, and macOS test workflows. This is evidence for using local hooks as convenience and remote CI as enforcement, not a claim that Node validates every commit message.

GitHub's [protected-branch documentation](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches) states that required status checks must succeed before merging. Its [ruleset documentation](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets) describes layered active rulesets and their visible enforcement state. Workflow YAML and successful run evidence alone do not prove that the host requires the check.

For cross-platform CI patterns, the live [`microsoft/vscode` workflow directory](https://github.com/microsoft/vscode/tree/main/.github/workflows) has reusable Linux, Windows, and macOS test workflows; the platform workflows select OS-specific runners/shells and pin GitHub actions by commit SHA. The live [`rust-lang/rust` CI workflow](https://github.com/rust-lang/rust/blob/main/.github/workflows/ci.yml) and its [job definitions](https://github.com/rust-lang/rust/blob/main/src/ci/github-actions/jobs.yml) enumerate Linux, macOS, Windows, and architecture-specific jobs. These are observed repository implementations, not a universal prescription to run every check on every OS.

T021 CI-contract parser references: GitHub's [workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax) specifies event triggers, job IDs, permissions and workflow fields; its [matrix guide](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/run-job-variations) defines job matrix expansion. GitHub notes that a workflow skipped by filters can leave a required check pending. The [protected branch status-check rules](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches) describe host-side merge enforcement; local YAML inspection cannot establish that rule or a check run for the current SHA.

The T021 local workflow parser will use the already locked [`serde-saphyr` 1.3.0](https://docs.rs/serde-saphyr/1.3.0/serde_saphyr/) directly in infra with an explicit event/depth/node/scalar budget, duplicate-key errors, zero aliases/anchors, rejected custom tags/merge keys, and disabled snippets. YAML data remains inert; parser output is inspected as data and never executes workflow steps or Actions.

T021 host discrimination uses Git's [`remote -v` documentation](https://git-scm.com/docs/git-remote), which reports remote names and configured fetch/push URLs. This is local configuration only (no network request); URLs may contain credentials and must stay in bounded process memory, never evidence/log output. A GitHub Actions YAML file without an observed supported GitHub remote remains Unknown, not RequiredCi.

T022 GitHub CLI/API evidence is pinned to the installed, fingerprinted `gh` tool
profile (the current host was observed as `gh 2.102.0`; this is not a claim
about other releases). General tool discovery remains `>=2.0.0,<3.0.0`, while
the GitHub repository-evidence driver itself accepts only `2.102.0` until a
new release receives source/fixture review. The official [`gh auth status` manual](https://cli.github.com/manual/gh_auth_status)
documents `--hostname`, `--active`, and JSON `hosts`; importantly JSON mode
returns exit 0 even when auth is unhealthy. Never pass `--show-token`: it emits
the credential in text and JSON. The tagged [`v2.102.0 auth status source`](https://github.com/cli/cli/blob/v2.102.0/pkg/cmd/auth/status/status.go)
shows host entries expose state, active, login, tokenSource and scopes, with
token omitted unless explicitly requested. Parse only the fields required for
an app-owned auth observation; do not persist the login, token source, raw
error, scopes string, or raw command output. Scope labels are not a general
permission inventory: GitHub documents fine-grained permissions separately and
per endpoint.

The official [`gh` environment manual](https://cli.github.com/manual/gh_help_environment)
documents the Windows default auth/config location as `$AppData/GitHub CLI`
unless `GH_CONFIG_DIR` is set. JameSkills therefore allowlists only the
platform-provided `APPDATA` path on Windows; it does not pass `GH_CONFIG_DIR`
or auth/host override variables to the child.

The official [`gh api` manual](https://cli.github.com/manual/gh_api) documents
that requests are authenticated, that `--hostname` selects the host, and that
adding fields/input can change the default method to POST. The tagged
[`v2.102.0 api source`](https://github.com/cli/cli/blob/v2.102.0/pkg/cmd/api/api.go)
confirms these semantics and that `GH_HOST` can select a different host.
Therefore the driver must use an app-built fixed argv with explicit `--method
GET`, explicit `--hostname github.com`, registered API paths, no arbitrary
fields/input/pagination/verbose/cache, and an environment that excludes
`GH_HOST`, `GH_TOKEN`, `GITHUB_TOKEN`, enterprise-token overrides, and user
arguments. Repository owner/name are accepted only after parsing the approved
Git remote and validating bounded path components; they never supply scheme,
host, endpoint path, or flags. `gh` follows REST redirects; its tagged
[`AddAuthTokenHeader` source](https://github.com/cli/cli/blob/v2.102.0/api/http_client.go)
only adds the configured token on the original hostname, and explicitly omits
it if a redirect changes hostname. JameSkills does not consume redirect URLs
as subsequent endpoints and still validates the final typed repository
identity before reporting `repo-read`.

For identity, GitHub REST [`GET /repos/{owner}/{repo}`](https://docs.github.com/en/rest/repos/repos#get-a-repository)
returns repository `full_name`, `owner.login` and `name`; the driver should
compare those typed fields with the selected remote and discard unrelated
response fields (including URLs, descriptions and permission metadata). REST
authentication docs describe 401 invalid credentials and 403/404 for missing
permissions/private resources; therefore 404 cannot safely prove nonexistence.
Fine-grained permissions are endpoint-specific and may be reported in
`X-Accepted-GitHub-Permissions`; OAuth scope headers do not enumerate
fine-grained grants. Record only the response category and explicitly observed
capability, not inferred permissions from account identity or a token scope
string.

REST [`rate limits`](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api)
document primary 403/429 and `Retry-After`/`X-RateLimit-Remaining`/`Reset`
handling; secondary limits may also return 403/429. A read-only evidence driver
must perform a small serial request set with bounded output/time and no
automatic pagination. It may expose a bounded retry hint, but exhaustion,
malformed/missing rate headers or ambiguous 403 remains Blocked/Unknown, never
Pass. GitHub's [REST best practices](https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api)
also note that 404 can mask inaccessible private resources and that redirects
exist; this reinforces binding requests to the approved GitHub API host and
avoiding retry loops.

T024 release evidence (GitHub REST docs current on 2026-10-05):
- [`GET /repos/{owner}/{repo}/releases`](https://docs.github.com/en/rest/releases/releases#list-releases)
  returns releases, not unassociated Git tags. Public published releases are
  visible; drafts are listed only to users with push access. Entries expose
  `tag_name`, `draft`, `prerelease`, and assets with nullable `digest`; response
  sizes are bounded and the checker does not paginate. A full 100-entry page is
  treated as possibly truncated. The digest is GitHub-reported SHA-256 metadata;
  the checker does not download or independently hash asset bytes.
- [`GET /repos/{owner}/{repo}/releases/tags/{tag}`](https://docs.github.com/en/rest/releases/releases#get-a-release-by-tag-name)
  retrieves a published release by tag, not a bare Git ref. Therefore a tag
  existing without a release cannot satisfy a release requirement. 403/404 do
  not independently prove absence or permission; only a complete successful
  release listing can establish that a required published version is absent.
- GitHub [Git references](https://docs.github.com/en/rest/git/refs#get-a-reference)
  resolve `refs/tags/<tag>` to an object and identify lightweight versus
  annotated tags. The [Get a tag endpoint](https://docs.github.com/en/rest/git/tags#get-a-tag)
  accepts an annotated tag-object SHA and reports GitHub signature verification
  metadata (`verification.verified`/`reason`). Signature/payload bytes and
  identities are not retained. Signed-tag requirements must remain Unknown for
  inaccessible or unsupported objects and Fail for known unsigned/unverified
  tags.
- Cargo's [workspace package fields](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-workspacepackage-table)
  allow a workspace root to declare the shared project version; npm's
  [`package.json` version field](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#version)
  declares a Node project's package version. T024 uses bounded, no-follow local
  manifests as project-version evidence, never `SkillManifest.semantic_version`.
  If both Rust and Node version fields are present they must agree; a valid
  manifest without a version field is not itself a version source. If neither
  source declares a version, or a declared value is malformed/ambiguous, the
  result remains Unknown.

T023 protection/evidence API contract (GitHub REST docs version current on
2026-10-05):
- [`GET /repos/{owner}/{repo}/rules/branches/{branch}`](https://docs.github.com/en/rest/repos/rules#get-rules-for-a-branch)
  returns active effective rules from repository and parent scopes, excluding
  `evaluate` and `disabled` rulesets. It has a branch path parameter (no wildcard)
  and paging up to 100; this driver will not paginate unboundedly.
- [`GET /repos/{owner}/{repo}/rulesets`](https://docs.github.com/en/rest/repos/rules#list-repository-rulesets)
  accepts `includes_parents=true` and branch `targets`, and exposes source,
  enforcement and bypass metadata when authorized. GitHub's ruleset GET docs
  state that `bypass_actors` is withheld unless the caller has write access to
  that ruleset. A missing bypass field cannot establish absence of bypass;
  `current_user_can_bypass` describes only the current actor, not all actors.
- Classic [`GET /repos/{owner}/{repo}/branches/{branch}/protection`](https://docs.github.com/en/rest/branches/branch-protection#get-branch-protection)
  returns required status checks, PR review requirements, admin enforcement,
  and PR bypass allowances; availability depends on plan and access. A 404 is
  not sufficient alone to infer no protection because private-resource access
  may be hidden. Combine classic and active effective rules, or return Unknown.
- [`GET /repos/{owner}/{repo}/commits/{ref}/check-runs`](https://docs.github.com/en/rest/checks/runs#list-check-runs-for-a-git-reference)
  supports a specific ref/SHA and returns each run's `head_sha`, `name`,
  `status`, and `conclusion`; only exact-SHA `completed/success` evidence passes.
  Use per-page 100 without `--paginate`; if `total_count` indicates truncation,
  Unknown. Legacy statuses are separate via [`GET /repos/{owner}/{repo}/commits/{ref}/status`](https://docs.github.com/en/rest/commits/statuses#get-the-combined-status-for-a-specific-reference),
  where GitHub defines combined state `success` only when every latest context
  succeeds. Check-run access on private repos depends on token type/permission;
  any 401/403/404 or unlisted fine-grained permission stays Blocked/Unknown.

The [fine-grained token permission matrix](https://docs.github.com/en/rest/authentication/permissions-required-for-fine-grained-personal-access-tokens)
lists effective `rules/branches` and ruleset reads under repository Metadata
read, and classic branch protection endpoints under Administration read.
`X-Accepted-GitHub-Permissions` is a response hint for the endpoint, not proof
that the active identity has that permission. HostRule can pass only if the
required data—including bypass visibility when requested—is actually returned;
RequiredCi additionally needs the active host rule and current-SHA checks.

T020.c.c npm suite driver uses the installed npm CLI `11.16.0`, verified on
the Windows host together with Node `24.18.0`. The tagged
[`package.json`](https://github.com/npm/cli/blob/v11.16.0/package.json) declares
Node engines `^20.17.0 || >=22.9.0`; the profile already discovers npm 9–11 and
Node 18–24, while the suite driver pins the exact npm CLI release and enforces
that engine separately. npm's tagged [`run` command](https://github.com/npm/cli/blob/v11.16.0/lib/commands/run.js)
checks the requested script key and executes only it when `ignore-scripts` is
true, suppressing matching pre/post hooks. The tagged
[`npm-cli.js`](https://github.com/npm/cli/blob/v11.16.0/bin/npm-cli.js) loads
relative implementation modules, so fingerprinting that entrypoint does not
attest the whole installed npm tree.

The npm CLI v11 [`npm run` docs](https://docs.npmjs.com/cli/v11/commands/npm-run)
and v11.16.0 source agree that `run-script` invokes the selected script through
the platform shell (`/bin/sh` or `cmd.exe`) and supports `script-shell`; this is
explicitly treated as execution of repository code after trust confirmation,
not as shell-free execution or a sandbox. The same docs specify that
`--ignore-scripts` still executes the specifically requested script but not its
pre/post scripts. npm's [config](https://docs.npmjs.com/cli/v11/using-npm/config)
and [`.npmrc`](https://docs.npmjs.com/cli/v11/configuring-npm/npmrc) docs list
project, user, global and builtin config sources; [folders](https://docs.npmjs.com/cli/v11/configuring-npm/folders)
documents global npm installation layouts. The T020.c.c contract requires the
driver to redirect user/global config to private empty files and block when a project-root `.npmrc` exists,
because credentials or config-defined shell behavior must not silently reach a
suite. A package tree/lockfile does not pin the globally installed npm CLI.

[`@npmcli/run-script` v10.0.4 `make-spawn-args.js`](https://github.com/npm/run-script/blob/v10.0.4/lib/make-spawn-args.js)
starts its child environment from `process.env` and sets the configured shell on
the spawn; [`run-script-pkg.js`](https://github.com/npm/run-script/blob/v10.0.4/lib/run-script-pkg.js)
obtains command text from the selected package script. JameSkills calls npm with
the minimal ProcessPort `ApprovedEnv`, suppresses lifecycle hooks and requires
explicit repository trust. This does not sandbox a script from same-user
filesystem access.

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
