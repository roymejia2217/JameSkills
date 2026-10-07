# Reanudación JameSkills

Fecha UTC: 2026-10-07
Rama / commit observado: `main` / `0381a15` (checkpoint previo a esta actualización).
Base: `main`=`0381a15`, merge squash secuencial de PRs #22–#27 desde la base #21.
PR / CI remota: PRs #22–#27 están MERGED. CI, Required CI y Governance SUCCESS en el head respectivo de cada PR. La rama local `feat/t020-commit-test-checks` se había restaurado a `c976f02` antes de la integración.

## PRs #22–#27 integradas; T042.e espera wiring de selección aprobada

- T020 está marcada completa en `tasks/todo.md`; PR #22 se integró primero a `main`. La rama T020 ya no contiene los cambios posteriores que pertenecen a T022–T042.
- Se integraron en orden #22, #23, #24, #25, #26 y #27. #27 incorporó T039–T042 con commits funcionales; T042.e sigue explícitamente pendiente de verificación/wiring nativo, y el fallback permanece `Unavailable`/quarantined.
- Durante la corrección de los PRs apilados, #25 reveló lockfile obsoleto, import faltante de `AtomicBool` y una expectativa de schema fijada al futuro. Se corrigieron y comprometieron en #25: `a896e43`, `ac9938f`, `26c7682`, `8ce5863`; el pre-push completo pasó con `CARGO_BUILD_JOBS=1`. PR #26 se sincronizó con esa base y con rustfmt en `0061f84`; su pre-push completo pasó también.
- T023.src `bd78191`, T023.a/b `db5df25`, T023.c `0989248`: rules efectivas/classic protection, bypass tri-state, PR/check contexts y `RequiredCi` solo con regla host activa y resultado successful del SHA local exacto; respeta `integration_id`/`app_id`.
- T023 verificada localmente: host_protection_checks 11 passed + 1 opt-in ignored; workspace tests, workspace Clippy `-D warnings`, fmt/diff-check pass. Integración GitHub CI read-only pasó 1/1, sin atribuir CI SUCCESS al SHA local.
- T024.src committed en `b4419fe` + precisión de versión en `dba65e5`; T024.a implementada en `73a3f62`. Proyecto se versiona desde Cargo workspace/package y/o Node `package.json`, no desde `SkillManifest`; releases publicadas deben tener tag SemVer coincidente; changelog, digest y tag signature solo se exigen según flags del profile.
- T024 RED `current_project_version_requires_a_published_matching_release_with_asset_digest`: `Unknown` porque no había dispatch para `ReleaseContract`; GREEN release_checks 12 passed + 1 opt-in ignored. Cubre conflicto de versiones, JSON duplicate key, semver/tag wrong, draft/prerelease, release ausente vs 403/404, lista de 100 sin paginar, SHA-256 asset digest, changelog con notas, firma annotated/tag lightweight y no requerir flags desactivados.
- T024 gates Windows MSVC: `cargo test --workspace --locked --features jameskills-desktop/test-support`, workspace Clippy con mismos features y `-D warnings`, `cargo fmt --all -- --check`, `git diff --check` pasan. Opt-in release read-only pasó 1/1; solo valida evidence/source/version bound, no exige que el checkout tenga release publicada ni marca el resultado Pass.
- C008 completado localmente: T022–T024 GitHub tests usan GET/read-only, los gates acumulados pasan, no se escribieron reglas/releases/tags. CI remoto conocido `37328599512` solo cubre `c976f02`; no atribuirlo al HEAD local.
- T025.src committed `0a0dfbf`; T025.a `f578924` conserva policies/plans tipados. `guidance_schema`: 5/5.
- T025.b.src `3bb5636`; planner `6f91cd6`; b2 `874accf` separa facts/check fingerprints. `guidance_planner`: 10/10, incluye fresh pass/Unknown/Blocked/expired/NotApplicable/branching.
- T025.c GuidanceService commit `4d39633`; `guidance_service`: 6/6 para start/advance/recheck, answer, fingerprint invalidation, report refresh/error, límite/close. Las sesiones son process-local, no durable SQLite.
- T025.d runtime wiring `d70eedf`; `guidance_runtime`: 1/1. OS/arch facts llevan `CheckEvidence` fresca; el PolicyCheckProvider runtime no disponible mantiene verifiers en Unknown.
- Gates acumulados T025 en Windows MSVC: `cargo test --workspace --locked --features jameskills-desktop/test-support`, workspace Clippy con mismos features y `-D warnings`, fmt y diff-check pasan. CI remoto conocido solo cubre `c976f02`; no atribuirlo a HEAD local.
- T026.src `5a378e4` fija boundaries conservadoras: no spawn por GuidanceAction, doctor solo registry/PATH.
- T026.a `bde09a2` reporta todos los tools Missing/Candidate/Blocked en CLI text/JSON. 3 focused doctor tests, CLI package 19 tests; PATH vacío no revela paths ni instala; candidatos no se sondean. Manual `cargo run -p jameskills-cli --locked -- doctor --json` muestra `version: null` sin probe.
- T026.b source `0c86855` cita Git rev-parse; implementación `3fac137` mapea cinco OfficialGuidanceSource y renderiza únicamente `(Git, RepositoryRoot)` como texto copiable; renderer ausente=Unsupported. Core conserva check Unknown tras acknowledge del copy action. RED infra renderer falló por la falta del map Git source; GREEN infra 2/2 y GuidanceService 7/7.
- T026 gates en Windows MSVC: `cargo test --workspace --locked --features jameskills-desktop/test-support`, workspace Clippy `-D warnings`, fmt y diff-check pasan. T026 se verifica localmente; GPUI todavía no está conectada a GuidanceService y repo mutations siguen fuera de scope, con API preview/digest prevista en T027. No atribuir CI remota a commits locales.
- T027.src `f2aa07d` separa RepositoryChangeService, plan/apply, digest y journal de GuidanceService/PolicyService. El hallazgo original —sin RepoChangePort/journal API— se ha resuelto parcialmente en T027.a/b; SQLite migration 002 ya crea `operations`, pero StoragePort aún no la expone.
- T027.a `3527835`: agrega `RepoTemplateId::RustCi`, target fijo `.github/workflows/jameskills-ci.yml`, `RepoChangePlan`, `ApprovedRepoChange` no deserializables y digest SHA-256 estable ligado a template/root/HEAD/target/prior/proposed/diff. No guarda bytes de template en el DTO; diff está bounded y rechaza controles de terminal. `repo_change_plan` 3/3; Clippy core `-D warnings` y fmt/diff-check pasan. RED comprobó que el digest inicial omitía root.
- T027.b `72c8e31`: `RepositoryChangeService` + `RepoChangePort` separados del PolicyService de checks y GuidanceService. Plan hace preview-only; apply exige root/head/template y digest exactos antes del write port; receipt incluye operation ID, target y hashes. RED confirmó que root mismatch llegaba a apply; `repo_change_service` 3/3, core suite completa, Clippy core `-D warnings` y fmt/diff-check pasan.
- T027.c `1b2e6e3`: root fingerprint canónico, planner create-only, paths no-follow/reparse check; target ajeno conflict, preview read-only. Windows junction/reparse opt-in pasó después con Developer Mode activado; Unix symlink runtime sigue sin ejecutar.
- `75d1faa` actualiza el runtime-fixture assertion del CI de ejemplo: exige triggers, contents:read, refs SHA, toolchain1.95.0 y ningún placeholder/`exit 1`; el test de bundle pasó.
- T027.d1 `e5089fe`: `OperationJournalPort` y SQLite usan tabla `operations` (schema v3, sin migración); payload JSON estricto 16KiB con paths locales/hashes, nunca contenido. Estado CAS transaccional y límites 256 pending. RED duplicate-ID detectó `INSERT OR REPLACE`; GREEN journal 5/5, core+infra suites completas y Clippy `-D warnings` pasan.
- T027.d2.src `9be943a` define Git fingerprint/environment aprobado y limita los probes a argv/read-only; composition aún no inicializa SQLite.
- T027.d2a `ca18acf`, corrección `8f52157`: requests y RepoChangePort llevan Git executable+fingerprint+`ApprovedEnv`; constructor exige confirmación del fingerprint exacto. RED confirmó que un digest distinto era aceptado; `repo_change_service` 4/4, Clippy core `-D warnings` y fmt/diff-check pasan.
- T027.d2b `d767f3c`: helper infra verifica Git profile/version y HEAD antes/después del preview con `--version` y `rev-parse HEAD` bajo ReadOnlyCheck/fingerprint. `template_plan` 7/7 y core+infra suites completas; Clippy core/infra pasa. ProcessPort fake verifica argv/permisos; todavía no es una implementación de `RepoChangePort` ni prueba un spawn real.
- T027.d2c.src `74f6097` cita cap-std `4.0.3` y fija root-scoped filesystem capability handles, nofollow/reparse checks, create-only hardlink, same-filesystem staging y recovery sin borrar edits ajenos.
- T027.d2c1 `74b0161`: cap-std pinned en Cargo.toml/lock; planner usa `Dir::open_ambient_dir` y lectura relativa. `cargo check -p infra --locked`, suite infra 7 passed + 1 ignored, Clippy infra `-D warnings`. Windows junction test luego pasó opt-in 1/1 tras activar Developer Mode; Unix symlink test aún no observado.
- T027.d2c2 `5f94c21`: LocalRepoChangePort verifica preview/Git/HEAD/digest, registra Planned/Approved/Staged/CommitPending/NewMoved/Verified/Committed, hard-link create-only y elimina solo stage con hash exacto. Parent `.github/workflows` debe existir; no crea scaffolding. `template_plan` 13 passed/2 ignored, `repo_change_journal` 5/5, suite infra y Clippy pasan.
- T027.d2c2 real Git `e718377`: opt-in `real_system_process_git_applies_registered_template_to_temporary_repository` pasó 1/1 en Windows con SystemProcessPort, Git profile/fingerprint y SQLite real en TempRepository; no toca el checkout. Runtime composition/CLI/GPUI aún no inyecta el SQLite journal.
- T027.d2c3.a local: `LocalRepoChangePort::recover_pending` reconcilia sin spawn; commit solo si target/stage tienen hash esperado y comparten identidad, rollback elimina únicamente staging con hash esperado, y target editado/no-owned queda Conflict/Pending. `same-file=1.0.6` compara handles abiertos relativamente desde `cap-std`; `repo_change_journal` 8/8 y snapshots Planned/Approved/Staged/CommitPending/NewMoved/Verified/RollbackPending prueban idempotencia.
- T027.d2c3.b local: `FailTransitionJournal` existe solo en `tests/template_plan.rs` y falla persistencia en CommitPending/NewMoved/Verified/Committed, representando interrupciones post-stage, post-hard-link, post-verify y post-cleanup. Tras cerrar y reabrir SQLite, recovery deja estado consistente y no invoca ProcessPort. El caso parametrizado pasó 1/1; `template_plan` 14 passed/2 ignored; `repo_change_journal` 8/8; suite infra completa; Clippy infra `-D warnings`, fmt y diff-check pasan en Windows MSVC.
- T027.e local: el runtime y el ejemplo documental describen revisión manual del template CI y declaran que guidance no aplica archivos ni activa hooks. `guidance_service` 8/8 valida `ManualInstruction` + AwaitingEvidence; `bundle_manifest` confirma que las copias permanecen sincronizadas. Se capturó RED primero; después un test de igualdad detectó una copia documental stale, corregida junto al ejemplo runtime.
- T029.a local: `AgentId` cerrado; capabilities independientes y matriz completa; cada estado exige source/date, Supported exige versión+fixture. VendorPlugin restringido a Antigravity User. Project detection requiere root aprobado; Verified exige executable+fingerprint+versión. `agent_registry` 8/8, suite core completa, Clippy core `-D warnings`, `cargo check -p jameskills-infra --locked`, fmt/diff-check pasan.
- T029.a.contract sincroniza el blueprint del port: por ahora `AgentPort` expone `profile`/`detect`; render/verify esperan Bundle/Receipt tipados. No se clasificó el fallo de import/compilación inicial como RED de dominio.
- T029.b.registry local: `AgentRegistry` contiene cinco perfiles estáticos y valida integridad/completitud. Los adapters no implementados aún no exponen detección ni Supported; todos permanecen NeedsVerification o Unsupported. `cargo test -p jameskills-infra --locked --test agent_registry` 4/4; suite infra completa, infra Clippy `-D warnings`, core `agent_registry` 8/8, core suite/Clippy y fmt/diff-check pasan en Windows MSVC.
- T029.b.sources mapea los cinco source IDs del registry a las URLs oficiales ya registradas en `docs/SOURCES.md`. No se afirma versión CLI ni fixture probado; las capabilities siguen NeedsVerification/Unsupported. T029 completada localmente, no implica adapter ejecutable.
- T030.a agrega `ApprovedAgentExecutable` y lo incorpora al `DetectionContext` solo si el fingerprint observado coincide con la confirmación explícita; `agent_registry` 9/9, core suite, core Clippy y fmt/diff-check pasan. `cargo check -p jameskills-infra --locked` pasa.
- T030.b separa ProcessIdentity Tool/Agent del ToolId de policies; fakes/consumers migrados y ProcessPort ejecuta un helper real bajo identidad Agent+fingerprint. Docs CONTRACTS sincronizado.
- T030.c `CodexAdapter` planifica target `$HOME/.agents/skills/<slug>` o `<repo>/.agents/skills/<slug>`; recalcula el hash de los bytes frente a `ValidatedBundle`, conserva el árbol relativo íntegro y no escribe/ejecuta/genera `openai.yaml`.
- T030.d usa candidate PATH read-only; sin aprobación fingerprint no ejecuta. Con executable confirmado, probe fijo `codex --version` usa ProcessPort/ReadOnlyCheck/ProcessIdentity::Agent; salida no parseable queda Blocked. Fuentes registradas incluyen install standalone/npm y source `main` mutable; no se infiere versión de release.
- T030 local: `codex_adapter` 8/8; suites core/infra completas; Clippy core/infra `-D warnings`, fmt/diff-check pasan. Codex no está instalado en PATH (`Get-Command codex` ausente); no se vio output/version real. Los tests de versión usan ProcessPort fake y no elevan capabilities.
- T031.a comparte `AgentArtifact`/`plan_file_copy_artifact` entre Codex y OpenCode; mantiene recalculado el bundle hash, copias exactas y paths relativos.
- T031.b local: OpenCode documentado en `.opencode/skills`, `~/.config/opencode/skills`, `XDG_CONFIG_HOME` y `OPENCODE_CONFIG_DIR`. Source `dev` confirma búsqueda en config dirs y versión vía `--version`. Probe requiere fingerprint aprobado; candidato sin aprobación no se ejecuta. `opencode_adapter` 4/4; capabilities siguen NeedsVerification y no se afirma versión nativa.
- `Get-Command opencode` observó un candidato en PATH, pero no se ejecutó ni fingerprintó; el usuario confirma que está instalado. Solicitar aprobación explícita antes de un probe real.
- T031.b.sources enlaza docs CLI/skills/config y snapshots `dev` de discovery/global config/version. No se fija version range porque no se observó release/fixture de la CLI instalada.
- T032 local: `PI_CODING_AGENT_DIR` absoluto no vacío cuando existe; User root `<agent-dir>/skills` (default `<home>/.pi/agent/skills`), Project `.pi/skills`. Release tag Pi v1.0.4 (package/version/source `pi --version` raw) y Windows asset SHA-256 anotados en SOURCES. Parser acepta exactamente la fixture raw `1.0.4`; profile capabilities siguen NeedsVerification.
- Pi CLI no está en PATH (`Get-Command pi` ausente); no se ejecutó binary native. `pi_adapter` 4/4; suite infra completa, Clippy infra `-D warnings`; core suite/Clippy, fmt/diff-check pasan en Windows MSVC.
- T033.a Antigravity: artifact plugin inerte en el CLI profile User, destino `.gemini/antigravity-cli/plugins/<name>`; Project Unsupported; `plugin.json` omite `$schema` hasta resolver discrepancia de schema formal. `antigravity_adapter` 5/5; no version probe ni plugin mutation real.
- Suite completa `cargo test -p jameskills-infra --locked` pasó en Windows MSVC tras T033.a/T034.a: adapters incluidos, `grok_adapter` 4/4, `antigravity_adapter` 5/5. Clippy infra primero detectó dos `needless_return` de T033.a; corregidos en `antigravity.rs` y `platform.rs`; después `cargo clippy -p jameskills-core -p jameskills-infra --all-targets --locked -- -D warnings` pasó.
- T034.a Grok paths/artifact hash-bound: User `$GROK_HOME/skills` o `~/.grok/skills`, Project `.grok/skills`; targeted 4/4 y suite infra completa. `Get-Command grok` no encontró CLI; no se probó `grok version`/`inspect --json` nativo, y la detección aprobada queda Blocked.
- T039.a `LibraryQuery` limita página 1–50, normaliza Unicode lowercase y acota filtros; `LibraryCursor` usa `(normalized_display_name, SkillId)`; DTOs inmutables preservan todos los heads/conflictos. `library_query` 4/4.
- T039.b schema v4 añade `library_catalog`, `revision_tags`, `revision_capabilities` e índices de búsqueda/history; upgrade backfillea nombres legacy con Unicode lowercase de Rust. RED runtime por catálogo normalizado ausente; GREEN `sqlite_migrations` 13/13, upgrade v1 y focused de `revisions_by_skill_and_id`.
- T039.c/d implementa query SQLite parametrizada, LIKE con `%`, `_`, `\\` escapados, filtros por latest-head tags/capabilities/state, cursor keyset estable y StoragePort async sobre `spawn_blocking`; LibraryService y RuntimeServices abren `data/library.sqlite3`.
- T039.e carga bytes solo en detalle explícito e historial paginado por revision ID con parents/tombstone observed-heads, sin timestamps ni blobs. `SaveRevisionRequest::with_validated_bundle` liga metadata al bundle/hash/id/semver/schema exactos y commit la indexa junto a revisión/head en una transacción.
- Verificación T039 Windows MSVC: `library_queries` 7/7 (dataset 125 skills, empates y páginas sin repetición/omisión), `library_query` 4/4, `sqlite_migrations` 13/13, `library_validation` 1/1. Workspace tests con `jameskills-desktop/test-support` pasaron; workspace Clippy `-D warnings`, fmt y diff-check pasaron. Tras añadir el caso de empate, focused pasó 1/1 y Clippy workspace se repitió.
- C013 verificado localmente tras T037–T039. CI remota del SHA local no observada; sin push.
- T040.a `SkillDraft` modela bytes exactos, `PortablePath`, base-head y generación sin imponer `validate_bundle`; aplica topes de 2,000 archivos/20 MiB. `library_authoring` core 4/4 cubre invalid bytes conservados, generation/base-head replacement, resource bounds y SaveDraftRequest CAS shape. Sin RED previo de comportamiento: el contrato de draft se hizo explícito antes de integrar persistencia.
- T040.a gates: core tests completos, core Clippy `-D warnings`, `cargo check --workspace --locked`, fmt y diff-check pasan en Windows MSVC.
- T040.b2 persistencia SQLite guarda envelope binario `JSD1` bounded con byte/path/skill/base-head/generation exactos; CAS usa transacción Immediate, conflicto preserva el draft ganador, decoder inválido queda Storage corrupt y no crea revisión/head. `library_authoring` infra 3/3; suite infra completa y Clippy core/infra `-D warnings`, fmt/diff-check verdes.
- T040.c expone `save_draft`/`load_draft` por `LibraryService`; `library_validation` infra 1/1 prueba RuntimeServices→SQLite con bytes inválidos exactos y que no se publica.
- T040.d1 `CreateSkill` genera ID y template portable `SKILL.md`/manifest válido editable. RED de validator detectó mismatch frontmatter.name vs manifest.slug; corregido. `library_authoring` core 6/6; core suite/Clippy, workspace check, fmt/diff-check pasan.
- T040.d2 `LibraryService::create_skill` y StoragePort/SQLite insertan skill+catalog+draft en transacción Immediate. Failure injection en el último INSERT demuestra rollback de filas anteriores. `library_authoring` infra 5/5.
- Verificación T040.d: core `library_authoring` 6/6, infra `library_authoring` 5/5, suites core/infra completas y workspace check/Clippy `-D warnings`, fmt/diff-check pasan Windows MSVC.
- T040.e1 `PublishDraft` fija skill, draft generation y expected-head set canónico/bounded 128; `library_authoring` core 7/7, Clippy core `-D warnings`.
- T040.e2 publicación end-to-end: StoragePort revalida y guarda archive hash-bound; `commit_revision` detecta identical bundle+version como no-op, exige versión SemVer mayor al cambiar bytes y consume draft generation solo dentro de la transacción revisión/head. `LibraryService::publish` valida draft/ID/base-head; errores/stale CAS dejan draft/head intactos.
- Verificación T040.e: `revision_storage` 15/15, infra `library_authoring` 6/6; core/infra suites completas y workspace `cargo check --locked`, Clippy workspace `-D warnings` con test-support, fmt/diff-check pasan Windows MSVC.
- T040.f1 Tokio runtime CLI exact `1.53.1`; f2 `library list/create/publish` delegates to RuntimeServices. List supports search/tags/capabilities/state/cursor; create stores valid draft; publish requires draft generation/expected heads. Import/export remain Unsupported until T042/export provider.
- Verification `cargo test -p jameskills-cli --locked` passed, including create→publish→list against a temporary real SQLite runtime. No CI remote observed for local SHA.
- T041.a/b asset APIs: mutations pure/inert, confined to `assets/`, `references/`, `templates/`; expected raw-file hash, inventory limits/casefold, referenced rename/remove returns explicit update-required; preview text bounded/UTF-8; SVG raw text only, binaries unsupported.
- `LibraryService` asset methods CAS-save a next draft generation, not `skill_heads`. Focused `library_assets` core 4/4 and `asset_editing_changes_only_the_saved_draft_until_a_later_publish` infra 1/1 passed; complete core/infra suites, workspace check, workspace Clippy `-D warnings` with test-support, fmt/diff-check also passed.
- `docs/CONTRACTS.md` y SECURITY establecen SVG raw text (nunca markup), preview UTF-8 64KiB y binarias Unsupported; references quedan protegidas por explicit update-required.
- T041 complete locally, no remote CI for local SHA. C014 waits for T042.
- T042.a1 añade `FileSystemPort::read_bundle_source`: directory, archivo `.jskill` o `SKILL.md` standalone acotado, parser valida central/local/checksums y devuelve bytes como BundleFiles inerte; reparse/symlink y malformed source no pasan. `library_import` infra inicial 2/2 Windows MSVC.
- T042.a2 `ImportPreview` revalida files, clasifica NewSkill/Identical/Conflict preservando heads bounded, registra sourceKind y permanece Quarantined sin writes; core `library_import` 3/3.
- T042.b1 schema v5 lookup index `revisions_by_skill_and_bundle(skill_id,bundle_hash,state,id)` plus v6 local `revision_trust`; `StoragePort::get_heads`/`skill_exists`/`find_revision_by_bundle` no cargan blobs. SQLite migrations 13/13, Clippy core/infra.
- T042.b2 `LibraryService::preview_import` lee fuente, valida bundle, determina new/identical/conflict y devuelve `Quarantined`; no persiste ni revisa. `library_import` infra 3/3 prueba read-only duplicate/conflict.
- T042.c apply imports exact duplicate as no-op; a same-UUID/different-content bundle can add a concurrent root while preserving old heads; revision_trust is Quarantined and no parents are fabricated. Failure injection rolls back SQL heads/metadata; content-addressed blob may remain orphaned and is inventory-safe. `library_import` infra 4/4.
- T042.d verificada localmente: standalone `SKILL.md` genera ID/manifest nuevos y draft quarantined, preserva bytes exactos, y apply solo puede crear draft (no blob/revision/head). Publish rechaza drafts quarantined. Core `library_import` 4/4; infra `library_import` 6/6, `library_authoring` 7/7, `sqlite_migrations` 13/13 y las suites enfocadas de assets/validation/revision-storage pasan en Windows MSVC. Clippy core+infra `-D warnings`, fmt y diff-check pasaron; tras ajustar fallback trust authored, se repitieron tests infra relevantes.
- T042.e.a local: enum `ImportScanStatus` redacted y `ImportScanPort`; Unavailable/NoFindings/Findings/Unknown/Blocked nunca alteran TrustState y el confirmation digest ata el estado mostrado. Scanner errors quedan Unknown; cancelación se propaga.
- T042.e.b local: `GitleaksImportScanner` solo acepta profile/candidate Gitleaks, fingerprint explícito y ProcessPort; empaqueta bytes ya validados, extrae a staging privado exclusivo por scan, corre el contrato existente 8.30.1, rechaza `.gitleaksignore`, borra staging/config y entrega únicamente estado redacted. `library_import_scanner` 3/3 cubre clean/findings/malformed, missing fingerprint sin ejecución/escritura y ignorefile bloqueado. Fakes verifican contrato, no la integración nativa.
- T042.f local: `library import --path` da preview JSON/Text con rutas/tamaños, clasificación, heads, scan/trust status y digest por resolución. `--apply` requiere resolución+digest; relee la fuente, valida digest/heads y usa el UUID mostrado para plain-SKILL. Tests cubren edit-after-preview -> stale sin writes -> preview fresco/apply; suite también prueba plain-SKILL como draft quarantined sin publicación.
- Próximo trabajo: completar el wiring de selección explícita del scanner aprobado en RuntimeServices/CLI, conservando `Unavailable`/quarantine hasta que el usuario lo seleccione. La verificación nativa actual se limita al fixture aislado; no afirmar que el runtime del producto ya escanea imports.
- Gitleaks Winget `Gitleaks.Gitleaks 8.30.1` se verificó en el ejecutable aprobado; SHA-256 `17157e2ee8b76fc8b1d8bee607a250e34b8a8023c8bc81822d4b5ee4d78fcb7c`. El test opt-in con `SystemProcessPort` sobre el fixture aislado pasó con estado redacted `NoFindings` y staging limpio. `build_services` no lo selecciona todavía; imports reales en el runtime siguen `Unavailable`/Quarantined.
- Bloqueos paralelos: T030 Codex ausente; T033.b sin fingerprint/output fixture `agy`; T034.b sin CLI/fixture Grok; T035 depende de T033/T034 completas.
- Windows reparse/junction runtime: el test que antes daba `ERROR_PRIVILEGE_NOT_HELD` pasó 1/1 tras activar Developer Mode, con `cargo test -p jameskills-infra --locked --test template_plan reparse_target_parent_is_blocked_without_writing_outside_root -- --ignored --exact`. Unix symlink runtime aún no observado. `RuntimeServices`/CLI/GPUI aún no inyecta SQLite journal; la integración Git real previa solo cubre T027.d2c2 y usó repositorio temporal.
- Verificación acumulada local Windows MSVC tras T042.f: `cargo test --workspace --features jameskills-desktop/test-support --locked` pasó en ese checkpoint; workspace Clippy con los mismos features y `-D warnings`, fmt, diff-check y `cargo check --workspace --locked` pasan. `library_import_scanner` 3/3. No atribuir ejecución Linux ni CI remota a estos cambios. La prueba opt-in de junction/reparse parent pasó 1/1 tras Developer Mode; opt-in Git apply real previo pasó 1/1.
- Runtime composition CLI/GPUI aún no inyecta el SQLite journal de RepositoryChangeService; la capability implementada no está expuesta como acción de producto.
- Los cambios T039–T042 se integraron mediante PR #27. El pre-push gate local pasó (Commitlint, fmt, Clippy, pruebas core/infra/CLI, desktop check y diff-check); CI remota también pasó en Linux y Windows para el head de PR #27. La verificación nativa de Gitleaks no forma parte de esos checks y sigue pendiente.

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

1. Continuar T041 (primera elegible): gestionar assets/referencias como datos con inventory/hash preservados, límites y staging seguro.
2. Después T042 conecta import real y duplicate-ID/quarantine; hasta entonces CLI import/export reportan Unsupported sin éxito falso.
3. T033.b y T034.b esperan fixture/fingerprint nativo; no mutar perfiles vendor ni declarar versiones sin evidencia. T035/T036 dependen de adapters completos.
4. `RuntimeServices`/CLI/GPUI no conecta todavía RepositoryChangeService ni SQLite journal; no presentar repo apply como acción disponible.

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
