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
    pub enforcement: Option<Enforcement>, // observado, no copiado del requirement
    pub evidence: Vec<CheckEvidence>,
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
Windows. Linux agrega `jameskills` a cada base; Windows usa RoamingAppData/
`JameSkills` para config y LocalAppData/`JameSkills/{Data,Cache}` para separar
datos y caché, aunque ambas bases Known Folder sean iguales. Resolver no crea
carpetas. Una observación ausente no demuestra ausencia global de hardware ni
de sesión, y `Unknown` jamás se transforma en `Present` por defecto.

### Tool detection y manifests de proyecto

Domain modela `ToolAvailability = Missing | Candidate | Verified | Blocked |
Unknown`, `ToolVersionStatus = Compatible | Incompatible | Unknown |
NotApplicable` y `ToolCapabilitySupport = Supported | NeedsVerification |
Unsupported`. `ToolDetection` conserva `tool_id`, disponibilidad, versión
SemVer opcional, status de versión, capacidades por `ToolOperation` y
`ToolEvidence { source_id, observed_at, tested_version, summary }`. Una versión
compatible de un Candidate no marca capabilities Supported; solo un chequeo
específico puede verificarlas. Resúmenes y source IDs son app-owned y acotados.

`infra::platform::load_tool_profiles() -> Result<Vec<ToolProfile>,
Vec<Diagnostic>>` carga exclusivamente `profiles/tools.toml` con schema cerrado,
argv fijo, rango SemVer y fuentes de guía registradas para Windows/Linux.
`find_tool_candidates(profiles, search_paths, platform) -> Vec<ToolCandidate>`
solo descubre candidatos; no los ejecuta y omite paths relativos. `.cmd` es
`CommandShim`, nunca un PE aprobado.

`infra::platform::detect_tools(search_paths, platform, approved_fingerprints,
cwd, environment, process, observed_at) -> AppResult<Vec<ToolDetection>>`
combina profiles y candidatos. Un tool ausente, candidato sin fingerprint o
wrapper bloqueado no genera spawn. Un candidato nativo se prueba únicamente
con `ExecutableFingerprint` explícito, argv del profile y `ProcessPort`; el
proveedor vuelve a calcular SHA-256 antes del spawn. Version output ausente o
no reconocido produce versión `Unknown`; una versión fuera del rango produce
`Incompatible`, sin elevar capabilities.

`infra::platform::inspect_project_manifests(root: &ApprovedRoot) ->
AppResult<ProjectManifestFacts>` solo lee `Cargo.toml` y `package.json` del root
seleccionado, máximo 1 MiB cada uno. `ProjectStack` es `Rust | Node |
RustAndNode | Generic | Unknown`. Nombres Node `scripts` pueden exponerse para
mapear comandos; valores nunca se devuelven ni ejecutan. Manifiestos malformed,
demasiado grandes, symlinks o archivos no regulares producen `Unknown`; ausencia
de ambos produce `Generic`. Ningún nombre/metadato de skill determina el stack.

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
El parser YAML usa `serde-saphyr` con budget estricto de profundidad, eventos,
documentos y bytes escalares; alias/anchor limitados a cero, duplicate keys y
merge keys como error, tags custom rechazadas y snippets desactivados.

`Policy` se construye con `domain::policy::parse_policy(&[u8])`; el DTO y sus
campos se exponen mediante getters. `Scope` es el tipo común `User | Project`.
Cada `Requirement` tiene id, descripción, `Severity`, required, `Phase`,
`Enforcement`, depends_on, guidance_id opcional, `applies_when` tipado opcional
y un `Check` tipado. `ApplicabilityFact` es `Os | Architecture | Stack | Host |
Context | Capability`; cada fact solo admite los valores registrados por el
parser y `Unknown` nunca satisface una condición. Check v1
admite git-repository, gitignore-patterns, tracked-secrets, readme-sections,
conventional-commit, github-access, protected-main-local, github-branch-policy,
ci-contract, ci-evidence, release-contract y toolchain-version. Los tool
requirements usan `ToolId` y `ToolOperation` de registry cerrado (incluido
GitHub `repository-read`), con `semver::VersionReq`; no
existe campo argv o shell en datos importados. El parser limita bytes/cantidades,
rechaza schema/campos/enums desconocidos, IDs duplicados, referencias de
dependencia ausentes/cíclicas, rangos inválidos y operaciones no autorizadas.
La existencia de policy/guidance paths y las referencias cruzadas entre archivos
se validan al ensamblar el bundle, no al parsear una policy aislada.

