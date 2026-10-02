# JameSkills — dossier de implementación

Plan completo de una aplicación de escritorio nativa Rust+GPUI Kit para Windows/Linux. Las skills son suites portables con instrucciones, políticas verificables, guía dinámica del entorno y assets. La biblioteca funciona offline, instala en cinco agentes de código y sincroniza backups cifrados mediante Google Drive.

Estado actual: especificaciones y plan. No hay aplicación implementada, Cargo.toml, repo protegido, Drive vinculado ni tests de app ejecutados.

## Ejecutar el plan

Las [instrucciones de agentes](AGENTS.md) fijan la ejecución reanudable. Comenzar por [handoff para GPT6 Luna](tasks/HANDOFF-LUNA.md), después [plan](tasks/plan.md) y [tareas](tasks/todo.md). Las tareas pequeñas tienen dependencias, archivos, funciones, pruebas RED/GREEN, aceptación y wiring. Registrar evidencia antes de marcarlas completas.

## Especificaciones

- [Requisitos y aceptación](docs/REQUIREMENTS.md)
- [Mapa de capacidades](CAPABILITY-MAP.md)
- [Arquitectura y estructura de archivos](docs/ARCHITECTURE.md)
- [Contratos, funciones y blueprints Rust/CLI](docs/CONTRACTS.md)
- [Formato común](docs/SPEC-skill-format.md)
- [Políticas y guía dinámica](docs/SPEC-policy-engine.md)
- [Agentes e instalación](docs/SPEC-agent-adapters.md)
- [Biblioteca y revisiones](docs/SPEC-skill-library.md)
- [Drive, cifrado, conflictos y restauración](docs/SPEC-cloud-sync.md)
- [Aplicación nativa](docs/SPEC-desktop-app.md)
- [Diseño de GUI y cada recorrido](docs/GUI.md)
- [Seguridad por frontera](docs/SECURITY.md)
- [TDD, XP y pruebas](docs/TESTING.md)
- [Puesta en marcha, Git, CI y distribución](docs/OPERATIONS.md)
- [Fuentes oficiales y discrepancias resueltas](docs/SOURCES.md)
- [Trazabilidad y auditoría documental](docs/TRACEABILITY.md)

## Ejemplo y límites

[Suite de referencia](docs/examples/repository-foundation/SKILL.md) es un fixture del esquema propuesto, para implementar y validar en TDD. Su plantilla CI tiene valores por resolver y debe fallar validación de producción hasta completarlos; no es workflow activo.

GPUI Kit0.7.0 fijado desde release/registry, OpenCode/Codex/Pi/AntigravityCLI/GrokBuild con contratos documentados. Unsupported/Unknown siempre visibles. Una skill puede guiar pasos del usuario, pero protección de main y gates CI dependen de reglas/permisos reales del proveedor. Tokens y passphrase no forman parte de la skill ni del backup.
