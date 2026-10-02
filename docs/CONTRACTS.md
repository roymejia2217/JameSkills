# Contratos canónicos y blueprints de código

Este archivo manda sobre nombres/tipos de interfaces en el plan. Código propio orientativo, todavía no compilado. Implementar primero los tests que fallan; no copiar dependencias/APIs upstream no comprobadas. Cada módulo domain reexporta solo tipos públicos; estructuras de SQLite/HTTP nunca llegan a una View.

## Tipos de dominio

~~~rust
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type AppResult<T> = Result<T, AppError>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(Uuid); // private; parse UUID or generate explicitly
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RevisionId(String); // private; exactly 64 lowercase SHA-256 hex
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperationId(Uuid); // private; parse UUID or generate explicitly
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentHash(String); // private; exactly 64 lowercase SHA-256 hex
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentId { Codex, OpenCode, Pi, Antigravity, Grok }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope { User, Project }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus { Pass, Fail, Blocked, Unknown, Unsupported, NotApplicable }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Enforcement { Instruction, LocalCheck, LocalHook, RequiredCi, HostRule }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity { Info, Warning, Error }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub source: String,          // registered source ID, not arbitrary URL
    pub observed_at: String,     // UTC RFC3339; display, not merge ordering
    pub revision: Option<RevisionId>,
    pub environment_fingerprint: String,
    pub summary: String,         // redacted and bounded
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckResult {
    pub requirement_id: String,
    pub status: CheckStatus,
    pub severity: Severity,
    pub enforcement: Enforcement,
    pub evidence: Vec<Evidence>,
    pub guidance_id: Option<String>,
}
pub enum AppError {
    Validation(Vec<Diagnostic>),
    NotFound,
    Conflict { current: Vec<RevisionId> },
    CapabilityUnavailable { id: String, guidance_id: String },
    PermissionDenied { operation: String },
    UntrustedInput { code: String },
    Storage { code: String },
    ExternalTool { tool_id: String, exit_code: Option<i32> },
    Network { code: String, retryable: bool },
    AuthenticationRequired,
    CryptoInvalid, // wrong password and tampering share safe message
    Cancelled,
}
~~~

AppError tiene implementación thiserror; Display estable y redactado; no deriva Serialize ni incluye errores de proveedor. Diagnostic {code, path:Option<PortablePath>, line:Option<u32>, column:Option<u32>, message, severity} es DTO serializable, con mensaje app-owned y bounded; nunca incluye contenido del secreto.

## Facts de plataforma e inicialización

Estos tipos viven en `jameskills-infra::platform`; no cambian el dominio ni
serializan paths de usuario a logs.

~~~rust
pub enum HostPlatform { Linux, Windows, Other }
pub enum Observation { Present, Absent, Unknown }
pub struct PlatformFacts {
    pub platform: HostPlatform,
    pub architecture: String,
    pub display_environment: Observation,
    pub gpu_device: Observation,
}
impl PlatformFacts { pub fn detect() -> Self; }
pub struct UserDirectories { pub config: PathBuf, pub data: PathBuf, pub cache: PathBuf }
pub enum PlatformError { BaseDirectoriesUnavailable }
pub fn resolve_user_dirs() -> Result<UserDirectories, PlatformError>;
~~~

`directories::BaseDirs` es la autoridad para XDG en Linux y Known Folders en
Windows; JameSkills agrega `jameskills` o `JameSkills` sin crear carpetas. Una
observación ausente no demuestra ausencia global de hardware ni de sesión, y
`Unknown` jamás se transforma en `Present` por defecto.

## Archivos portables, revisiones e instalación

`SkillId` y `OperationId` exponen `new()`, `parse(&str)` y `as_uuid()`; campos privados. `RevisionId` y `ContentHash` exponen `from_digest([u8; 32])`, `parse_hex(&str)` y `as_str()`; campos privados, lowercase exacto. Deserializar siempre pasa por los constructores validados.

