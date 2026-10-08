# Seguridad y protección por frontera

Modelo de amenazas para implementación, no certificado de seguridad. Contrato crypto único: SPEC-cloud-sync; no debilitar controles para terminar una tarea.

## Activos y actores

Activos: biblioteca/historial, datos proyectos, archivos agentes, OAuth tokens, master key/passphrase, confianza del usuario y evidencia de checks.
Actores: usuario legítimo, bundle/ZIP malicioso, skill con prompt injection, CLI/proceso no confiable, respuesta remota corrupta/replay y otro proceso local.
Compromiso OS/malware mismo usuario no se impide con passphrase/keyring: biblioteca local plaintext. No prometer sandbox por Rust/GPUI.
Datos externos nunca autorizan ejecución/redirección/privilegios/trust automáticamente.

## Fronteras y controles

| Frontera | Amenaza | Control normativo | Verificación |
|---|---|---|---|
| Import/fs | traversal/bomb/symlink/reparse/case collision | PortablePath, límites, staging/blob privado, expected-heads, TrustState local Quarantined por default | import/safe_bundle adversarial |
| Parse | YAML aliases/tags/schema ambiguo | Parser restringido, bounded bytes, unknown fields fail | format malicious |
| Instructions | prompt injection/secret commands | datos inertes en app, trust review antes instalar | import no execution, review flow |
| Tools | PATH hijack/npm shim falso | candidate provenance, identity hash aprobada y args driver | resolver changed binary |
| Process | injection/leak/hang | argv allowlist, env mínimo, timeout+kill child group | process exact argv/descendants |
| Host | falsa protección/stale CI | effective rulesets/classic+bypass+SHA exact, Unknown no pass | github denied/bypass fixtures |
| Install | overwrite/TOCTOU/crash | owner hashes, locks, no-follow handles, journal recovery | failpoints+interprocess |
| SQLite | injection/stale save | bind params, transactions, expected-head, migration backup | storage real tests |
| Repository bindings | project path/head/facts se filtran o quedan stale | binding/report en tablas SQLite local-only, revisión suite exacta, root/head/environment fingerprint revalidado; ausentes de bundle/backup/snapshot | bindings cambia root, WSL, suite revision y conserva resultados solo como stale |
| OAuth | interception/CSRF/leak | browser PKCE/state/loopback oneuse, keyring | oauth attack fake localhost |
| Drive | SSRF/lost writes/duplicates | fixed HTTPS hosts, pagination, snapshots inmutables+DAG | reordered fake remote |
| Crypto | tamper/KDF DoS/OS-tied restore | RustCrypto, fixed params before KDF, independent OS-random nonces, AAD, passphrase wrap, authenticate-before-decode | per-header-field/wrapped-key/payload tamper, wrong passphrase, RNG failure, immutable retry bytes |
| UI jobs | stale mutations/cancel falso | request+generation+revision, durable receipts | reducer concurrency |
| Logs/exports | secrets leak | typed redaction, allowlist payload, sin paths/creds | canary absent serialization |
| Release | supply chain/binary altered | locked deps, pinned Actions, scopes, checksum/signature | CI+package verify |

## Filesystem

Linux dirs0700/files privados0600; Windows ACL usuario KnownFolder, comprobar no Everyone broad grant, no chmod falso. No elevar privilegios default.
Approved root canonical una vez y después relative handles con no-follow; no basta string starts_with. cap-std/fs2/windows-sys candidatos requieren comprobar semánticas reales; no asumir cap-std prohíbe todos symlinks. Ancestor chain/reparse y concurrent swap tests cuando OS permite.
Cross-device rename se rechaza o staging same fs; recovery solo owned hashes unchanged. Locks orden fijo por recurso, UI no bloquea render en mutex.
Export picker valida destination/overwrite; DB path mostrado y override startup seguro; no mover SQLite abierta.
Markdown HTML/remote image autoload deshabilitado. SVG arbitrary no render si sanitizer mantenido ausente; conservar bytes cuarentena. Iconos Kit vetted sí. Library asset preview entrega el SVG solo como texto UTF-8 bounded, nunca como markup activo; binarios permanecen Unsupported y no se ejecutan.

