# Reanudación JameSkills

Fecha UTC: 2026-10-04
Rama / commits: `feat/t016-repository-facts` sobre `main` `175aa50`; T016.dep `f0151d7`, T016.dep.e `98d914c`, T016.a `df8d327`, T016.b `441ffbc`, T016.c `677423d`, T016.d `9c672e9`.
PR #19 T015 merged, CI 9/9. PR de T016 pendiente.

## Tarea activa

T016.dep/a/b/c/d completas localmente. El código y facts están listos; crear PR #20 y esperar CI remoto antes de cerrar el parent.

## Evidencia T016.dep

- RED: `cargo check -p jameskills-core -p jameskills-infra --locked` pidió actualizar lock.
- GREEN: sin `--locked` resolvió `command-group 5.0.1` + `nix 0.27.1`; `cargo check -p jameskills-core -p jameskills-infra --locked` 0 en Windows MSVC.
- `async-trait 0.1.92`, Tokio 1.53.1 y command-group 5.0.1 tienen fuentes, versiones, yanked/license/MSRV registrados en `docs/SOURCES.md`.
- command-group documenta group_spawn con Unix process group y Windows Job Object; no usar wait_with_output por su lectura secuencial de stdout/stderr en Windows.
- RED T016.a: `process_contract` test no compila porque `ports::process` y DTOs faltaban.
- GREEN T016.a: process_contract 3/3; core 62/62 Windows; core Clippy/fmt/diff clean.
- RED T016.b: `process_execution` no compila porque `SystemProcessPort` no existía.
- GREEN T016.b: `process_execution` 3/3 Windows (streams simultáneos, límite, timeout/cancel); infra 64/64, core 62/62, workspace Clippy/fmt/diff clean.
- RED T016.c: `repository_facts` no compila porque faltan `RepositoryState` y `collect_repository_facts`.
- GREEN T016.c: repository_facts 3/3 Windows; fake comprueba args/cwd con metacaracteres y real Git temporal cubre attached/detached/worktree/submodule. Core 62/62, infra 67/67; workspace Clippy/fmt/diff clean.
- GREEN T016.d local: lock/runtime deps fijados, test execution/facts y suites acumuladas verificadas; C005 permanece abierto por el smoke de ventana T005.

## T005 blocker y próxima elegibilidad

T005 requiere smoke de ventana visible/captura y display/GPU observados; sigue sin demostrar en Windows, y Linux carece sesión gráfica/GPU y development libs `xcb`, `xkbcommon`, `xkbcommon-x11`. Evidencia: `docs/PLATFORM-EVIDENCE.md`. T008 depende de T005. T016 es la siguiente tarea independiente elegible; no presentar build/test-support como smoke nativo.

## Próxima acción exacta

Revisar diff/commits, push `feat/t016-repository-facts`, crear PR #20 y esperar CI Linux/Windows. No cerrar T016 hasta gates remotos verdes. Después reevaluar T005; si sigue bloqueada, comenzar T017, la siguiente tarea independiente.

## Lecturas mínimas

`tasks/todo.md` T016; `docs/CONTRACTS.md` process types; `docs/SECURITY.md` process; `docs/SOURCES.md` T016 process runtime; `docs/ARCHITECTURE.md` port/lifecycle.

Preservar `target/` y `JameSkills-implementation-dossier.zip` sin seguimiento.