`PortablePath` tiene campo privado y constructor `PortablePath::new(String) -> Result<Self, PathValidationError>`; expone `as_str()`. No implementa `From<String>` sin validación. Su algoritmo se define en SPEC-skill-format: NFC, separador `/`, componentes relativos no vacíos, sin `.`/`..`, backslash, colon, prefijos de unidad, nombres de dispositivo Windows ni trailing dot/space, máximo 240 bytes UTF-8. Deserialize vuelve a validar.

`SkillManifest` y `SkillFrontmatter` solo se construyen mediante los parsers
validados de `domain::skill`; tienen getters de solo lectura. `SkillManifest`
incluye schema_version, SkillId, slug/display_name/description/license,
`semver::Version` app y suite, PortablePath de policy/guidance, tags,
CapabilityDeclaration y extensions string-map. `SkillFrontmatter` retiene
source/body UTF-8 sin reserializar, el header estándar name/description,
compatibility y metadata string-map. `parse_manifest(&str)` y
`parse_frontmatter(&[u8])` devuelven `Result<T, Vec<Diagnostic>>` con códigos
propios y línea/columna cuando el parser provee span. `validate_skill_pair`
verifica slug/name, description y metadata JameSkills opcional. Entradas
externas no se convierten directamente a estos tipos por Deserialize.

Bundle { manifest: SkillManifest, frontmatter: SkillFrontmatter, files: BTreeMap<PortablePath, Vec<u8>>, trust: TrustState }.
TrustState = Quarantined | Reviewed. TrustState local, no autoridad obtenida de contenido importado.
RevisionRecord { id, skill_id, bundle_hash, parents: Vec<RevisionId>, kind: RevisionKind, semantic_version }.
RevisionKind = Content | Tombstone { observed_heads: Vec<RevisionId> }.
Revision parents ordenados únicos. Blob inmutable guarda bundle; registro de revisión separa padres, así un mismo contenido puede participar en merge distinto.
SaveRevisionRequest { bundle, expected_heads, parent_ids, reason }; SaveRevisionResult { revision, new_heads }.

AgentDetection { id, executable: Option<ApprovedExecutable>, version: Option<String>, profile_root, availability, capabilities: BTreeMap<CapabilityId, CapabilitySupport>, evidence }.
Availability = Missing | Candidate | Verified | Blocked.
CapabilitySupport = Supported | NeedsVerification | Unsupported; incluir source/date/tested_version metadata.
InstallPlan { operation_id, agent, scope, source_revision, source_hash, target, changes: Vec<FileChange>, required_checks, vendor_action?, fingerprint, expires_at }.
FileChange = Create | ReplaceOwned { before_hash, after_hash } | ConflictUnowned | ConflictEdited.
InstallReceipt { operation_id, agent, scope, skill_id, revision_id, target, files: Vec<InstalledFileHash>, vendor_package?, installed_at }.
ApprovedInstall { plan, confirmation_digest } se construye únicamente por UI/CLI tras selección explícita; no desde manifest.

## Ports

Ports object-safe async vía async-trait; core depende de serde, semver, uuid, sha2, thiserror, async-trait; nunca reqwest/tokio::fs/GPUI. Contexto de cancelación abstraído por CancellationPort o token pasado del aplicativo; no mezclar UI context con runtime.

~~~rust
#[async_trait::async_trait]
pub trait StoragePort: Send + Sync {
    async fn list_skills(&self, query: LibraryQuery) -> AppResult<LibraryPage>;
    async fn load_revision(&self, id: &RevisionId) -> AppResult<RevisionRecord>;
    async fn get_heads(&self, id: SkillId) -> AppResult<Vec<RevisionId>>;
    async fn commit_revision(&self, request: CommitRevision) -> AppResult<SaveRevisionResult>;
    async fn capture_snapshot(&self) -> AppResult<SnapshotPayload>;
    async fn merge_snapshot(&self, plan: MergePlan) -> AppResult<MergeResult>;
    async fn journal(&self, operation: OperationJournal) -> AppResult<()>;
}

