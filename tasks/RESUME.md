# Reanudación JameSkills

Fecha UTC: 2026-10-04
Rama / HEAD: `feat/t020-commit-test-checks` / `HEAD` (base de slice `caa9a23`; T020.a `1cd44fc`).
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

## Slice actual T020.b1 (T020.a comprometido)

- Se verificó la documentación oficial actual del CLI y la fuente/tag `@commitlint/cli` v21.2.2.
- La fuente 21.2.2 implementa `--default-config` y `--edit <file>`; el CLI anterior 20.2.0 no implementa `--default-config`. La página viva ahora reporta v21.2.3, que no se acepta sin revisión/fixture propia.
- RED: `cargo test -p jameskills-infra --locked --test tool_detection commitlint_profile_supports_the_reviewed_default_config_cli` falló porque `profiles/tools.toml` solo permitía `>=19,<21`.
- GREEN: el test focal pasa 1/1 después de fijar `=21.2.2`; rechaza 20.2.0 y 21.2.3.
- RED T020.b1: E0599 por `check_conventional_commit` ausente. GREEN `cargo test -p jameskills-infra --locked --test commit_test_checks` 3/3; fake comprueba argv, archivo/config privados y limpieza, exit 0/1, salida no filtrada, version pin y PATH hacia Git aprobado.
- HumanCrop/JamePrompt usan Husky `npm exec ... commitlint --edit "$1"`; JameFirewall lo usa en CI por rango; ImageMD valida metadata/subjects como datos en Python. El método reutiliza Commitlint oficial 21.2.2 (`--default-config --edit`) en vez de un parser sustituto.
- `--edit` ejecuta internamente `git config core.commentChar`; el driver limita PATH para esa llamada al Git aprobado primero y revalida su fingerprint antes del spawn. Un JSON vacío explícito más `--default-config` evita cargar config JS de repositorio/ancestros.
- El test es fake de ProcessPort, no ejecución real de Commitlint. En Windows el CLI npm estándar aparece como `.cmd` shim y sigue Blocked; T020.b2 debe resolver el entrypoint Node de forma aprobada y cablear el provider. No declarar Pass real todavía.

## Patrones de repos relacionados revisados

- HumanCrop y JamePrompt usan Husky `commit-msg` con `npm exec --no -- commitlint --config commitlint.config.cjs --edit "$1"`; ambos extienden `@commitlint/config-conventional`. HumanCrop fija CLI/config 21.2.3; JamePrompt fija 21.2.2 y tiene pruebas de mensajes aceptados/rechazados.
- JameFirewall fija CLI/config 21.2.2, corre `npm exec --no -- commitlint --config ... --from <base> --to <head>` en CI y mantiene self-test del contrato; su gobernanza de título usa `action-semantic-pull-request`.
- ImageMD no usa Commitlint: `ci/governance.py` valida títulos y subjects de commits como datos obtenidos por API, sin ejecutar su contenido.
- T020 usa el CLI oficial 21.2.2, no un parser sustituto. `--edit` oficial invoca internamente `git config core.commentChar`; el driver actual solo admite Git/Commitlint nativos aprobados, coloca Git primero en PATH para ese comando fijo y usa cwd/config privados.

## Verificaciones locales acumuladas

- `cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked`: pasó.
- `cargo test --workspace --features jameskills-desktop/test-support --locked`: pasó en Windows, incluidos 14 desktop unit tests y 2 lifecycle tests.
- `cargo clippy --workspace --all-targets --features jameskills-desktop/test-support --locked -- -D warnings`: pasó.
- `cargo fmt --all -- --check`, `git diff --check`: pasaron.
- `cargo build -p jameskills-desktop --target x86_64-pc-windows-msvc --locked`: pasó.
- `scripts/test-commitlint.sh` por Git Bash y `npm exec --no -- commitlint --from main --to HEAD --verbose`: pasaron para commits probados.
- CI remoto y build Linux: no ejecutados. El workflow local refleja sus gates en `.github/workflows/ci.yml`; CI real requiere PR.

## Próxima acción exacta

1. Revisar el diff limitado a los cinco archivos T020.b1, ejecutar `repo_document_checks`/`commit_test_checks`, fmt, Clippy infra y diff check; crear commit Conventional Commit válido.
2. T020.b2: resolver `@commitlint/cli/cli.js` por Node con identidad/hash aprobados, sin ejecutar wrapper `.cmd`, y conectar el provider.
3. T020.c: acción explícita de test drivers; inspección normal no lanza Cargo/npm scripts.
4. Si se obtiene Gitleaks 8.30.1 verificado, cerrar el bloqueo T019; mantener C006/C005 abierto hasta evidencia nativa, sin inferir Pass Linux/GPU.

## Preservación y lecturas

Preservar `target/` y `JameSkills-implementation-dossier.zip` locales sin seguimiento. Revisar `AGENTS.md`, T020/C006 en `tasks/todo.md`, `docs/CONTRACTS.md`, `docs/SECURITY.md`, el spec pertinente y `.github/workflows/ci.yml` antes del siguiente incremento.