## Secret handling

SecretInput no Serialize, Debug redacted, zeroize drop. Infra sanea errores terceros antes AppError.
Refresh token keyring, access RAM, master+wrapping key de sesión cache solo opt-in keyring, nunca passphrase original. Lock quita llave runtime, biblioteca source local sigue legible. Windows Credential Manager/Linux Secret Service backend conforme library pin; missing vault=Blocked, sin fallback plaintext.
No dump env/read auth.json de agentes. Gitleaks redacted obligatorio pre-upload para revisions nuevas; unavailable guía/Blocked; findings conservan local, impedir cloud hasta remover.
Backup excluye drafts/receipts/project paths/keyring/evidence. Nunca zip todo appdir.
Fixtures canary falso, pruebas stdout/json/log/archives metadata no lo revelan.
No passphrase argv/env/log/UI event. Account labels locales sin afirmar email no observado.

## Crypto

SPEC-cloud-sync fija header176/AAD/KDF/wrap/encrypted ZIP. No variante paralela aquí.
.jskill export legible explícito; .jskills-backup encrypted mandatory. Unknown version ->Unsupported, no downgrade/heurística. Size/header limits BEFORE KDF; authenticate whole payload BEFORE decompress/restore.
Salt estable vault normal; wrap/payload nonces fresh independientes. RNG fallo aborta.
Replay ancestor no reduce heads; vaultID binding evita cross-vault merge.
Rotación contraseña usa nuevo vault/master; old exports no se revocan mágicamente. Google ve IDs opacos/tamaños ciphertext, no contenido/nombres skill.
`CryptoProvider` usa Argon2id v19 con parámetros fijos, XChaCha20Poly1305 y los AAD byte-exactos del header v1. Wrong passphrase, wrap/tag/payload/header tamper comparten `CryptoInvalid`; no se llama al ZIP decoder hasta que la autenticación de todo el payload haya pasado. Las claves y buffers de plaintext de sesión son `Zeroizing`; cada error de OS CSPRNG aborta la operación sin reutilizar nonces.

## Red y procesos

Endpoints registrados app; no skill URL ejecutable. Proxy usuario opt-in conserva TLS; nunca --insecure para desbloquear.
timeouts+output/download cap+bounded retry jitter/cancel. No cloud background antes connect.
Linux child process group/Windows Job Object cancel incluye descendientes; checks no installers interactivos. Repo package scripts necesitan trust.
GitHub gh auth suyo, app consume estado JSON redacted. No extraer token leyendo archivos.
Agente install no incluye hooks/MCP arbitarios del bundle. Bridge hook futuro app-owned versioned y permiso explícito.

## Publicación

Cargo.lock commit; dependency update PR tests fuente; Actions SHA verificado, mínimo contents:read CI; write solo release environment protegido. Nada pull_request_target ejecutando PR con secrets.
Gitleaks/audit/deny reales y pinned. Advisory blocking no se ignora sin evaluación explícita y plazo registrado.
Rulesets main PR+quality+review y bypass report real. Owner host requiere guía verificable; JSON template no aplica settings.
Signing material solo CI seguro owner; ningún secret repo/Drive. Sin provisión no badge Signed.
Installer per-user, sin instalar agentes ni admin default; uninstall conserva biblioteca, purge separate consent.
SECURITY raíz proceso disclosure definido owner, no email inventado.

## Revisión por tarea

¿Nueva frontera? ¿parse bounded? ¿secrets serialized? ¿filesystem replace? ¿argv trusted? ¿cancel children? ¿Unknown vs Pass? ¿expected-head? Registrar hallazgos en evidence. Pérdida de datos/secrets impide cerrar checkpoint/release.