#[async_trait::async_trait]
pub trait FileSystemPort: Send + Sync {
    async fn inspect_bundle(&self, root: &ApprovedRoot) -> AppResult<BundleEntries>;
    async fn stage_bundle(&self, bundle: &Bundle) -> AppResult<StagedBundle>;
    async fn read_blob(&self, hash: &ContentHash) -> AppResult<Vec<u8>>;
    async fn store_blob(&self, bundle: &Bundle) -> AppResult<ContentHash>;
    async fn apply_install(&self, plan: &ApprovedInstall) -> AppResult<InstallReceipt>;
    async fn recover_operations(&self) -> AppResult<Vec<RecoveryOutcome>>;
}

#[async_trait::async_trait]
pub trait ProcessPort: Send + Sync {
    async fn run(&self, spec: ProcessSpec) -> AppResult<ProcessOutput>;
}

#[async_trait::async_trait]
pub trait AgentPort: Send + Sync {
    async fn detect(&self, context: DetectionContext) -> AppResult<AgentDetection>;
    fn render_export(&self, bundle: &Bundle, scope: Scope) -> AppResult<AgentExport>;
    async fn verify_install(&self, receipt: &InstallReceipt) -> AppResult<Verification>;
}

#[async_trait::async_trait]
pub trait RemoteSnapshotPort: Send + Sync {
    async fn list_page(&self, page: Option<String>) -> AppResult<RemotePage>;
    async fn download(&self, file: &RemoteFile) -> AppResult<EncryptedSnapshot>;
    async fn upload(&self, snapshot: &EncryptedSnapshot) -> AppResult<RemoteFile>;
}

pub trait CryptoPort: Send + Sync {
    fn create_vault(&self, password: &SecretInput) -> AppResult<UnlockedVault>;
    fn unlock_vault(&self, data: &EncryptedSnapshot, password: &SecretInput) -> AppResult<UnlockedVault>;
    fn open_with_vault(&self, data: &EncryptedSnapshot, key: &UnlockedVault) -> AppResult<VerifiedSnapshot>;
    fn seal(&self, data: &SnapshotPayload, key: &UnlockedVault) -> AppResult<EncryptedSnapshot>;
    fn open(&self, data: &EncryptedSnapshot, password: &SecretInput) -> AppResult<VerifiedSnapshot>;
}
// Este Port es síncrono CPU; infrastructure lo ejecuta en spawn_blocking.
// VerifiedSnapshot constructor privado en core validation, no simple Deserialize.

#[async_trait::async_trait]
pub trait SecretStorePort: Send + Sync {
    async fn capabilities(&self) -> AppResult<SecretStoreCapabilities>;
    async fn store(&self, name: SecretName, value: SecretInput) -> AppResult<()>;
    async fn load(&self, name: SecretName) -> AppResult<SecretInput>;
    async fn delete(&self, name: SecretName) -> AppResult<()>;
}

pub trait ClockPort: Send + Sync {
    fn now_utc(&self) -> String;
    fn monotonic_ms(&self) -> u64;
}
~~~

ApprovedRoot, ApprovedExecutable y SecretInput tienen constructores controlados; SecretInput implementa zeroize/zeroize_on_drop y Debug = "[REDACTED]". No serializar SecretInput ni pasarlo como argv.
ProcessSpec { executable: ApprovedExecutable, tool_id, args: Vec<OsString>, cwd: ApprovedRoot, env: ApprovedEnv, timeout: Duration, output_limit_bytes, permission: ProcessPermission }.
ProcessPermission = ReadOnlyCheck | ExplicitMutation(OperationId). Allowlist driver's args verificada, logs solo tool_id/timing/exit.
ApprovedEnv mínimo; rutas PATH necesarias, HOME/USERPROFILE por driver, idioma fijo cuando parseador depende. No heredar XAI_API_KEY/GEMINI_API_KEY ni credenciales ajenas.
Windows .cmd de npm no se ejecuta como PE. Resolver wrapper conocido a node.exe+entrypoint aprobado cuando sea posible; fallback oficial específico explícito limitado y testeado, sin construir una línea arbitraria shell. tools/COMMITLINT y drivers git son del proyecto, no importados de skills.

## Servicios

