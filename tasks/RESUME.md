# Reanudación JameSkills

Fecha UTC: 2026-10-02
Rama / commit: `feat/bootstrap-toolchain` / `f32f55c` (T011.schema), `2c9d60c` (T011.contract), `af8ec2a` (cierre T010)
Última tarea / checkpoint completo: T011; T007 shared runtime, T009 CLI contractual, T010 formato de skill y T011 schema tipado completos. C003 sigue pendiente porque shell T008 espera desktop T005/native gates.
Tarea activa y estado: T012, validar árboles/importaciones; pendiente por dependencia T004 sin cierre nativo Windows.
Prueba roja y resultado: T011 `cargo test -p jameskills-core --test policy_schema --locked --offline` falló por el parser temporal `policy.unsupported`; GREEN focal 5/5. Rechaza schema/severity/tool/operation/command desconocidos, refs faltantes/ciclos; fixture cubre 10 requisitos.
Último comando verde y resultado: `cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked --offline` 47/47; Clippy combinado all-targets `-D warnings`; `cargo fmt --all -- --check`; `git diff --check`.
Archivos modificados: ninguno pendiente; zip del dossier y `target/` ajenos al índice y preservados.
Contratos modificados y documento: `serde-saphyr` con presupuesto YAML explícito; policy TOML cerrado con DTOs tipados, registry de ToolId/ToolOperation, Scope global y dependencia DAG. Contratos en `docs/CONTRACTS.md`, límites y kinds en `docs/SPEC-policy-engine.md`.
Bloqueos con fuente/evidencia saneada: Debian13 headless carece development libs `xcb`, `xkbcommon`, `xkbcommon-x11`; sin display/GPU. No hay Windows runner. No declarar smoke nativo.
Próximas tareas elegibles: ninguna pendiente con deps completas: T012 espera T004. T004 tiene completadas las subtareas a/b/c; T004.c registra que scripts PowerShell solo fueron parseados/consultados desde Linux, no probados en Windows. T005/T006/T008 también esperan T004.
Próxima acción exacta: dividir T012 en un slice `T012.a` de validación pura de inventario/rutas portables (T010+T011) y `T012.b` de filesystem/symlink/reparse en OS reales (T012.a+T004), para avanzar sin presentar pruebas Linux como evidencia Windows.
Lecturas mínimas: T004/T012 de `tasks/todo.md`, contratos FileSystem/Bundle en `docs/CONTRACTS.md`, paths/archives en `docs/SPEC-skill-format.md`, pruebas portables en `docs/SECURITY.md` y wiring `docs/ARCHITECTURE.md`.
Evidencia manual Linux / Windows pendiente: ventana GPUI real en Linux con display/GPU; build y recorrido GPUI real en Windows/MSVC.
