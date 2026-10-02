# Puesta en marcha, CI, repositorio y distribución

Runbook FUTURO. /workspace de planificación no tiene Rust ni repoGit inicial; solo dossier. Comandos requieren herramientas/account owner, nunca inventar credenciales ni afirmar ejecución.

## Arranque desarrollo

1. Conservar docs; inspeccionar si repo apareció antes git init; crear workspace4 crates y branch bootstrap/jameskills.
2. Rust1.92.0 baseline/edición2024, version0.1.0, Kit=0.7.0. Lockfile registra resolver real; MSRV mayor evidenciado se actualiza junto toolchain/CI/evidence. Sin latest/*/patch arbitrario.
3. Instalar prereqs fuente y spike ShellView+Button+Assets nativo ambosOS. CLI doctor independiente GPU.
4. Registro exact dependency/tool versions docs/evidence/T001.md; factory storage+recovery->shell.
5. Importar golden suite, validar/publicar/detectar agentes, instalación user scope.
6. Drive tras tests crypto+DAG+restore, owner provisiona OAuth.
7. CI/packages/smoke/firma antes distribución y publicación autorizada.

## Linux

Ubuntu24.04 x86_64 referencia, sesión gráfica y GPU Vulkan:
~~~
sudo apt update
sudo apt install -y gcc g++ clang pkg-config cmake libfontconfig-dev libwayland-dev libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev libssl-dev libzstd-dev vulkan-validationlayers libvulkan1
rustup toolchain install 1.92.0 --component rustfmt clippy
rustup override set 1.92.0
cargo build -p jameskills-desktop --locked
cargo run -p jameskills-desktop --locked
~~~
Instrucciones owner, skill/app no ejecuta sudo. libvulkan1 es loader, no driver; comprobar vulkaninfo desde vulkan-tools si disponible y display Wayland/X11. Headless /workspace no prueba GUI. Distros distintas requieren equivalent packages.
Rust mediante instalador oficial usuario; no pipes remotos silenciosos ni cambiar TLS/proxy por setup.

## Windows

Framework Windows10+, QA release Windows11 x86_64. VS2022 Build Tools DesktopC++/WindowsSDK, CMake PATH y Rust MSVC, no GNU. PowerShell5.1 suficiente si scripts compatible, pwsh7 si tarea documenta prereq.
~~~
rustup toolchain install 1.92.0-x86_64-pc-windows-msvc --component rustfmt clippy
rustup override set 1.92.0-x86_64-pc-windows-msvc
rustup show active-toolchain
cmake --version
cargo build -p jameskills-desktop --target x86_64-pc-windows-msvc --locked
cargo run -p jameskills-desktop --locked
~~~
Paths KnownFolders; separar Windows/WSL. Política IT faltante guía dueño, nunca desactivar antivirus/seguridad.

## Golden suite

Una vez targets existentes y ejemplo docs copiado a examples/repository-foundation:
~~~
cargo run -p jameskills-cli --locked -- validate --path examples/repository-foundation --json
cargo run -p jameskills-cli --locked -- library import --path examples/repository-foundation --json
cargo run -p jameskills-cli --locked -- check --repo . --skill f9c0199f-c4ce-4b04-85dd-ae12a7db292b --profile rust --json --strict
cargo run -p jameskills-cli --locked -- agents detect --json
cargo run -p jameskills-cli --locked -- doctor --json
~~~
Check estricta puede exit1/3 por host/tool faltante; correcto hasta guía completa. Tests deterministic no pedir creds todo pipeline.

## Git del propio producto

Conventional Commits+commitlint/config-conventional devtools package-lock; Git-cliff changelog, no release magic.
.gitignore: /target, .env/.env.* excepto example sin secreto, DB/cache runtime, signing/OAuth temporales, artifacts/evidence privados. No Cargo.lock ni toda docs/evidence ni fixtures seguros.
.gitattributes source/docs LF, binarios binary; raw suite imported no CRLF rewrite fueraGit.
README quickstart/commands/arquitectura/limits/security/contribuir/licencia; PR template problema/resultados/tests/source evidence.
Rulesets main PR+review+quality status estable+no forcepush/delete, bypass según autoridad efectiva. Template JSON no aplica servidor, owner configura UI/API y revalida.
Comandos remote repo create/push/PR necesitan autorización contextual de implementación; este dossier no ejecuta esos pasos. No push main.
Secrets Gitleaks staged/CI, owner host/keyring; cero secretos para probar detector.

## CI

ci.yml permissions contents:read, checkout SHA verificado, persistcredentials false, fetch-depth para lint mensajes. Matrix Windows/Ubuntu24.04 toolchain exact+deps+cache OS/rust/lock; fmt/clippy/core/infra/CLI y desktop test-support explícito; release builds ambos.
Agregador quality required necesita all jobs y evalúa fail/cancel no pass. triggers pull_request/push y merge_group si mergequeue; sin paths filters que omitan required result.
security.yml Gitleaks/cargo-audit/deny pinned reales, db/license checks. No pull_request_target checkout PR con secrets. Dependabot actualiza refs+lock en PR.
release.yml protected tags/env, verify tag commitquality, semver/changelog/build/sign/checksum/SBOM/provenance/upload. Actions SHA fuente oficial durante implementación, no hex inventado. Signing secrets mínimos solo protected release.
Workflow escrito no equivale repo protected/release published. Ownerstep missing se marca blocked con guía, sigue artifacts locales.

## Distribución v1

Windows per-user MSI WiX pinned+.wxs product filetree/startmenu/icon/version MSI semver mapping; zip portable adicional CLI+GUI+licenses. Code signing ownercertificate en CI, Authenticode verified; install/uninstall preserves library unless purge explicit.
Linux tar.gz release x86_64 portable con deps/glibc floor verificada, .desktop/icon y ~/.local/bin install. deb/AppImage opcionales solo tarea+runtime probado, no prometer todasdistro.
Checksums y firma/provenance, third-party licenses icon notices. No install agents automáticamente.
Sin autoupdater v1 que modifique executable: Ajustes enlace official release y instrucciones verificarchecksum. Update checker si hay task solo consulta pública sin creds, no auto-run.
Fresh machine: downloadverify/install/appAssets/create/check/agentinstall/Driverestore/close/reopen/uninstall preserves data. Requiere ambos OS reales.
License propuesta Apache2 propia; validar intención owner antes publicación si requiere elegir, sin copiar GPL2FAS.

## OAuth provisionamiento

Owner Cloud project+DriveAPI+consent screen+Desktop client; testing audience/testusers, scopes mínimo drive.appdata. Production brand/domain/support/terms verdaderos owner, no inventados. Scope no sensible no elimina todos publishing requirements.
ClientID público config; mismo deployment/app identity para ambos dispositivos. Otro OAuth project/client puede no ver appData existente; live test restore, no suponer.
PKCE/external browser/loopback ambosOS. Si testing token expires/invalidgrant NeedsReauth truthful.
ChatGPT Drive connector no runtime de la app: cliente OAuth propio.
No plaintext token fallback si keyring missing. Cuenta etiqueta local sinemail inventado.
Reset permanente files.delete, no trash, backup cifrado previo y lista exactfiles vault approved.

## Recovery

Crashinstall: journal+hashes old/new recover owned unchanged; edited conflicto respetado, no rm-rf guessedpath.
DB corrupt: readonlysafe+previousbackup/blob export posible; no recrear empty silent.
OAuth expired: reauth y vaultchooser, no biblioteca duplicada aleatoria.
Remote corrupt: quarantineID y local intact, preservar snapshotgood.
Passphrase lost: mastercached keyring accesible puede export/rotate; sin llave/password ciphertext inaccesible; biblioteca local legible si existe. No borrar backup para arreglar prompt.
Agentversion unsupported: capability disabled con source testedversion, archivos preservados.
Signing unavailable: artifactlocal y publication pendiente, nunca labelSigned.

Completar tareas/specs reales sin stubs, registrar hardware/owner gates y avanzar independientes. Informe final distingue appvalidada/paquetes/remote repo/release. No evidencia account/GPU con fake.
