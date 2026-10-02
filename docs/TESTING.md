# TDD, XP y estrategia de verificación

Pruebas futuras parte del plan; ninguna prueba de app ejecutada al redactarlo. El dossier se audita aparte: links/DAG/formatos/examples/cobertura.

## Ciclo TDD y XP

1. Elegir comportamiento pequeño con resultado observable.
2. RED falla por comportamiento ausente, no syntax/dependency/environment. Registrar test/output.
3. GREEN mínimo código+wiring, sin fake release.
4. REFACTOR simplifica, conserva tests/encapsulación.
5. Focused test + fmt/clippy targets; checkpoint amplía fronteras.
6. Commit convencional en rama, evidence/resume.

XP: slices verticales, integración frecuente, ownership con contratos, diseño simple, refactor protegido, feedback UI por slice y ritmo reanudable. Pairing agente/humano para permisos/experiencia, revisión diff; no exigir múltiples agentes ni meetings. Acceptance antes código.

## Herramientas

Rust built-in tests + tokio::test; proptest para DAG/paths; tempfile/SQLite reales; wiremock o httpmock localhost; ClockPort fake; fake keyring. GPUI Kit test-support headless y evidencia nativa separada.
cargo-llvm-cov compatible baseline para coverage; nextest opcional. cargo-fuzz parser/archive en nightly separado app stable; fuzz prioritario inputs untrusted, no buttons triviales.
Criterion domain/SQL, Kit profiler opt-in frames; benchmarks release hardware registrado.
Exact versiones herramientas desde registry fuente T001; commands actualizados si ayuda oficial cambia.

## Matriz mínima

| Test file | Casos |
|---|---|
| core/tests/format.rs | golden, name limits, future schema, YAML aliases, hashes orden/CRLF, missing references |
| core/tests/policy.rs | requiredUnknown strict fail, deps/N-A, staleCI SHA, effective rules+bypass |
| core/tests/guidance.rs | OS/tool/permission ramas, no manualPass, ciclos, facts stale |
| core/tests/library.rs | stale publish, no-op, bump, invalid draft saved, UUID clone, delete/restore |
| core/tests/install.rs | dry plan, unowned/edited conflict, expired digest/head changed |
| core/tests/sync.rs | concurrent heads, descendant prunes, delete/edit, reversed clock, resolve parents all |
| infra/tests/fs_security.rs | traversal/UNC/device/casefold, symlink/reparse/hardlink, bombs, outside-root untouched |
| infra/tests/storage.rs | old migrations, rollback, missing blob, expectedhead concurrent |
| infra/tests/install_recovery.rs | failpoints cada journal/rename/commit, next start old/new coherente, edits preservados |
| infra/tests/agents.rs | 5 probes/versions, HOME overrides, Windows shims, missing auth no read, unsupported scope |
| infra/tests/oauth.rs | PKCE/state/path/oneuse/timeout/bind, refresh1x, invalidgrant |
| infra/tests/drive.rs | pagination, duplicates, timeout same snapshot bytes, 429/403/401, oversize |
| infra/tests/crypto.rs | header vector, all fields tamper, wrongpass safe, bad KDF before cost, crossOS recovery |
| infra/tests/github.rs | denied/unknown, classic+rulesets+bypass, exactSHA, redaction |
| cli/tests/cli.rs | binary flags, JSONschema, exitcodes, noGPU, redaction |
| cli/tests/ci_guard.rs | mandatory badcommit/unknown stops quality |
| desktop/tests/routing.rs | stale query ignored, receipt persists after navigation |
| desktop/tests/library_flow.rs | create/draft/publish/export, focus+keyboard+errors |
| desktop/tests/install_flow.rs | scope/diff/apply/receipt, collision |
| desktop/tests/guidance_flow.rs | completed ->recheck unchanged remainsBlocked |
| desktop/tests/sync_flow.rs | lock/offline/conflict/restore/cancel |

Rutas completas ARCHITECTURE. Unit dominio sin IO/GPU. Medium tempfs/localhost sin cuentas personales. Live opt-in account/repo test con evidence saneada y cleanup deliberado.

## Propiedades críticas

merge idempotente/conmutativo con graph completo; archive reorder samehash/byte mutate different; required missing no strict0; failure install deja old/new coherente o RecoveryRequired; replay ancestor no heads rollback; secrets nunca serialize/log.
Coverage objetivo inicial domain>=85% ramas si tool soporta; fronteras seguridad100% de casos listados. No tests getters/third-party para inflar porcentaje. Measurement ausente se informa, no inventa. No bajar thresholds para verde.

## Comandos futuros

Después bootstrap/lock:
~~~
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p jameskills-core --locked
cargo test -p jameskills-infra --locked
cargo test -p jameskills-cli --locked
cargo test -p jameskills-desktop --features test-support --locked
cargo test --workspace --features jameskills-desktop/test-support --locked
cargo build -p jameskills-cli --release --locked
cargo build -p jameskills-desktop --release --locked
cargo llvm-cov -p jameskills-core --locked --html
cargo deny --locked check
cargo audit --file Cargo.lock
~~~
Desktop feature test-support forwards Kit y UI tests required-features; CI invoca esa feature explícita. Kit glob puede shadow built-in test: imports explícitos; UI #[gpui_kit::test], dominio #[test].
Audit/deny syntax comprobar help pin T001; actualizar scripts/docs/CI juntos. No repetir suite sin cambio/fallo/concern nuevo.

## Evidencia

Cada2–3 tareas checkpoint runnable flow, testing e evidence. Hardware ausente no checkpoint completo aunque fake pasa; avanzar task independiente.
docs/evidence/<task-id>.md futuro: OS/arch/version, command/output saneados, RED behavior, GREEN count, manual steps, screenshots paths, scope regresión, commit.
Screenshots fixture sin email/tokens/path privado. Release QA Ubuntu24.04 Wayland/X11 declared y Windows11 GPU apta, DPI100/150/200, teclado/focus y icons fonts.
Performance REQUIREMENTS p50/p95 machine+dataset release; medir RAM snapshot/KDF y bounded work. No highperformance claim por Rust.
Live Drive disposable vault accountowner: upload->profile nuevo otroOS->restore samepass; dos perfiles edits+delete; disconnect/reconnect. Signing/install smoke aparte unit.
Plan auditing hoy links/IDs/topología/specs/examples, no app tests ni declaración compilado.
