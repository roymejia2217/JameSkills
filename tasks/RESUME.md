# Reanudación JameSkills

Fecha UTC: 2026-10-03
Rama / commits: `feat/t015a-canonical-example` sobre `main` `843521c`; T015.a1 `4e9251b`, T015.a2 `55fd2ec`, T015.b1 `6bc0a96`, T015.b2a `b9f3f29`, T015.b2b `f758217`; T015.b3 cambios locales.
T037/T038 (incluida T038.a) están cerradas en `main`; PR #18 se fusionó con CI Windows/Linux 9/9.

## Tarea activa

T015.a y T015.b (b1+b2+b3) completas localmente. Activa T015.c: template CI y asset SVG del ejemplo, inertes y seguros. T005 permanece pendiente por smoke visual nativo; T008 depende de T005.

## T015.a1 RED/GREEN

- RED fuente: test del ejemplo del dossier detectó `frontmatter.unknown_field` por `license` fuera del DTO estándar; tras corregirlo, detectó `policy.invalid` por campos/checks no compatibles con T010/T011.
- RED destino: `cargo test -p jameskills-core --locked --test bundle_manifest portable_repository_example_is_available_at_the_runtime_fixture_path` falla porque falta `examples/repository-foundation/jameskills.toml`.
- GREEN T015.a1: `official_repository_example_passes_manifest_skill_and_policy_parsers` 1/1; `policy_schema` 5/5. Fuente `docs/examples/repository-foundation` pasa manifest/frontmatter/policy.
- GREEN T015.a2: `portable_repository_example_is_available_at_the_runtime_fixture_path` 1/1; `bundle_manifest` 15/15; core 49/49 Windows; core Clippy, workspace fmt y diff check pasan.
- RED T015.b1: test runtime falla porque todavía no existe `examples/repository-foundation/guidance/repository.toml`.
- GREEN T015.b1: guidance test 1/1; `bundle_manifest` 16/16, core 50/50, `policy_schema` 5/5; core Clippy/fmt/diff clean. Elimina plan `github-access-setup` porque el policy parser no soporta una requirement con ese ID; actions limitadas a enums y source IDs registrados.
- RED T015.b2a: `runtime_reference_material_matches_documented_sources` falla porque aún no existe `references/standards.md` bajo la ruta runtime.
- GREEN T015.b2b: `runtime_reference_material_matches_documented_sources` 1/1 al alinear referencias/README con fuentes del dossier.
- RED T015.b3: test detectó link SKILL a `templates/gitignore.txt` que no existía en runtime ni coincidía con layout planeado.
- GREEN T015.b3: `runtime_skill_links_to_the_user_safe_gitignore_template` 1/1; core 52/52, Clippy/fmt/diff clean.

## Próximos pasos

1. Revisar T015.c, fuente template CI y SVG del dossier frente a seguridad/contratos; comprobar que el asset no contiene elementos activos o href externos.
2. Copiar los dos recursos runtime y actualizar evidencia; después implementar T015.d validate service/CLI con FileSystemPort real.
3. Commit convencional y PR verde a `main`; el smoke visual T005 continúa bloqueado con evidencia documentada.

## Lecturas y contratos

`tasks/todo.md` T015; `docs/CONTRACTS.md`; `docs/SPEC-skill-format.md`; `docs/SPEC-policy-engine.md`; fuentes `docs/examples/repository-foundation/*`.

T005: `docs/PLATFORM-EVIDENCE.md` documenta Windows build sin captura/display/GPU observados y Linux contenedor sin sesión gráfica/GPU, además de libs `xcb`, `xkbcommon`, `xkbcommon-x11` ausentes. No afirmar smoke ni Pass por build. Preservar los artefactos locales sin seguimiento `target/` y `JameSkills-implementation-dossier.zip`.
