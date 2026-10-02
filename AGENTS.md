# JameSkills — instrucciones para agentes de implementación

La instrucción actual del usuario manda sobre este archivo. El estado inicial del repositorio es un dossier de planificación, no una app terminada. Implementar cuando el usuario encargue ejecutar el plan; para solicitudes documentales editar el dossier.

## Fuente de verdad y contexto

Leer README.md y tasks/HANDOFF-LUNA.md. tasks/todo.md es checklist/DAG, docs/CONTRACTS.md fija firmas y flags, docs/SPEC-<module>.md contratos, docs/ARCHITECTURE.md wiring, docs/SECURITY.md controles. Leer spec/archivos de tarea actual, no todo el dossier cada turno. Consultar tasks/RESUME.md si existe y verificar estado real antes reanudar.

El usuario encargó el plan completo sin aprobaciones documentales entre fases. No inventar gates de fase nuevos. Permisos/credenciales/GPU/firma que dependan del usuario se guían con pasos verificables y solo bloquean sus dependientes. La autorización de producir documentos no publica repos/releases ni ejecuta cambios externos.

## Ejecución

- Primera pendiente con dependencias completas según orden topológico; IDs no orden numérico.
- RED significativo -> mínimo GREEN -> REFACTOR -> focused tests -> evidencia/commit. No syntax error como RED de dominio, no cero tests como GREEN.
- <=5 archivos por incremento; subtareas incluyen wiring/Cargo/module/factory/fixtures. Si falta presupuesto subdividir explícito.
- Contratos finales se incorporan progresivamente con proveedores reales; no success stubs ni módulos declarados inexistentes.
- Mantener core sin GPUI/HTTP/SQLite; desktop/CLI comparten services, no IO en render.
- Rust1.95.0 baseline por GPUI `cold_path`, Kit=0.7.0/source tag; lock real y MSRV comprobado en `docs/PLATFORM-EVIDENCE.md`. No cambiar GUI nativa por web.
- Funcionalidad incompatible muestra Unsupported/Unknown/Blocked; no etiquetar Pass por un tick o documento.
- Cada2–3 tareas checkpoint y RESUME con branch/commit/tests/nextaction/blocked evidence saneada.
- No marcar tarea hasta AC/evidence; preservar cambios ajenos y source types/tests previos.
- Conventional Commits en rama; no push main remoto automático ni bajar gates para verde.

## Invariantes de seguridad

Import/preview no ejecutan código. Portable paths/archives safe. Tool argv/approvedbinary no shellinput; tokens keyring, no log/envdump/passphraseargv. expected-head y journals antes mutar. authority exigida distinta observada. Drive snapshots inmutables causal DAG, no overwrite singleton. Crypto bytes/AAD de SPEC-cloud-sync y llaves sesión encapsuladas. Restore auth+validate+diff+recovery antes commit. No borrar archivos ajenos/edits ni branch concurrente.

## Verificación y entrega

Comandos posteriores a bootstrap en TESTING/OPERATIONS; UI tests feature test-support explícita. Native runtime OS/Google/installedagents no sustituibles por mocks; registrar diferencias. Nueva API externa requiere fuente/version fixture. La v1 íntegra cumple R01–R12, CLI+GUI+cipher sync+conflicts/restore+packages Windows/Linux, sin botones fake ni TODO críticos. Publicación externa solo con autorización correspondiente.
