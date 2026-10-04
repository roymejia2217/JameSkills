# Reanudación JameSkills

Fecha UTC: 2026-10-03
Rama / commit: `feat/t015a-canonical-example`, T015.a1 `4e9251b` sobre `main` `843521c`; T015.a2 está en el working tree.
T037/T038 (incluida T038.a) están cerradas en `main`; PR #18 se fusionó con CI Windows/Linux 9/9.

## Tarea activa

T015.a (a1+a2) completa localmente: fuentes parsables y tres archivos runtime validados. Siguiente subtarea T015.b: guidance, referencias y templates de lectura. T005 permanece pendiente por smoke visual nativo; T008 depende de T005. T015 fue la primera tarea independiente elegible en el orden topológico.

## T015.a1 RED/GREEN

- RED fuente: test del ejemplo del dossier detectó `frontmatter.unknown_field` por `license` fuera del DTO estándar; tras corregirlo, detectó `policy.invalid` por campos/checks no compatibles con T010/T011.
- RED destino: `cargo test -p jameskills-core --locked --test bundle_manifest portable_repository_example_is_available_at_the_runtime_fixture_path` falla porque falta `examples/repository-foundation/jameskills.toml`.
- GREEN T015.a1: `official_repository_example_passes_manifest_skill_and_policy_parsers` 1/1; `policy_schema` 5/5. Fuente `docs/examples/repository-foundation` pasa manifest/frontmatter/policy.
- GREEN T015.a2: `portable_repository_example_is_available_at_the_runtime_fixture_path` 1/1; `bundle_manifest` 15/15; core 49/49 Windows; core Clippy, workspace fmt y diff check pasan.

## Próximos pasos

1. Revisar parsers/contratos de guidance en T015.b frente a `docs/examples/repository-foundation/guidance/repository.toml` y validar todas sus referencias con la política runtime ya copiada.
2. Completar T015.b en su presupuesto de archivos, probar el DAG tipado y los template references, registrar evidencia.
3. Commit convencional y PR verde a `main`; el smoke visual T005 continúa bloqueado con evidencia documentada.

## Lecturas y contratos

`tasks/todo.md` T015; `docs/CONTRACTS.md`; `docs/SPEC-skill-format.md`; `docs/SPEC-policy-engine.md`; fuentes `docs/examples/repository-foundation/*`.

T005: `docs/PLATFORM-EVIDENCE.md` documenta Windows build sin captura/display/GPU observados y Linux contenedor sin sesión gráfica/GPU, además de libs `xcb`, `xkbcommon`, `xkbcommon-x11` ausentes. No afirmar smoke ni Pass por build. Preservar los artefactos locales sin seguimiento `target/` y `JameSkills-implementation-dossier.zip`.
