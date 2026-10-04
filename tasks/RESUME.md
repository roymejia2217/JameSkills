# Reanudación JameSkills

Fecha UTC: 2026-10-03
Rama / commits: `feat/t015a-canonical-example` sobre `main` `843521c`; T015.a1 `4e9251b`, T015.a2 `55fd2ec`, T015.b1 `6bc0a96`, T015.b2a `b9f3f29`; T015.b2b cambios locales.
T037/T038 (incluida T038.a) están cerradas en `main`; PR #18 se fusionó con CI Windows/Linux 9/9.

## Tarea activa

T015.a (a1+a2), T015.b1 y T015.b2 (b2a+b2b) completas localmente. Activa T015.b3: `.gitignore` template y enlace en SKILL.md. T005 permanece pendiente por smoke visual nativo; T008 depende de T005.

## T015.a1 RED/GREEN

- RED fuente: test del ejemplo del dossier detectó `frontmatter.unknown_field` por `license` fuera del DTO estándar; tras corregirlo, detectó `policy.invalid` por campos/checks no compatibles con T010/T011.
- RED destino: `cargo test -p jameskills-core --locked --test bundle_manifest portable_repository_example_is_available_at_the_runtime_fixture_path` falla porque falta `examples/repository-foundation/jameskills.toml`.
- GREEN T015.a1: `official_repository_example_passes_manifest_skill_and_policy_parsers` 1/1; `policy_schema` 5/5. Fuente `docs/examples/repository-foundation` pasa manifest/frontmatter/policy.
- GREEN T015.a2: `portable_repository_example_is_available_at_the_runtime_fixture_path` 1/1; `bundle_manifest` 15/15; core 49/49 Windows; core Clippy, workspace fmt y diff check pasan.
- RED T015.b1: test runtime falla porque todavía no existe `examples/repository-foundation/guidance/repository.toml`.
- GREEN T015.b1: guidance test 1/1; `bundle_manifest` 16/16, core 50/50, `policy_schema` 5/5; core Clippy/fmt/diff clean. Elimina plan `github-access-setup` porque el policy parser no soporta una requirement con ese ID; actions limitadas a enums y source IDs registrados.
- RED T015.b2a: `runtime_reference_material_matches_documented_sources` falla porque aún no existe `references/standards.md` bajo la ruta runtime.
- GREEN T015.b2b: `runtime_reference_material_matches_documented_sources` 1/1 al alinear referencias/README con fuentes del dossier.

## Próximos pasos

1. Copiar `gitignore.txt` como `templates/.gitignore`, sin cambiar contenido, y actualizar el enlace correspondiente en SKILL.md.
2. Añadir aserción de enlaces del SKILL a los recursos runtime; correr la suite core y verificar todos los links/TOML.
3. Commit convencional y PR verde a `main`; el smoke visual T005 continúa bloqueado con evidencia documentada.

## Lecturas y contratos

`tasks/todo.md` T015; `docs/CONTRACTS.md`; `docs/SPEC-skill-format.md`; `docs/SPEC-policy-engine.md`; fuentes `docs/examples/repository-foundation/*`.

T005: `docs/PLATFORM-EVIDENCE.md` documenta Windows build sin captura/display/GPU observados y Linux contenedor sin sesión gráfica/GPU, además de libs `xcb`, `xkbcommon`, `xkbcommon-x11` ausentes. No afirmar smoke ni Pass por build. Preservar los artefactos locales sin seguimiento `target/` y `JameSkills-implementation-dossier.zip`.