El primer wiring de infraestructura publica `RuntimeServices { facts,
directories, clock }` mediante `infra::composition::build_services(dirs)`.
Valida que config/data/cache sean absolutas, distintas y no solapadas; no crea
directorios. Su `SystemClock` entrega UTC RFC3339 y elapsed monotonic local. El
factory se amplía con providers reales en sus tareas; todavía no promete ni
registra servicios de biblioteca, políticas, instalación o sync.

ApplicationServices conserva Arc<LibraryService>, Arc<PolicyService>, Arc<InstallService>, Arc<SyncService>, Arc<GuidanceService>. Para tests constructor recibe ports fake, sin init de SQLite/GPU/keyring.

Funciones públicas previstas:

| Módulo | Funciones y resultados |
|---|---|
| domain/skill | parse_frontmatter(bytes)->Result; parse_manifest(toml)->Result; validate_bundle(&Bundle)->Vec<Diagnostic>; hash_bundle(entries)->ContentHash; compute_revision(record)->RevisionId |
| domain/policy | parse_policy(bytes)->Policy; evaluate_predicate(check, observation)->CheckResult; strict_exit(report)->u8 |
| domain/guidance | next_step(plan, facts, evidence)->GuidanceDecision; validate_guidance_graph(plan)->Result |
| domain/sync | validate_snapshot(payload)->Result; merge_heads(local, remote, graph)->MergePlan; resolve_heads(choice)->RevisionRecord |
| application/library | create_skill(CreateSkill); save_draft(SaveDraft); publish(SaveRevisionRequest); import_bundle(ImportRequest); export_bundle(ExportRequest); delete_skill(DeleteRequest); list_skills(LibraryQuery) |
| application/policy | check(CheckRequest)->CheckReport; plan_repo_changes(RepoPolicyRequest)->RepoChangePlan; apply_repo_changes(ApprovedRepoChange)->ApplyResult |
| application/install | detect_agents(DetectionContext); plan_install(InstallRequest)->InstallPlan; apply_install(ApprovedInstall)->InstallReceipt; remove_installation(RemoveRequest)->RemovalResult |
| application/guidance | start_guidance(StartGuidance); advance(session_id, UserAnswer)->GuidanceDecision; recheck(session_id)->GuidanceProgress |
| application/sync | plan_remote_reset(ResetRequest)->RemoteResetPlan; apply_remote_reset(ApprovedReset)->ResetResult; connect(ConnectRequest); disconnect(DisconnectRequest); unlock(UnlockRequest); sync_once(SyncRequest)->SyncResult; preview_restore(RestoreRequest)->RestorePlan; apply_restore(ApprovedRestore)->RestoreResult |

No olvidar expected_heads/revision en Publish, ApplyInstall, RepoChange, ResolveConflict y Restore. Mutable operations tienen OperationId y journal.

## Reducer y ejemplos de TDD

~~~rust
pub fn strict_exit(results: &[CheckResult], required_ids: &[String]) -> u8 {
    let blocked = required_ids.iter().any(|id| {
        match results.iter().find(|r| &r.requirement_id == id) {
            Some(r) => !matches!(r.status, CheckStatus::Pass | CheckStatus::NotApplicable),
            None => true,
        }
    });
    if blocked { 1 } else { 0 }
}
~~~

NotApplicable solo permitido por condición declarada evaluada con facts confiables; no método para omitir cualquier requisito. Test: required unknown ->1; required missing ->1; warning no requerida no bloquea; required false scope ->NotApplicable permitido.

~~~rust
#[test]
fn concurrent_edits_keep_both_heads() {
    // Fixture: B parent A; C parent A; local=B, remote=C.
    let graph = revision_graph(&[("A", &[]), ("B", &["A"]), ("C", &["A"])]);
    let plan = merge_heads(&["B".into()], &["C".into()], &graph);
    assert_eq!(plan.heads, vec!["B".into(), "C".into()]);
    assert_eq!(plan.conflicts.len(), 1);
}
~~~

Test anterior es blueprint: helper revision_graph y tipos strings se adaptan a IDs válidos en fixture; nunca hardcodear resultado de merge para hacerlo pasar. Añadir descendant B->D produce solo D; tombstone observado B con C concurrente conserva C+delete conflict.

