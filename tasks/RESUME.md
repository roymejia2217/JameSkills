# Reanudación JameSkills

Fecha UTC: 2026-10-04
Rama / commit base: `feat/t016-repository-facts` sobre `main` `175aa50`; T016.dep `f0151d7`.
PR #19 T015 merged, CI 9/9. T016.dep/e completa localmente; PR de T016 pendiente.

## Tarea activa

T016.a — definir ProcessSpec/ApprovedExecutable/ApprovedRoot/ApprovedEnv/ProcessOutput, CancellationToken y ProcessPort async. Después implementar runner de proceso cross-platform con `command-group` y collect_repository_facts usando argv Git fijo.

## Evidencia T016.dep

- RED: `cargo check -p jameskills-core -p jameskills-infra --locked` pidió actualizar lock.
- GREEN: sin `--locked` resolvió `command-group 5.0.1` + `nix 0.27.1`; `cargo check -p jameskills-core -p jameskills-infra --locked` 0 en Windows MSVC.
- `async-trait 0.1.92`, Tokio 1.53.1 y command-group 5.0.1 tienen fuentes, versiones, yanked/license/MSRV registrados en `docs/SOURCES.md`.
- command-group documenta group_spawn con Unix process group y Windows Job Object; no usar wait_with_output por su lectura secuencial de stdout/stderr en Windows.

## T005 blocker y próxima elegibilidad

T005 requiere smoke de ventana visible/captura y display/GPU observados; sigue sin demostrar en Windows, y Linux carece sesión gráfica/GPU y development libs `xcb`, `xkbcommon`, `xkbcommon-x11`. Evidencia: `docs/PLATFORM-EVIDENCE.md`. T008 depende de T005. T016 es la siguiente tarea independiente elegible; no presentar build/test-support como smoke nativo.

## Próxima acción exacta

Agregar primero contratos y test RED de ProcessPort; luego tests cross-platform que verifican argv separado, caps, timeout/cancel de grupo, stderr/stdout simultáneos y repo normal/detached/worktree/submodule. No ejecutar hooks/fetch/push ni heredar env secrets.

## Lecturas mínimas

`tasks/todo.md` T016.a–c; `docs/CONTRACTS.md` process types; `docs/SECURITY.md` process; `docs/SOURCES.md` T016 process runtime; `docs/ARCHITECTURE.md` port/lifecycle.

Preservar `target/` y `JameSkills-implementation-dossier.zip` sin seguimiento.
