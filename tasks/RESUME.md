# Reanudación JameSkills

Fecha UTC: 2026-10-04
Rama / base: `feat/t017-tool-capabilities` sobre `main` `b53ed68`.
T015 (PR #19) y T016 (PR #20) merged con CI 9/9. C005 permanece sin marcar por el smoke nativo de T005.

## Tarea activa

T017.a está completa localmente; activa T017.b: registry y probes reales limitados a `profiles/tools.toml`. T005 requiere ventana visible/captura y observación display/GPU en Windows; Linux no tiene display/GPU ni development libs. T008 depende de T005.

## RED/GREEN

- RED: `cargo test -p jameskills-core --locked --test tool_capabilities` falla E0432 porque `domain::guidance` y ToolDetection no existen.
- GREEN T017.a: `tool_capabilities` 3/3; core 65/65 Windows; core Clippy/fmt/diff clean. Availability, compatibility y operation support separan Missing/Unknown/Blocked/Incompatible; Candidate compatible sigue NeedsVerification; evidencia sólo tiene source ID, observed_at, tested_version y summary app-authored.
- T017.b ejecutará solo probes declarados en `profiles/tools.toml`, con ProcessPort/ApprovedExecutable y argv registrados.

## Próxima acción

T017.b: construir perfiles/probes de registry y conectarlos a ProcessPort; ignorar tool claims de skill names y validar resultados con versiones/provenance antes de Verified/Supported.

## Lecturas mínimas

`tasks/todo.md` T017; `docs/CONTRACTS.md` ToolId/Evidence/CheckStatus; `docs/SPEC-policy-engine.md` tool registry, probes y status semantics; `docs/SECURITY.md` Process.

Preservar `target/` y `JameSkills-implementation-dossier.zip` locales sin seguimiento.
