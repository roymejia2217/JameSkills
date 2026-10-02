# Reanudación JameSkills

Fecha UTC: 2026-10-02
Rama / commit: `feat/bootstrap-toolchain` / `9ea9749`, `7bf2bd3`, `f4eb1b3`
Última tarea / checkpoint completo: T002. C001 pendiente hasta T003.
Tarea activa y estado: iniciar T003.a/T003.b, desktop y CLI.
Prueba roja y resultado: `cargo metadata --format-version 1 --no-deps` falló sin `Cargo.toml`; toolchain Rust/Cargo no estaba instalada.
Último comando verde y resultado: metadata estructural con core sin deps e infra -> core; `cargo check -p jameskills-core -p jameskills-infra`; `cargo fmt --all -- --check`.
Archivos modificados: workspace y crates core/infra; evidencia toolchain/plataforma.
Contratos modificados y documento: ninguno. T001 mantiene el pin de Kit 0.7.0 y baseline Rust 1.92.0.
Bloqueos con fuente/evidencia saneada: Linux es contenedor sin display ni `/dev/dri`; no hay host Windows. Evidencia y requisitos publicados separados en `docs/PLATFORM-EVIDENCE.md`. No declarar smoke nativo hasta runner real.
Próximas tareas elegibles: T003.a/T003.b (T002 completada).
Próxima acción exacta: comprobar que aún no existe CLI ejecutable ni separación GUI; crear los dos targets, con GPUI solo en desktop, y generar Cargo.lock.
Lecturas mínimas: `AGENTS.md`, sección T003 de `tasks/todo.md`, `docs/ARCHITECTURE.md`, `docs/CONTRACTS.md`, fuente de GPUI Kit v0.7.0.
Evidencia manual Linux / Windows pendiente: ventana GPUI real en Linux con display/GPU; build y recorrido GPUI real en Windows/MSVC.
