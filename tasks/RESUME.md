# Reanudación JameSkills

Fecha UTC: 2026-10-03
Rama / commit: `feat/t038-blob-recovery` / `8389058`; PR #18 contra `main`.
Última tarea completa en `main`: T037, T038 base (PR #17). T038.a implementada y localmente verificada; PR #18 tuvo CI Windows/Linux 9/9 verde. Se agregó evidencia/checklist al PR #18; esa actualización documental debe pasar sus checks antes de merge.

## Tarea activa

Cerrar y squash-mergear PR #18 tras los checks de la actualización documental. Luego comenzar T015.a, primer incremento independiente elegible según el orden del DAG.

## Evidencia T038.a

- RED: `cargo test -p jameskills-infra --locked --test revision_storage` falló con E0599 por ausencia de `orphan_blob_hashes`.
- GREEN: `revision_storage` 14/14 Windows; `cargo test -p jameskills-infra --locked` 60/60.
- `cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked`, `cargo check -p jameskills-desktop --locked`, workspace Clippy `-D warnings`, fmt y diff check pasaron en el pre-push hook.
- PR #18: build Linux/Windows, tests, clippy, fmt, commitlint, PR Governance, README Policy y Required CI pasaron (9/9).
- Blobs nombrados por hash se verifican antes de escritura/commit y al abrir SQLite; los huérfanos se enumeran y retienen, el rollback conserva la head anterior y permite reintento. Revisiones con blob referenciado ausente/corrupto bloquean apertura.

## Próximas tareas elegibles y bloqueos

- T005 sigue pendiente: Windows compila, pero falta smoke de ventana visible/captura y display/GPU observados; Linux no tiene sesión gráfica/GPU ni libs de desarrollo `xcb`, `xkbcommon`, `xkbcommon-x11`. Evidencia detallada en `docs/PLATFORM-EVIDENCE.md`; T008 depende de T005.
- T010/T011/T012/T013/T014 ya tienen acceptance y evidencia marcada; corregidos sus campos de estado en `tasks/todo.md`.
- T015 (T015.a primero) tiene dependencias satisfechas: T009, T011, T014, T037. El fixture fuente ya existe bajo `docs/examples/repository-foundation`; no copiarlo hasta comparar sus contratos.
- T039 depende de T037 y T038.a, pero aparece después en el orden del DAG y no precede T015.

## Próxima acción exacta

Después de merge de PR #18: actualizar desde `main`, leer T015 y `docs/examples/repository-foundation`, ejecutar validación/red de T015.a y completar solo sus tres archivos con evidencia.

## Lecturas mínimas

`tasks/todo.md` T015.a; `docs/CONTRACTS.md`; `docs/SPEC-skill-format.md`; `docs/SPEC-policy-engine.md`; `docs/examples/repository-foundation/*`.

No hay credenciales o secretos requeridos para T015.a. Preservar los artefactos locales no seguidos `target/` y `JameSkills-implementation-dossier.zip`.