`validate_bundle` compila las policies y planes declarados a `ValidatedBundle`;
éste expone `policies() -> &[Policy]` y `guidance_plans() -> &[GuidancePlan]`
además de manifest/hash/count, pero no retiene bytes TOML crudos. `GuidancePlan`
y `GuidanceStep` son DTOs inmutables construidos por el parser: IDs únicos,
`requirement_ids` conocidos, pasos en DAG, `requires` dentro del plan y
`verification_requirement_ids` no vacíos y limitados a esos requisitos.
`applies_when` solo admite facts/enums registrados. Action es cerrada:
`ManualInstruction`, `OpenOfficialUrl` con source ID del registry,
`CopyApprovedCommand` con tool/operation registrados, `SelectLocalPath` con
purpose acotado, `AnswerChoice` con opciones acotadas o `Recheck`. Ninguna
acción ejecuta procesos, abre URLs arbitrarias ni contiene secretos.
Una capa de presentación resuelve `OpenOfficialUrl.source` por registry app-owned
y `CopyApprovedCommand` solo por un renderer exacto registrado de tool/operation;
si falta renderer la acción es Unsupported. El texto copiable son datos para el
usuario y no se pasa a ProcessPort ni modifica CheckResult. `SelectLocalPath`
solicita un picker, `AnswerChoice` solo conserva un choice registrado y
`Recheck` vuelve a consultar los providers; ninguna respuesta autoriza writes.

`infra::platform::render_guidance_action(&GuidanceAction) ->
RenderedGuidanceAction` transforma el enum cerrado en datos de presentación.
`OpenOfficialUrl` resuelve solo los cinco `OfficialGuidanceSource` app-owned.
`CopyApprovedCommand` tiene renderer únicamente para el par exacto
`(Git, RepositoryRoot)` y produce `git rev-parse --show-toplevel`; otras
combinaciones son `Unsupported`. El texto es solo copiable: este renderer no
recibe `ProcessPort`, no ejecuta comandos ni produce evidencia. Las acciones de
picker, choice, instrucción manual y recheck permanecen variantes de datos; el
host de UI decide cómo presentarlas sin elevar su autoridad.

`GuidanceFacts` contiene observaciones por `ApplicabilityFact`, cada una con
valor del registry y `CheckEvidence`; la colección está ligada a un environment
fingerprint. `GuidanceFactsProvider::observe_facts()` solo aporta facts realmente
medidos. Fact ausente, expirado o con fingerprint obsoleto es Unknown. El
planner puro
`next_step(plan, facts, reports, answers, now_monotonic_ms)->GuidanceDecision`
recorre steps topológicamente: applicability mismatch con fact fresco da
NotApplicable; prerequisites incompletos bloquean solo sus descendientes; un
step solo es Completed si todos sus verifier IDs tienen CheckResult Pass con
evidence presente/fresca. `GuidanceDecision` expone estado acotado por step y
como máximo el siguiente paso actionable.

`GuidanceService` recibe `Arc<PolicyService>`, `GuidanceFactsProvider` y
`ClockPort`. `start_guidance(Arc<ValidatedBundle>, plan_id)` captura facts y checks
para las policies del bundle. `advance(session_id, UserAnswer)` acepta solo
acknowledge o choice registrada y vuelve a calcular el planner; nunca altera
check status/evidence. `recheck(session_id)` vuelve a pedir facts y ejecutar
PolicyService; reemplaza reports anteriores. Si cambia el fingerprint borra
answers del plan. Hay como máximo 64 sesiones vivas; `close_session` libera una.
Session progress es process-local en este slice; no se declara persistencia
aunque exista una tabla reservada en el esquema SQLite.

