# Handoff para GPT6 Luna: ejecutar JameSkills v1

Este repositorio contiene un plan, no una aplicación implementada. Todas las tareas iniciales de `tasks/todo.md` están pendientes. El usuario pidió a GPT-6.1 Sol redactar el plan completo para que puedas ejecutarlo de principio a fin. Sigue el alcance completo; una demo de biblioteca sin cloud/restore/paquetes no satisface v1.

## Inicio de la ejecución

1. Lee las instrucciones actuales del usuario, `AGENTS.md` si existe, `CAPABILITY-MAP.md`, `tasks/plan.md` y el índice de `tasks/todo.md`.
2. Lee `docs/CONTRACTS.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY.md`, `docs/SOURCES.md` y la spec de la tarea seleccionada. No implementes basándote únicamente en este resumen.
3. Si existe `tasks/RESUME.md`, comprueba su rama/commit/evidencia contra `git status --short` y el repo real. No repitas tareas completadas ni sobrescribas cambios ajenos.
4. Elige la primera tarea pendiente con todas sus dependencias completadas; inicialmente T001. Si falta una credencial/plataforma, registra el bloqueo y continúa una tarea independiente cuyo DAG lo permita.
5. Escribe la prueba significativa primero, registra rojo por comportamiento ausente, implementa, prueba, revisa y registra verde. Las tareas iniciales de toolchain/config usan comprobaciones de sus requisitos. Nunca presentes un error de instalación como una prueba roja del dominio.
6. Mantén <=5 archivos por tarea incluyendo wiring, tests y manifiestos. Si hacen falta más, introduce Txxx.a/Txxx.b y deps en checklist; cierra el ID padre cuando ambas estén verificadas.
7. Marca `[x]` solo tras satisfacer aceptación y verificación. Commits convencionales en rama. Cada tres tareas cierra el checkpoint y actualiza `tasks/RESUME.md`.

No hay aprobación humana obligatoria entre fases del plan: el encargo documental autorizó el plan completo. El inicio de implementación debe venir de la instrucción actual del usuario. Las credenciales, permisos de sistema, OAuth, GPUs, signing y publicación real se resuelven con la guía y autorización que aplique a esa operación; no los sustituyas por valores ficticios.

## Contratos que debes conservar

- Rust nativo, GPUI y **GPUI Kit** (`gpui-kit = "=0.7.0"` oficial; 0.6.5 está yanked); toolchain Rust 1.95.0 baseline y GPUI snapshot 0.3.7 vía reexports. La página del Kit cita 1.92, pero el source real usa `cold_path` estable en 1.95; ver `docs/PLATFORM-EVIDENCE.md`. No cambiar el proyecto a web ni a otro kit por una compilación difícil.
- Crates `jameskills-core`, `jameskills-infra`, `jameskills-desktop`, `jameskills-cli`. Casos de uso y puertos compartidos; UI/CLI son consumidores. CLI no inicializa GPU.
- `SKILL.md` portable + `jameskills.toml` v1 + políticas TOML tipadas. Importar no ejecuta contenido. Herramientas registradas usan argv y executable absoluto aprobado.
- Los cinco agentes usan fuentes y versiones comprobadas. Antigravity usa `agy plugin` y plugin.json; Grok usa `grok version`/`grok inspect --json` y `.grok/skills`/GROK_HOME. Cada capacidad particular no demostrada permanece bloqueada; no confundir Antigravity CLI con rutas del IDE.
- Checks muestran evidencia, desconocido/bloqueado y alcance. Hooks locales son eludibles. Protección de main/PR/CI exige privilegios y evidencia remota; textos no garantizan enforcement.
- Snapshots Drive completos e inmutables; appDataFolder/drive.appdata; DAG causal, unión, conflictos y tombstones; sin HEAD/CAS imaginarios ni GC remota automática v1.
- Cifrado según bytes exactos de SPEC-cloud-sync (referenciados desde SECURITY): XChaCha20Poly1305, llave maestra random, wrapping Argon2id fijo, header autenticado, límites antes de KDF. UnlockedVault conserva master+wrappingkey+salt+vaultID; passphrase zeroized tras KDF, cache opt-in solo ese material versionado. Keyring para tokens; no plaintext fallback; keyring ausente bloquea cloud persistente, permite biblioteca offline/export cifrado manual.
- Restore/import/install requieren preview, validación, staging y transacción con rollback. Ningún job antiguo escribe sobre una revisión más reciente.

## Contexto mínimo de una sesión

Mantén a mano: ID actual, criterios de aceptación, prueba roja, archivos <=5, firmas de contrato, deps y próximo checkpoint. Abre solo la spec pertinente y APIs compartidas necesarias. Al descubrir un cambio de contrato actualiza spec/arquitectura/tarea antes de ampliar el código.

Plantilla para el primer `tasks/RESUME.md` y posteriores:

```markdown
# Reanudación JameSkills
Fecha UTC:
Rama / commit:
Última tarea / checkpoint completo:
Tarea activa y estado:
Prueba roja y resultado:
Último comando verde y resultado:
Archivos modificados:
Contratos modificados y documento:
Bloqueos con fuente/evidencia saneada:
Próximas tareas elegibles:
Próxima acción exacta:
Lecturas mínimas:
Evidencia manual Linux / Windows pendiente:
```

## Condiciones para finalizar

Completa todos los criterios de `tasks/plan.md` y specs, incluyendo GUI nativa, CLI CI, cinco adaptadores documentados con capacidades incompatibles explícitas y probadas, cloud cifrado, conflictos, recovery/restore y paquetes Windows/Linux. Distingue “tests con fake pasan”, “integración real comprobada”, “artifact empaquetado” y “release publicado”. Informa los bloqueos reales con instrucciones verificables y conserva sus tareas pendientes. No declares una v1 completa si una función obligatoria quedó como TODO, botón de muestra o stub de éxito.
