# Arquitectura, encapsulamiento y relaciones de archivos

## Decisiones

Rust workspace edición 2024, cuatro crates; dominio/application sin GUI ni proveedores. SQLite es índice transaccional; revisiones inmutables viven en content-addressed storage. GPUI Kit =0.7.0 es la única dependencia de UI. Un CLI usa los mismos casos de uso para CI/headless. Infra realiza IO a través de ports; las vistas solo envían Commands y reciben UiEvents.

~~~mermaid
flowchart TD
  GUI[jameskills-desktop: entidades y vistas] --> APP[jameskills-core: ApplicationServices]
  CLI[jameskills-cli: comandos y JSON] --> APP
  APP --> DOMAIN[domain: formato, políticas, DAG, guía]
  APP --> PORTS[ports: contratos IO]
  INFRA[jameskills-infra: adapters y factory] -. implements .-> PORTS
  GUI --> FACTORY[infra::composition::build_services]
  CLI --> FACTORY
  FACTORY --> APP
  INFRA --> FS[Filesystem + SQLite]
  INFRA --> AGENT[CLI agentes + tools aprobadas]
  INFRA --> REMOTE[Drive + GitHub]
  INFRA --> SECRET[Keyring + crypto]
~~~

No dependencias circulares. Infra depende de core. Desktop y CLI dependen de core e infra; core jamás importa infra/GPUI. No dinámicas de plugins ejecutables Rust/JS, no service locator global.

## Árbol objetivo

~~~
Cargo.toml
Cargo.lock
rust-toolchain.toml
AGENTS.md
README.md
CONTRIBUTING.md
SECURITY.md
CHANGELOG.md
LICENSE
THIRD-PARTY-NOTICES.md
.gitignore
.gitattributes
deny.toml
cliff.toml
commitlint.config.cjs
package.json                         # solo commitlint, sin frontend JS
package-lock.json
.github/
  workflows/{ci,security,release}.yml
  pull_request_template.md
  CODEOWNERS
  dependabot.yml
packaging/
  windows/{jameskills.wxs,README.md}
  linux/{jameskills.desktop,README.md}
  assets/{app.ico,app.png}
profiles/tools.toml                   # registro construido por el aplicativo
examples/repository-foundation/       # fixture suite oficial de JameSkills
docs/                                # dossier actual + evidencia futura
tasks/{plan,todo,HANDOFF-LUNA}.md
crates/
  jameskills-core/
    Cargo.toml
    src/{lib,error}.rs
    src/domain/{mod,ids,skill,policy,agent,library,sync,guidance}.rs
    src/application/{mod,library,policy,install,sync,guidance}.rs
    src/ports/{mod,storage,filesystem,process,agent,remote,crypto,secrets,clock}.rs
    tests/{format,policy,guidance,library,install,sync}.rs
  jameskills-infra/
    Cargo.toml
    src/{lib,composition,sqlite,fs,process,platform,crypto,keyring,github}.rs
    src/agents/{mod,codex,opencode,pi,antigravity,grok}.rs
    src/google/{mod,oauth,drive}.rs
    migrations/{001_library,002_operations,003_sync}.sql
    tests/{fs_security,storage,install_recovery,agents,oauth,drive,crypto,github}.rs
  jameskills-cli/
    Cargo.toml
    src/{main,commands,output}.rs
    tests/{cli,ci_guard}.rs
  jameskills-desktop/
    Cargo.toml
    src/{main,lib,composition,bridge,state,routes,theme}.rs
    src/views/{mod,onboarding,library,skill_editor,checks,agents,sync,settings}.rs
    src/components/{mod,requirement_card,operation_preview,conflict_dialog,status_bar}.rs
    tests/{routing,library_flow,install_flow,guidance_flow,sync_flow}.rs
tests/fixtures/{skills,agents,archives,remote,snapshots}/
~~~

Un módulo puede subdividirse al crecer; mantener reexports y tests. El catálogo de tareas limita cada incremento a 3–5 archivos; registrar módulos en una tarea de scaffolding evita modificar lib.rs en cada trabajo. No escribir todo el árbol vacío para aparentar implementación.

## Datos y rutas

AppPaths via directorios de SO:
- Linux: XDG_DATA_HOME/jameskills o ~/.local/share/jameskills; XDG_CONFIG_HOME/jameskills o ~/.config/jameskills; XDG_CACHE_HOME/jameskills o ~/.cache/jameskills.
- Windows: Known Folders RoamingAppData/JameSkills para configuración; LocalAppData/JameSkills para datos/cache. Resolver API/directories, no interpolar C:\\Users literal.
- Datos: library.sqlite3; blobs/<sha256-prefix>/<sha256>.bundle; staging/<op-uuid>/; recovery/<op-uuid>/; operations/ solo journal no secretos.
- Settings públicos: settings.toml con theme, language, tool-paths autorizadas, sync schedule, OAuth client_id; nunca refresh token/passphrase.
- UUID dispositivo y receipts: solo locale, excluidos de sync.
- Paths agente conservan convención del agente: no trasladar ~/.pi/.agents/.grok a AppData porque los datos propios de JameSkills sí viven ahí.

## Persistencia SQL

Migrations con SQL explícito y transacción; schema_version SQLite user_version. WAL, foreign_keys=ON, busy_timeout=5s, single writer async actor para no bloquear UI. Upgrade prueba DB antigua -> nueva; backup antes de upgrade destructivo; esquema futuro abre en modo seguro sin escribir.

