# Reanudación JameSkills

Fecha UTC: 2026-10-02
Rama / commit: `feat/bootstrap-toolchain` / `6634afa` (cierre T011), `f32f55c` (T011.schema), `2c9d60c` (T011.contract)
Última tarea / checkpoint completo: T011; T007 shared runtime, T009 CLI contractual, T010 formato de skill y T011 schema tipado completos. C003 sigue pendiente porque shell T008 espera desktop T005/native gates.
Tarea activa y estado: T012.dep/contract, descomponer validación pura de inventario de la capa OS. T012 parent y T012.b siguen bloqueados por T004.
Prueba roja y resultado: T011 `cargo test -p jameskills-core --test policy_schema --locked --offline` falló por el parser temporal `policy.unsupported`; GREEN focal 5/5. Rechaza schema/severity/tool/operation/command desconocidos, refs faltantes/ciclos; fixture cubre 10 requisitos.
Último comando verde y resultado: `cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked --offline` 47/47; Clippy combinado all-targets `-D warnings`; `cargo fmt --all -- --check`; `git diff --check`.
Archivos modificados: ninguno pendiente; zip del dossier y `target/` ajenos al índice y preservados.
Contratos modificados y documento: `serde-saphyr` con presupuesto YAML explícito; policy TOML cerrado con DTOs tipados, registry de ToolId/ToolOperation, Scope global y dependencia DAG. Contratos en `docs/CONTRACTS.md`, límites y kinds en `docs/SPEC-policy-engine.md`.
Bloqueos con fuente/evidencia saneada: Debian13 headless carece development libs `xcb`, `xkbcommon`, `xkbcommon-x11`; sin display/GPU. No hay Windows runner. No declarar smoke nativo.
Próximas tareas elegibles: `T012.dep` y `T012.contract` son documentales/dep independientes; `T012.a` depende de T010/T011. T012 parent y T012.b esperan T004. T004 tiene completadas las subtareas a/b/c; T004.c registra que PowerShell solo se analizó desde Linux, no se probó en Windows. T005/T006/T008 también esperan T004.
Próxima acción exacta: fijar `icu_casemap` (unicode-casefold 0.2.0 usa datos Unicode9.0), lockearlo/documentarlo, luego implementar en core el inventario bounded con full case-fold + NFC, sin abrir filesystem ni ZIP.
Lecturas mínimas: T004/T012 de `tasks/todo.md`, contratos FileSystem/Bundle en `docs/CONTRACTS.md`, paths/archives en `docs/SPEC-skill-format.md`, pruebas portables en `docs/SECURITY.md` y wiring `docs/ARCHITECTURE.md`.
Evidencia manual Linux / Windows pendiente: ventana GPUI real en Linux con display/GPU; build y recorrido GPUI real en Windows/MSVC.
