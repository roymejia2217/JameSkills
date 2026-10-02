# Reanudación JameSkills

Fecha UTC: 2026-10-02
Rama / commit: `feat/bootstrap-toolchain` / `56a28e3` (T012.a), `48d8ebf` (T012.dep), `ed801a9` (T012.contract)
Última tarea / checkpoint completo: T012.a, inventario portable puro; T011 cerrado y T012 parent abierto hasta filesystem real Linux/Windows. T007/T009/T010/T011 también completos. C003 sigue pendiente porque shell T008 espera desktop T005/native gates.
Tarea activa y estado: T012.b, inspección/staging filesystem y ZIP; aún no implementada. T004 no puede cerrarse en este entorno porque falta host/runner Windows/MSVC.
Prueba roja y resultado: T012.a `cargo test -p jameskills-core --test portable_bundle_inventory --locked --offline` falló en aceptación inicial `bundle.validation.unavailable`; otro RED identificó colisión en prefijos de directorio `Docs/...`/`docs/...`; GREEN focused 5/5. Cubre Unicode `Straße`/`STRASSE`, duplicate, special file types, SKILL/text/count/total size.
Último comando verde y resultado: `cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked --offline` 52/52; Clippy combinado all-targets `-D warnings`; `cargo fmt --all -- --check`; `git diff --check`.
Archivos modificados: ninguno pendiente; zip del dossier y `target/` ajenos al índice y preservados.
Contratos modificados y documento: `serde-saphyr` con presupuesto YAML explícito; policy TOML cerrado con DTOs tipados y registry; `BundleEntry` bounded, regular files only, Unicode full case-fold vía ICU 2.3 + NFC y árbol de rutas privado. Contratos en `docs/CONTRACTS.md`; límites en `docs/SPEC-skill-format.md`.
Bloqueos con fuente/evidencia saneada: Debian13 headless carece development libs `xcb`, `xkbcommon`, `xkbcommon-x11`; sin display/GPU. No hay Windows runner. No declarar smoke nativo.
Próximas tareas elegibles: T012.b Linux puede avanzar como slice nativo verificable tras T012.a. T012.b Windows y cierre de T012 esperan T004. T004 tiene completadas las subtareas a/b/c; PowerShell solo se analizó desde Linux, no se probó en Windows. T005/T006/T008 también esperan T004.
Próxima acción exacta: elegir estrategia no-follow con semántica real Linux/Windows; implementar inspección Linux y ZIP en staging bounded y mantener el smoke Windows pendiente hasta tener runner real.
Lecturas mínimas: T004/T012 de `tasks/todo.md`, contratos FileSystem/Bundle en `docs/CONTRACTS.md`, paths/archives en `docs/SPEC-skill-format.md`, pruebas portables en `docs/SECURITY.md` y wiring `docs/ARCHITECTURE.md`.
Evidencia manual Linux / Windows pendiente: ventana GPUI real en Linux con display/GPU; build y recorrido GPUI real en Windows/MSVC.
