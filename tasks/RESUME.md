# Reanudación JameSkills

Fecha UTC: 2026-10-02
Rama / commit: `feat/bootstrap-toolchain` / `521ea0e` (T010.b), `17cfaeb` (T010.dep)
Última tarea / checkpoint completo: T010; T007 shared runtime, T009 CLI contractual y T010 formato de skill cerrados. C003 permanece pendiente porque shell T008 espera desktop T005/native gates.
Tarea activa y estado: T011, schema de políticas tipadas y requisitos de herramientas.
Prueba roja y resultado: T010 amplió bundle_manifest con rechazo de keys duplicadas/unknown y conservar límites de contenido; todos los 13 contratos enfocados pasan tras el parser Serde restringido. La primera ejecución intentada durante el cambio dependiente falló al compilar por el import viejo de yaml-rust2; no se contabilizó como RED funcional.
Último comando verde y resultado: `cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked --offline` 42/42; Clippy combinado all-targets `-D warnings`; `cargo fmt --all -- --check`; `git diff --check`.
Archivos modificados: ninguno pendiente; zip del dossier y `target/` ajenos al índice y preservados.
Contratos modificados y documento: `serde-saphyr` con presupuesto YAML explícito, unknown fields/duplicate keys strict y diagnósticos estáticos saneados; DTO públicos SkillManifest/Frontmatter desde parsers validados. Contratos en `docs/CONTRACTS.md`, límites/formato en `docs/SPEC-skill-format.md`.
Bloqueos con fuente/evidencia saneada: Debian13 headless carece development libs `xcb`, `xkbcommon`, `xkbcommon-x11`; sin display/GPU. No hay Windows runner. No declarar smoke nativo.
Próximas tareas elegibles: T011. T012 espera T011 y T004 nativo; T005/T006/T008 siguen dependiendo de T004 native verification.
Próxima acción exacta: diseñar DTOs `Requirement`/`ToolRequirement` y registry cerrado de verificaciones/acciones; escribir primero pruebas que rechacen versiones, severities, tools y shell libre, además de enlaces fuera de suite.
Lecturas mínimas: T011 de `tasks/todo.md`, secciones de políticas de `docs/SPEC-skill-format.md`, `docs/CONTRACTS.md` y `docs/ARCHITECTURE.md`.
Evidencia manual Linux / Windows pendiente: ventana GPUI real en Linux con display/GPU; build y recorrido GPUI real en Windows/MSVC.
