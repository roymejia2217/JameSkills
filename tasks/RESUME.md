# Reanudación JameSkills

Fecha UTC: 2026-10-03
Rama / commits: `feat/t015a-canonical-example` sobre `main` `843521c`; T015.a1 `4e9251b`, T015.a2 `55fd2ec`, T015.b1 `6bc0a96`, T015.b2a `b9f3f29`, T015.b2b `f758217`, T015.b3 `707ad44`, T015.c `baed6a1`, T015.d1 `925acaf`, T015.d2 `3404322`, d2e `8a5ee49`; T015.d3 local.
T037/T038 (incluida T038.a) están cerradas en `main`; PR #18 se fusionó con CI Windows/Linux 9/9.

## Tarea activa

T015.a, T015.b (b1+b2+b3), T015.c, T015.d1, T015.d2 y d2e completas localmente. Activa T015.d4: cablear validate JSON/text en CLI con errors/diagnostics reales. T005 permanece pendiente por smoke visual nativo; T008 depende de T005.

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
- RED T015.c: `runtime_ci_template_is_fail_closed_and_svg_asset_has_no_active_content` falla porque falta `templates/ci-rust.yml` runtime.
- GREEN T015.c: 1/1 focused; `bundle_manifest` 19/19, core 53/53, core Clippy/fmt/diff clean. SVG es propio/static y el CI template conserva placeholders `exit 1` hasta resolver SHA/toolchain.
- RED T015.d baseline: `cargo test -p jameskills-cli --locked --test validate_bundle` compila y falla porque `validate` aún responde Unsupported (exit 3) al ejemplo oficial.
- RED T015.d1: import de test falló E0432 porque `validate_bundle` no existía.
- GREEN T015.d1: core 56/56; bundle oficial entrega hash y summary, falta policy resource y acción guidance libre se rechazan; core Clippy/fmt/diff clean.
- GREEN T015.d2: service tests 2/2; core 58/58; core Clippy/fmt/diff clean. El service usa provider inyectable y propaga diagnostics sin SQLite.
- RED T015.d3: `RuntimeServices::library` no existía.
- GREEN T015.d3: infra 61/61 Windows; factory compone LocalFileSystem/LibraryService y valida runtime fixture sin crear config/data/cache; workspace Clippy/fmt/diff green.

## Próximos pasos

1. T015.d4 hace que main componga servicios para validate y que dispatch use LibraryService; respuesta JSON/texto incluye summary/diagnostics sin echo de entrada.
2. Ejecutar test CLI oficial + bundle inválido, CLI/core/infra tests y los checks del workspace.
3. Ejecutar T015 CLI/core/infra tests, actualizar checklist/evidencia, commit y PR verde; T005 continúa bloqueado con evidencia documentada.

## Lecturas y contratos

`tasks/todo.md` T015; `docs/CONTRACTS.md`; `docs/SPEC-skill-format.md`; `docs/SPEC-policy-engine.md`; fuentes `docs/examples/repository-foundation/*`.

T005: `docs/PLATFORM-EVIDENCE.md` documenta Windows build sin captura/display/GPU observados y Linux contenedor sin sesión gráfica/GPU, además de libs `xcb`, `xkbcommon`, `xkbcommon-x11` ausentes. No afirmar smoke ni Pass por build. Preservar los artefactos locales sin seguimiento `target/` y `JameSkills-implementation-dossier.zip`.
