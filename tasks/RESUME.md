# Reanudación JameSkills

Fecha UTC: 2026-10-04
Rama / commits: `feat/t016-repository-facts` sobre `main` `175aa50`; T016.dep `f0151d7`, T016.dep.e `98d914c`; T016.a local.
PR #19 T015 merged, CI 9/9. PR de T016 pendiente.

## Tarea activa

T016.a está implementada localmente: ProcessSpec/ApprovedExecutable/ApprovedRoot/ApprovedEnv/ProcessOutput, CancellationToken y ProcessPort async; contracts focused 3/3. Activa T016.b: runner process-group con límites/cancel.

## Evidencia T016.dep

- RED: `cargo check -p jameskills-core -p jameskills-infra --locked` pidió actualizar lock.
- GREEN: sin `--locked` resolvió `command-group 5.0.1` + `nix 0.27.1`; `cargo check -p jameskills-core -p jameskills-infra --locked` 0 en Windows MSVC.
- `async-trait 0.1.92`, Tokio 1.53.1 y command-group 5.0.1 tienen fuentes, versiones, yanked/license/MSRV registrados en `docs/SOURCES.md`.
- command-group documenta group_spawn con Unix process group y Windows Job Object; no usar wait_with_output por su lectura secuencial de stdout/stderr en Windows.
- RED T016.a: `process_contract` test no compila porque `ports::process` y DTOs faltaban.
- GREEN T016.a: process_contract 3/3; core 62/62 Windows; core Clippy/fmt/diff clean.

## T005 blocker y próxima elegibilidad

T005 requiere smoke de ventana visible/captura y display/GPU observados; sigue sin demostrar en Windows, y Linux carece sesión gráfica/GPU y development libs `xcb`, `xkbcommon`, `xkbcommon-x11`. Evidencia: `docs/PLATFORM-EVIDENCE.md`. T008 depende de T005. T016 es la siguiente tarea independiente elegible; no presentar build/test-support como smoke nativo.

## Próxima acción exacta

T016.b: implementar runner en thread blocking con process group/Job Object, drenar ambos outputs en paralelo, imponer límite conjunto y matar grupo ante timeout/cancel. Añadir test helper spawn cross-platform que no invoque shell.

## Lecturas mínimas

`tasks/todo.md` T016.a–c; `docs/CONTRACTS.md` process types; `docs/SECURITY.md` process; `docs/SOURCES.md` T016 process runtime; `docs/ARCHITECTURE.md` port/lifecycle.

Preservar `target/` y `JameSkills-implementation-dossier.zip` sin seguimiento.