## CLI estable v1

Binario jameskills-cli, nombre mostrado jameskills. JSON wrapper {schema_version:1, command, operation_id?, data?, error?}. Exit:0 éxito;1 requisito obligatorio incumplido;2 argumentos/formato;3 entorno/permiso/auth;4 IO/red/crypto;130 cancelado.
- doctor --json
- validate --path <bundle> --json
- check --repo <path> --skill <uuid> --profile <rust|node|generic> --json --strict
- library list --json; library import --path <bundle|jskill> --json
- library export --skill <uuid> --output <path.jskill> --json
- agents detect --json
- install plan --skill <uuid> --agent <id> --scope <user|project> [--repo <path>] --output <plan.json>
- install apply --plan <plan.json> --confirm-digest <sha256>
- install remove --receipt <uuid> [--confirm-digest <sha256>]
- backup export --output <path.jskills-backup>; backup restore --input <path> --preview
- backup restore --input <path> --apply --confirm-digest <sha256>
- sync status --json; sync run --json

Plan JSON no constituye autorización ni prueba de integridad por sí mismo: validar paths/hash/version/fingerprint y nunca ejecutar fields arbitrarios. Passphrase por TTY oculto; CI cloud deshabilitado por defecto. No --password ni env con contraseña. JSON stdout redacted, diagnósticos stderr. CLI test dispatcher inyecta fake services y filesystem temporal.

## Administración remota separada

RemoteSnapshotAdminPort implementa async delete_file(&RemoteFile)->AppResult<()> en infra/google/drive.rs. SyncService recibe esta capacidad separada, solo invocada por ApprovedReset creado tras preview+recovery export verificado. ResetRequest selecciona vaultId actual y accountBindingId, nunca fileIds arbitrarios de skill. RemoteResetPlan contiene exact fileIds listados completos, ciphertext hashes conocidos, cutoff generation y digest; apply revalida lista y consentimiento concreto. Borrado parcial deja journal+estado visible y se reanuda solo IDs aprobados; nunca borrar nuevos snapshots que aparecieron después preview. No usar papelera appDataFolder.

Config persistencia local infra config validated; PreferencesController desktop guarda theme/language/schedule/toolpath de forma atómica usando infraestructura pública sin secretos, no importa core en sentido inverso. AppPaths override startup puede usar nuevo directorio tras guía export/restore, fuera mutación DB en uso.

## Contratos finales e introducción incremental

Estos son contratos de la v1 terminada. Introducir tipos/ports/métodos cuando exista su primer proveedor real y registrar módulo en el mismo incremento. No implementar métodos futuros devolviendo éxito ni declarar tipos no creados para aparentar factory completa. ApplicationServices y build_services se amplían con servicios reales en su slice; el shell temprano comunica capacidades todavía no implementadas, que desaparecen antes aceptación final. No usar Option sin estado visible como forma permanente de omitir funcionalidad. StoragePort::capture_snapshot se agrega con DTOs sync presentes y snapshot capture real, no antes de definirlos.

CheckResult.enforcement es autoridad OBSERVADA por evidencia, mientras requirement.enforcement es autoridad EXIGIDA. Un driver que valida un mensaje commit produce LocalCheck; no puede devolver LocalHook porque el manifest lo pidió. Para LocalHook validar configuración Git efectiva y hook gestionado que invoca el driver aprobado, incluido hash/permission/path. Para RequiredCi exigir proveedor que obliga el check más check-run SHA actual; workflow existente produce solo LocalCheck. Si autoridad observada no alcanza requerida, requisito permanece Fail/Unknown/Blocked según causa aunque el predicado de contenido pase. No usar comparaciones ordinales generales de Enforcement: cada variante tiene evidencia necesaria explícita.

UnlockedVault encapsula master_key + wrapping_key derivada + salt + vault_id. Solo estas llaves de sesión, nunca passphrase original, se cachean opt-in en keyring y se zeroizan al lock. CryptoPort.create_vault/unlock_vault/open_with_vault dan el wiring necesario para sellar nuevos snapshots con WrapAAD variable y nonce fresh sin conservar contraseña plaintext.