`Check::CiContract { workflow_paths, required_jobs }` trata `required_jobs` como
IDs de `jobs`, no como display names de status checks. Inspecciona solo paths bajo
`.github/workflows/` con extensión `.yml` o `.yaml` del root aprobado, máximo 8
archivos y 256 KiB por workflow, con `serde-saphyr` y budgets estrictos. Requiere triggers `push` y
`pull_request`, los jobs declarados por el profile cubiertos por ambos eventos,
permisos explícitos con keys del registry GitHub y sin grants `write`, refs SHA
completos/digest para Actions externas y ausencia de `continue-on-error` en
jobs/steps requeridos. Un filtro de paths o activity-types que omita `pull_request`,
una gate/job/runner/needs ausente o permisos write es Fail conocido; YAML
malformado, permisos implícitos/desconocidos, branch filter sin protected-branch
scope, condición dinámica, reusable workflow requerido o sintaxis fuera del
subset es Unknown. La lectura no ejecuta YAML, Actions ni steps. El resultado
satisfactorio es `LocalCheck` únicamente; no prueba provider remoto, branch
protection/ruleset ni check-runs para SHA actual. `CiEvidence` permanece Unknown
hasta que exista el provider host/SHA exacto correspondiente.

`CheckEvidence` lleva source_id, RFC3339 UTC, revision opcional, fingerprint
`sha256:` y resumen app-authored `String` acotado a 256 bytes; se permite
contenido dinámico saneado para ligar una observación remota a repository/ref/check.
El driver no copia valores crudos de proveedor, URLs, identidad ni secretos. Su
expiry monotónica es válida solo en el proceso que la observó. CheckObservation
sin evidencia nunca produce Pass; evidencia expirada convierte el resultado
en Unknown.

~~~rust
pub enum CheckStatus { Pass, Fail, Blocked, Unknown, Unsupported, NotApplicable }
pub struct CheckObservation { status, enforcement: Option<Enforcement>, evidence: Vec<CheckEvidence> }
pub struct CheckReport { results: Vec<CheckResult>, required_ids: BTreeSet<String> }
pub fn evaluate_predicate(
    requirement: &Requirement,
    observation: &CheckObservation,
    now_monotonic_ms: u64,
) -> CheckResult;
pub fn strict_exit(report: &CheckReport) -> u8;
~~~

`TestSuiteRunResult` separa `TestSuiteDeclaration` (`Declared | Missing |
Unknown`) de `TestSuiteExecution` (`NotRun | Blocked | Passed | Failed`) y
`exit_code`. Solo `Passed` acepta exit 0 y `Failed` acepta exit distinto de 0;
`NotRun`/`Blocked` no tienen exit code y una suite `Missing` no puede marcarse
ejecutada. La ejecución es una acción explícita, separada de `PolicyService::check`.
`TestSuiteRunApproval::after_explicit_trust_confirmation` es un DTO app-owned,
no deserializable, que liga `ApprovedRoot`, suite fija, `RepositoryHead`, hash
de manifests y `OperationId`; por sí solo no ejecuta procesos ni representa
evidencia de suite exitosa. El servicio/runner concreto debe revalidar
root/head/manifiestos justo antes del spawn; policy inspection nunca invoca el
runner.

El provider Cargo obtiene la declaración mediante `cargo metadata --no-deps
--format-version 1 --locked --offline`; solo considera `packages` cuyos IDs están
en `workspace_members`, y declara suite si alguno tiene `targets[].test = true`.
Esto constata targets seleccionables por Cargo, no que las pruebas pasen ni una
cobertura determinada. Tras aprobación explícita el argv fijo ejecuta
`cargo test --workspace --locked --manifest-path <root>/Cargo.toml`, es decir,
los targets de test de los miembros del workspace. La inspección usa
`ReadOnlyCheck`; la ejecución aprobada usa `ExplicitMutation(OperationId)`, con
timeout/salida limitados y revalidación del root, HEAD y fingerprint de los
manifiestos antes del spawn. Cargo exit 0 se registra como Passed; exit no cero
como Failed (puede representar fallo de compilación o de prueba); spawn/tool no
disponible o cancelación no se convierte en Pass.

`PolicyCheckProvider::observe(&Requirement)` es async e inyectado a
`PolicyService::new(provider, clock)`. `PolicyService::check(CheckRequest)` es async y evalúa
todos los requisitos, conserva autoridad observada aparte de la exigida y devuelve
guidance_id estructurado. Unknown/Blocked/Fail/Unsupported requerido y cualquier
resultado ausente dan strict exit 1; un resultado opcional no bloquea. `NotApplicable`
solo nace de mismatch de un `applies_when` registrado con fact evidence fresca;
una respuesta del provider que diga NotApplicable sin ese fundamento queda Unknown.