Tablas:
- skills(id TEXT PK, slug TEXT, display_name TEXT, created_at TEXT); slug no necesariamente único porque se pueden importar dos UUID con mismo nombre, colisión visible.
- revisions(id TEXT PK sha256, skill_id FK, bundle_hash TEXT, semantic_version TEXT, schema_version INTEGER, state TEXT, created_at TEXT). PK revision = hash de bytes canónicos.
- revision_parents(revision_id FK, parent_revision_id TEXT, PRIMARY KEY ambos); parent puede aún no estar descargado, no FK forzada a revisions.
- skill_heads(skill_id FK, revision_id FK, PRIMARY KEY ambos). >1 heads = conflicto.
- drafts(skill_id PK, base_head TEXT nullable, draft_json BLOB, generation INTEGER); locales no se sincronizan.
- deletions(skill_id TEXT, deletion_revision_id TEXT PK, observed_heads_json BLOB); borrado causal representado también en DAG.
- operations(id PK, kind TEXT, state TEXT, journal_json BLOB, updated_at TEXT). Sin tokens/llaves.
- installations(id PK, agent TEXT, scope TEXT, target_path TEXT, skill_id TEXT, revision_id TEXT, receipt_json BLOB). Un archivo compartido puede tener múltiples asociaciones pero un owner.
- remote_files(account_binding_id TEXT, snapshot_id TEXT, remote_file_id TEXT, ciphertext_hash TEXT, PRIMARY KEY account+remote_file_id).
- snapshots(id TEXT PK UUID, vault_id TEXT, encrypted_cache_path TEXT nullable, metadata_json BLOB, sync_state TEXT).
- guidance_sessions(id PK, skill_id TEXT, revision_id TEXT, environment_fingerprint TEXT, progress_json BLOB). Evidence redacted, expiración.

Transaction API no devuelve una conexión global. storage.commit_revision(request) valida expected_heads y actualiza revisión+heads en UNA transacción; archivos blob se escriben antes. Un blob huérfano tras rollback no es dato publicado y se limpia localmente cuando no referenciado ni staging activo.

## Composición

infra::composition::build_services(AppConfig, AppPaths, ShutdownToken) -> AppResult<ApplicationServices>:
1. Validar configuración pública/paths y crear dirs privados.
2. Abrir storage + migrations + recuperar journals antes de aceptar Commands.
3. Crear registry tools, process runner, platform detector y agentes.
4. Crear KeyringSecretStore y CryptoProvider; keyring ausente produce capability Blocked, no plaintext fallback.
5. Crear clientes reqwest TLS+timeouts. Drive sin client config queda disconnected; GitHub vía gh aprobado sin almacenar token propio v1.
6. Construir servicios library/policy/install/sync/guidance con Arc ports.
7. Desktop inicia runtime Tokio en thread independiente y su bridge; CLI runtime Tokio normal.

No volver a construir servicios por render ni por ventana. Shutdown cancela consultas seguras; operaciones mutantes concluyen punto transaccional o journal recuperable; join deadline y recovery siguiente inicio.

## Wiring GUI y async

CommandEnvelope { request_id, route_generation, skill_id?, expected_revision?, command }.
- Views mantienen Entity<InputState>, Subscription y editor DraftState.
- Handler toma valores y envía Command; bridge valida bounded queue (64), coalesce búsquedas, conserva cancel token y JobId.
- Tokio ejecuta IO; spawn_blocking para SQLite/KDF/descompresión/procesos CPU. Semáforos: 4 checks, 2 filesystem, 1 mutación por destination/vault.
- UiEvent contiene resultado domain DTO, request_id/route_generation/revision.
- Reducer puro apply_event ignora búsquedas obsoletas, pero registra mutación completada en activity independientemente de ruta; no perder recibo si vista cambió.
- WeakEntity actualiza en thread GPUI y cx.notify(); suscripción retenida en view. No usar mutex bloqueante de runtime dentro render.
- Cancelación antes de commit aborta; después de commit devuelve resultado y refresca; no simular rollback inexistente.
- Toast es resumen; resultados/error tienen panel persistente y retry específico.

## Flujos

Crear/editar: editor -> validate_bundle -> save_revision(expected_heads) -> blob seguro -> transacción -> RevisionSaved -> lista actualizada -> sync pending.

Instalar: selected revision -> detect agent -> compute_install_plan -> mostrar files/capabilities/hash -> consentimiento concreto -> revalidate target+head+tool identity -> stage+journal -> rename o vendor command -> verify -> receipt -> InstallationCompleted. Registry policy eval aparte muestra enforcement real.

Checks: repo profile -> Git read-only capabilities -> typed checks -> ProcessPort approved driver -> CheckReport -> guidance.plan_for(report). Cambiar archivo no autoriza git commit/push.

Sync: unlocked vault+account -> list ALL pages -> download bounded unseen -> crypto.open+validate -> merge DAG local -> preserve conflicts -> snapshot local captured generation -> seal -> upload immutable -> persist remote id -> re-list heads. Si library cambió durante upload, volver pending, jamás afirmar fully synced prematuramente.

Restore: inspect/decrypt -> quarantine validate -> diff current vs backup -> export recovery current -> journal -> commit revisions and heads -> refresh UI. Installations/profile paths no se restauran automáticamente.

## Extensibilidad controlada

Agregar proveedor/agent implica implementar port, fixture, capability matrix, documentación y test contract. Un archivo skill no puede registrar binarios, endpoints ni hooks ejecutables. Cada adapter declara Unsupported para capacidades que no realiza. Cambios schema/crypto requieren migrator separado; nunca upgrade silencioso que destruya export vieja.
