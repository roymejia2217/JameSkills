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

Discrepancia resuelta: la página installation aún describe 0.6.5 y ejemplos versionados 0.6.0; el registro y el release ofrecen 0.7.0. Las URLs /versions/v0.7.0/docs/... devolvieron 404. El ejecutor debe usar fuente tag v0.7.0/docs.rs versión específica para firmas de código. No convertir 0.6.5 en dependencia por copiar esa página.

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

Las fuentes de esta última sección son referencias de implementación, no todas APIs específicas fueron ejecutadas aquí. T001 registra versiones exactas; cada driver guarda help/source y fixtures antes de declarar compatibilidad. Si cambia un contrato, actualizar docs + tarea afectada + tests; no improvisar flags.