`RepositoryPolicyCheckProvider` implementa `Check::CiContract` como un check local
de datos, no como una ejecución de GitHub Actions. Antes de inspeccionar workflows,
requiere Git nativo/fingerprinted con versión registrada compatible y al menos un
remote configurado por `git remote -v` cuyo fetch/push host sea exactamente
`github.com`; el comando solo lee config local, bounded, sin red. GitLab, GitHub
Enterprise no registrado, URLs no analizables, mezcla de hosts o ausencia de
remotes deja resultado Unknown; esto tampoco prueba que el repositorio exista en
el host ni que las reglas de branch estén activas. URLs, incluidas credenciales,
no se copian a evidencia ni logs. El check solo admite workflow paths bajo
`.github/workflows/` con extensión `.yml` o `.yaml`, máximo 8 archivos y 256 KiB por archivo; rechaza
symlinks/archivos no regulares antes de parsear. `serde-saphyr` recibe un budget
cerrado (1 documento, depth 32, 8,192 nodos, 16,384 eventos, scalar bytes bounded,
sin aliases/anchors/merge keys/custom tags/duplicate keys ni snippets). El parser
comprueba `push` y `pull_request` (con `opened` y `synchronize` si hay activity
filters), jobs/runner/needs declarados, los jobs solicitados presentes en ambos
eventos, permisos explícitos registrados sin `write`, pinned refs para Actions
externas, y que jobs/steps requeridos no habiliten `continue-on-error`. Condiciones
de job/step no conocidas, reusable workflows requeridos o formas fuera del subset dan Unknown;
fallas comprobables del contrato dan Fail. Pass tiene autoridad `LocalCheck`.
No inspecciona protección/rulesets del host ni un check-run para SHA actual; `ci-evidence`
y `RequiredCi` permanecen Unknown/Blocked hasta tener esos proveedores remotos.

`infra::fs::ApprovedRepositoryTool::new(executable, fingerprint)` y
`RepositoryPolicyCheckProvider::new(root, git, gitleaks, environment, process,
clock, environment_fingerprint)` conectan checks locales al service. El provider
implementa readme-sections vía AST Markdown bounded, gitignore-patterns con Git
`check-ignore --no-index -v -z` sobre rutas sintéticas registradas y tracked-secrets
con Gitleaks redacted/fingerprint-checked. El Gitleaks profile actual escanea el
working tree solo con Gitleaks 8.30.1, la única versión validada, y un `--config`
privado app-owned que fuerza `useDefault=true`; la
config `.gitleaks.toml` del repo no determina las reglas. El staging se limpia al
terminar incluso si falla el spawn. Si existe `.gitleaksignore`, el check queda
Blocked antes de lanzar procesos porque el driver oficial también la aplica desde
el target y no ofrece un bypass independiente. `include_history=true` devuelve
Unsupported hasta que el ejecutable Git hijo tenga un driver/identidad aprobada
independiente. Ningún output crudo se copia a CheckEvidence.

`RepositoryPolicyCheckProvider::with_commitlint` acepta un `ApprovedCommitlint`
nativo o `ApprovedCommitlintNode`. El route Node exige Node nativo con fingerprint
aprobado, Node dentro del rango app-owned y >=22.12.0, y entrypoint aprobado cuyo
path termina en `node_modules/@commitlint/cli/cli.js`; valida con ese entrypoint
la versión exacta `@commitlint/cli@21.2.2`. `ProcessSpec` revalida los fingerprints
del runtime y del script antes de spawn. No se ejecuta el `.cmd` de npm ni se usa
shell.

`LocalFileSystem::check_conventional_commit(root, git, commitlint, environment,
process, observed_at, environment_fingerprint)` lee solo el mensaje HEAD con
Git `--no-pager log -1 --format=%B` (máximo 64 KiB), lo escribe junto con una
config JSON app-owned vacía en un directorio privado fuera del repo y ejecuta el
CLI Commitlint 21.2.2 con `--default-config --config <private-json> --edit
<private-message>`. El cwd privado y `--config` explícito evitan ejecutar
configuración del proyecto. Para el route Node, el proceso conserva cwd privado
y pasa `--cwd <repo-root>` al CLI: el `--edit` oficial exige resolver el root
Git, mientras `--config` absoluto obliga a cargar solo el JSON privado. El PATH
antepone la carpeta del Git aprobado para la llamada interna `git config
core.commentChar`. Las rutas Windows `\\?\` se normalizan solo en argv Node;
los paths originales permanecen aprobados y fingerprinted por el ProcessPort.
Este límite está respaldado por el loader `load-config.ts` y el lector
`get-edit-commit.ts` oficiales de Commitlint v21.2.2, registrados en
`docs/SOURCES.md`.
Exit 0/1 significa Pass/Fail como LocalCheck; otro código o CLI no registrado
queda Blocked. Message, stdout y stderr no se copian a evidencia. El fingerprint
del entrypoint no equivale a una auditoría completa del árbol de dependencias Node.
Tras exit 0/1, el provider consulta read-only `git rev-parse --git-path
hooks/commit-msg` usando el mismo Git aprobado/fingerprinted y el root aprobado.
La evidencia del hook nunca eleva la autoridad: no se ejecuta el hook. Solo se
inspeccionan archivos regulares, bounded (16 KiB), no-symlink y dentro del root;
en Unix también se comprueba el bit executable. El contenido se lee dos veces y
se comparan sus SHA-256 para detectar cambios durante la lectura. La ruta, el
digest y el contenido del hook no se copian a logs/evidencia. Incluso con target instalado,
la evidencia indica que argv/driver identity e invocación no están probados y el
resultado continúa como `LocalCheck`: el bootstrap de Husky y comandos npm/shell
no prueban que se invocó el entrypoint Commitlint aprobado. `LocalHook` requiere
un contrato aparte que enlace configuración efectiva, script gestionado, argv y
fingerprints del driver; sigue siendo eludible con `--no-verify`/`HUSKY=0`.

El runner Node previsto mapea `NodeLint | NodeTest | NodeBuild` a las claves root
`lint`, `test` y `build` detectadas como datos; nunca recibe un nombre de script
ni argumentos libres del caller. Debe ejecutar npm CLI `11.16.0` cargando su
`npm-cli.js` mediante el Node nativo aprobado, no `npm.cmd`; Node debe satisfacer
el engine declarado por esa versión de npm (`^20.17.0 || >=22.9.0`), dentro del
rango de Node admitido por el profile. Runtime y entrypoint npm deben fingerprintarse
y volverse a validar antes del spawn; esta huella no verifica todos los módulos
relativos del paquete npm.

La invocación definida para el driver fijo `npm run-script <suite>` con `--prefix` al root aprobado,
`--workspaces=false`, `--ignore-scripts`, un `--script-shell` del sistema fijado
por plataforma, y `--userconfig`/`--globalconfig` dirigidos a archivos privados
vacíos fuera del repo. npm documenta que `--ignore-scripts` suprime los hooks
`pre<event>`/`post<event>` pero ejecuta el script solicitado; npm ejecuta ese
texto mediante `/bin/sh` en POSIX o `cmd.exe` en Windows. Por tanto la aprobación
explícita es consentimiento para ejecutar código del repositorio bajo su shell
de plataforma, no una sandbox ni un argv extraído de `package.json`. La implementación
no pasará texto de scripts como argumento ni iniciará el shim `.cmd`. Si existe
`.npmrc` en el root, la ejecución deberá quedar Blocked; el runner no debe
incorporar credenciales de config npm del proyecto/usuario, ni importar los valores de `scripts` a logs o
evidencia. Root, HEAD, manifests, declaración y selección se revalidan antes de
`ExplicitMutation`; cancelación, npm no disponible, script ausente y exit no
cero mantienen estados separados. Un exit 0 certifica exit del script solicitado,
no cobertura ni que el script haya probado un objetivo específico.
`infra::fs::npm_cli_entrypoint_for_candidate(&ToolCandidate) -> Option<PathBuf>`
resuelve layouts registrados de npm sin ejecutar el candidate launcher. El
provider requiere un `ApprovedNodeNpm` que ata un Node aprobado a ese entrypoint
JavaScript y su fingerprint; el ProcessPort vuelve a validar ambos antes de
spawn. `.npmrc` root presente o no regular bloquea, y el fingerprint de npm-cli.js
no acredita integridad del conjunto de módulos npm cargados.

Bundle { manifest: SkillManifest, frontmatter: SkillFrontmatter, files: BTreeMap<PortablePath, Vec<u8>>, trust: TrustState }.
`BundleEntry { path: PortablePath, kind: EntryKind, compressed_bytes: u64, uncompressed_bytes: u64 }` modela metadatos no confiables. `validate_bundle_inventory(&[BundleEntry]) -> Result<ValidatedInventory, Vec<Diagnostic>>` es lógica pura: limita 20MiB/2000 entries/2MiB por texto/256KiB SKILL, permite solo archivos regulares, rechaza duplicate/case-fold path collisions; nunca accede al filesystem. `ValidatedInventory` y sus entries tienen campos privados. `EntryKind` incluye file, directory, symlink, hardlink y reparse point para rechazar todos salvo regular file.
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

`FileSystemPort::inspect_bundle` y `stage_bundle` implementan la capa OS real por
encima del inventario puro: canonical root + relative handles no-follow, chain de
ancestros, sin symlinks/hardlinks/reparse points, contabilización streaming de ZIP
central+entries, límites antes de reservar/extractar y staging privado. Cualquier
entrada inválida detiene la operación sin escribir destino.

ApprovedRoot, ApprovedExecutable y SecretInput tienen constructores controlados; SecretInput implementa zeroize/zeroize_on_drop y Debug = "[REDACTED]". No serializar SecretInput ni pasarlo como argv.
`ExecutableFingerprint` encapsula el SHA-256 de un ejecutable revisado explícitamente.
`ProcessSpec` puede llevar `approved_executable_fingerprint: Option<ExecutableFingerprint>`;
el proveedor vuelve a calcularlo con lectura limitada antes de spawn. Los probes de
tools registrados no ejecutan candidatos sin fingerprint aprobado; presencia o PATH
por sí solos solo producen `Candidate`.
ProcessSpec { executable: ApprovedExecutable, tool_id, args: Vec<OsString>, cwd: ApprovedRoot, env: ApprovedEnv, timeout: Duration, output_limit_bytes, permission: ProcessPermission, approved_executable_fingerprint: Option<ExecutableFingerprint>, approved_script: Option<(ApprovedScript, ExecutableFingerprint)> }.
`SystemProcessPort` permite `ReadOnlyCheck` y `ExplicitMutation(OperationId)`;
ambos lanzan solo el executable aprobado, argv separados, cwd/environment
aprobados, fingerprints y límites bounded, y cancelan el grupo completo. El ID
no constituye por sí solo trust/consent del repositorio: el driver de suites
debe exigir la aprobación tipada explícita y construir argv desde registry antes
de solicitar `ExplicitMutation`. Ningún policy inspection solicita esa acción.
`ApprovedEnv` es mínimo y acepta las rutas `PATH`, `HOME`/`USERPROFILE` por
driver e idioma fijo cuando el parser lo requiere. En Windows se puede aprobar
`APPDATA` para que el `gh` fijado lea su configuración/auth propios; no se
propagan `GH_HOST`, `GH_TOKEN` ni `GITHUB_TOKEN`. Para toolchain MSVC también
puede conservar `INCLUDE`, `LIB`, `LIBPATH`, `VCINSTALLDIR`, `VCToolsInstallDir`,
`WindowsSdkDir`, `WindowsSDKVersion`, `UniversalCRTSdkDir` y `UCRTVersion`, que
ubican compilador, headers y bibliotecas del SDK. Estos nombres se allowlistean
individualmente; `CL` y `_CL_` permanecen rechazados porque permiten añadir
argumentos al compilador/linker mediante el entorno. Nunca heredar
`XAI_API_KEY`/`GEMINI_API_KEY` ni credenciales ajenas.
`ProcessSpec::with_approved_script(ApprovedScript, ExecutableFingerprint)` ata
un entrypoint JavaScript aprobado al runtime aprobado. `SystemProcessPort`
revalida que siga siendo un archivo regular, canónico y con el mismo SHA-256
inmediatamente antes del spawn. El contrato cubre el entrypoint; no sustituye
la verificación del paquete/dependencias declarados por el driver.
Windows .cmd de npm no se ejecuta como PE. Resolver wrapper conocido a node.exe+entrypoint aprobado cuando sea posible; fallback oficial específico explícito limitado y testeado, sin construir una línea arbitraria shell. tools/COMMITLINT y drivers git son del proyecto, no importados de skills.

## Servicios

El primer wiring de infraestructura publica `RuntimeServices { facts,
directories, clock, library, policy }` mediante `infra::composition::build_services(dirs)`.
Valida que config/data/cache sean absolutas, distintas y no solapadas; no crea
directorios. Su `SystemClock` entrega UTC RFC3339 y elapsed monotonic local.
PolicyService usa un provider conservador que devuelve Unknown hasta que existan
providers reales; ausencia de driver nunca se reporta como Pass.

ApplicationServices conserva Arc<LibraryService>, Arc<PolicyService>, Arc<InstallService>, Arc<SyncService>, Arc<GuidanceService>. Para tests constructor recibe ports fake, sin init de SQLite/GPU/keyring.

Funciones públicas previstas:

| Módulo | Funciones y resultados |
|---|---|
| domain/skill | parse_frontmatter(bytes)->Result; parse_manifest(toml)->Result; validate_bundle(&Bundle)->Vec<Diagnostic>; hash_bundle(entries)->ContentHash; compute_revision(record)->RevisionId |
| domain/policy | parse_policy(bytes)->Policy; evaluate_predicate(requirement, observation, now_monotonic_ms)->CheckResult; strict_exit(report)->u8 |
| domain/guidance | next_step(plan, facts, evidence)->GuidanceDecision; validate_guidance_graph(plan)->Result |
| domain/sync | validate_snapshot(payload)->Result; merge_heads(local, remote, graph)->MergePlan; resolve_heads(choice)->RevisionRecord |
| application/library | create_skill(CreateSkill); save_draft(SaveDraft); publish(SaveRevisionRequest); import_bundle(ImportRequest); export_bundle(ExportRequest); delete_skill(DeleteRequest); list_skills(LibraryQuery) |
| application/policy | PolicyService::check(CheckRequest)->AppResult<CheckReport> (async) |
| application/repo_change | RepositoryChangeService::plan_repo_changes(RepoPolicyRequest)->AppResult<RepoChangePlan>; apply_repo_changes(ApprovedRepoChange)->AppResult<ApplyResult> (async) |
| application/install | detect_agents(DetectionContext); plan_install(InstallRequest)->InstallPlan; apply_install(ApprovedInstall)->InstallReceipt; remove_installation(RemoveRequest)->RemovalResult |
| application/guidance | start_guidance(Arc<ValidatedBundle>, plan_id)->GuidanceProgress; advance(session_id, UserAnswer)->GuidanceProgress; recheck(session_id)->GuidanceProgress; close_session(session_id) |
| application/sync | plan_remote_reset(ResetRequest)->RemoteResetPlan; apply_remote_reset(ApprovedReset)->ResetResult; connect(ConnectRequest); disconnect(DisconnectRequest); unlock(UnlockRequest); sync_once(SyncRequest)->SyncResult; preview_restore(RestoreRequest)->RestorePlan; apply_restore(ApprovedRestore)->RestoreResult |

`GuidanceFacts` conserva por `ApplicabilityFact` un valor registrado, su
`CheckEvidence` fresca y un environment fingerprint. `next_step(plan, facts,
reports, answers, now_monotonic_ms)` calcula el orden topológico y devuelve
`GuidanceDecision` + estados por step. Fact absent/expired es Unknown, mismatch
fresh es NotApplicable; requisito de verificación solo completa un step con
`CheckStatus::Pass`, evidencia presente y no expirada. Dependientes quedan
pendientes cuando un prerequisito no pasa, sin bloquear ramas independientes.

`GuidanceService` toma un `Arc<ValidatedBundle>`, ID de plan y un
`GuidanceFactsProvider`; usa el `PolicyService` del bundle para observar sus
requisitos. `start_guidance` crea un `OperationId` de sesión; `advance` admite
solo `Acknowledge(step_id)` o `Choose(step_id, choice)` válidos para la acción y
`recheck` vuelve a observar facts/checks y reemplaza los reports previos. User
answers no son evidence ni alteran status; al cambiar el environment fingerprint
se limpian answers y resultados dependientes. Las sesiones de este slice son
acotadas y process-local; no se afirma persistencia durable en `guidance_sessions`
hasta conectar un StoragePort de sesión.

### Cambios aprobados de repositorio

`RepositoryChangeService::plan_repo_changes(RepoPolicyRequest) -> RepoChangePlan`
y `apply_repo_changes(ApprovedRepoChange) -> ApplyResult` pertenecen a un
servicio de aplicación dedicado, no a `GuidanceService` ni al check-only
`PolicyService`. El request acepta solo
un `RepoTemplateId` app-owned (nunca bytes, rutas o comandos del bundle) y queda
vinculado al `ApprovedRoot` seleccionado y `RepositoryHead` observado. El plan
expone targets portables, diff bounded, hash anterior/ausencia esperada, hash del
contenido nuevo y digest de confirmación sobre root/head/target/estados. La
confirmación es un DTO no deserializable ligado al plan y su `OperationId`.

Apply revalida root, HEAD y estado/hash previo inmediatamente antes de escribir;
un conflicto deja intacto el destino. Stage vive en el mismo filesystem del
target, y un journal durable en `operations/` registra fases/hashes sin contenido
privado. Recovery solo limpia o restaura staging/backup cuando owner y hashes
siguen siendo los registrados; cambios concurrentes quedan visibles como
conflicto, nunca se sobrescriben. Commit no hace git add/commit/push, no ejecuta
templates ni instala/activa hooks. Hooks, si se añade un template local, son
opcionales y eludibles. Contenido importado es dato inerte: no puede ampliar el
registry de templates/actions ni generar argv.

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

`doctor --json` incluye facts de plataforma y detección bounded de profiles de
`profiles/tools.toml`. Solo inspecciona candidatos nativos/shims/Missing con
`find_tool_candidates`; no ejecuta candidatos, instala tools ni revela paths.
Native candidate sin fingerprint sigue Candidate/NeedsVerification y no tiene
versión observada. Missing/Blocked puede incluir el install-guide ID/URL del
registry para acción manual; guía no equivale a instalación ni a capability
verificada. Respuesta mantiene el envelope/exit-code documentado.

Plan JSON no constituye autorización ni prueba de integridad por sí mismo: validar paths/hash/version/fingerprint y nunca ejecutar fields arbitrarios. Passphrase por TTY oculto; CI cloud deshabilitado por defecto. No --password ni env con contraseña. JSON stdout redacted, diagnósticos stderr. CLI test dispatcher inyecta fake services y filesystem temporal.

## Administración remota separada

RemoteSnapshotAdminPort implementa async delete_file(&RemoteFile)->AppResult<()> en infra/google/drive.rs. SyncService recibe esta capacidad separada, solo invocada por ApprovedReset creado tras preview+recovery export verificado. ResetRequest selecciona vaultId actual y accountBindingId, nunca fileIds arbitrarios de skill. RemoteResetPlan contiene exact fileIds listados completos, ciphertext hashes conocidos, cutoff generation y digest; apply revalida lista y consentimiento concreto. Borrado parcial deja journal+estado visible y se reanuda solo IDs aprobados; nunca borrar nuevos snapshots que aparecieron después preview. No usar papelera appDataFolder.

Config persistencia local infra config validated; PreferencesController desktop guarda theme/language/schedule/toolpath de forma atómica usando infraestructura pública sin secretos, no importa core en sentido inverso. AppPaths override startup puede usar nuevo directorio tras guía export/restore, fuera mutación DB en uso.

## Contratos finales e introducción incremental

Estos son contratos de la v1 terminada. Introducir tipos/ports/métodos cuando exista su primer proveedor real y registrar módulo en el mismo incremento. No implementar métodos futuros devolviendo éxito ni declarar tipos no creados para aparentar factory completa. ApplicationServices y build_services se amplían con servicios reales en su slice; el shell temprano comunica capacidades todavía no implementadas, que desaparecen antes aceptación final. No usar Option sin estado visible como forma permanente de omitir funcionalidad. StoragePort::capture_snapshot se agrega con DTOs sync presentes y snapshot capture real, no antes de definirlos.

CheckResult.enforcement es autoridad OBSERVADA por evidencia, mientras requirement.enforcement es autoridad EXIGIDA. Un driver que valida un mensaje commit produce LocalCheck; no puede devolver LocalHook porque el manifest lo pidió. Para LocalHook validar configuración Git efectiva y hook gestionado que invoca el driver aprobado, incluido hash/permission/path. Para RequiredCi exigir proveedor que obliga el check más check-run SHA actual; workflow existente produce solo LocalCheck. Si autoridad observada no alcanza requerida, requisito permanece Fail/Unknown/Blocked según causa aunque el predicado de contenido pase. No usar comparaciones ordinales generales de Enforcement: cada variante tiene evidencia necesaria explícita.

UnlockedVault encapsula master_key + wrapping_key derivada + salt + vault_id. Solo estas llaves de sesión, nunca passphrase original, se cachean opt-in en keyring y se zeroizan al lock. CryptoPort.create_vault/unlock_vault/open_with_vault dan el wiring necesario para sellar nuevos snapshots con WrapAAD variable y nonce fresh sin conservar contraseña plaintext.
