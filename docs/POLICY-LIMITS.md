# Límites de evidencia de políticas GitHub

## Branch protection y rulesets

JameSkills no escribe configuración en GitHub. Para `github-branch-policy`,
`GET /repos/{owner}/{repo}/rules/branches/{branch}` sirve como fuente de reglas
efectivas activas de repositorio y ámbitos padre; GitHub excluye reglasets en
`evaluate` o `disabled`. Se consulta además classic branch protection para
reviews y required status checks. Ambas respuestas deben estar completas y
parseables antes de afirmar `HostRule`.

El endpoint classic puede responder 404 por falta de regla visible, permisos o
plan; por eso 404 es `Unknown`. Una lista effective vacía combinada con una
respuesta classic 200 que declara campos ausentes/null permite detectar la
falta de PR/check rules y responder `Fail`. Reglas desconocidas, datos parciales
o listas que alcancen el límite de 100 sin poder descartar truncamiento son
`Unknown`.

La lectura reconoce los tipos de regla `pull_request` y
`required_status_checks`; los contexts classic y los efectivos se unen para
comparar los checks requeridos por el profile. No se reportan nombres de
usuarios, equipos o apps, URLs ni cuerpos REST en evidencia.

El estado de bypass se evalúa por separado. Las reglasets aplicables se
seleccionan para el branch consultado; condiciones glob o valores que no se
puedan resolver conservadoramente producen `Unknown`. `bypass_actors` puede
omitirse si el usuario no tiene write access al ruleset y
`current_user_can_bypass` solo describe al actor autenticado. Classic
`enforce_admins` y allowances también se consideran. Actor visible significa
`Fail` si se exige no-bypass; datos ocultos significan `Unknown`. Si no se exige
no-bypass, la regla/PR puede pasar con un campo `bypass=unknown` explícito; eso
no afirma que main sea absolutamente inaccesible.

## CI de un SHA exacto

`ci-evidence` es un check distinto de `ci-contract` y de branch protection.
Solo puede devolver `RequiredCi` cuando una regla activa exige los checks del
profile y los resultados obtenidos para el SHA local exacto son terminales y
exitosos. El endpoint acepta branch/tag además de SHA; el driver usa únicamente
el `RepositoryHead` hexadecimal leído localmente por Git y verifica cada
`head_sha` devuelto.

Check runs (`completed` + `success`) y legacy commit statuses (`success`) son
fuentes diferentes. Si la regla host vincula un context a `integration_id` o
`app_id`, solo un check run emitido por esa misma app satisface el context; un
commit status o check de otra app no sustituye la fuente restringida. Contexts
sin app binding pueden observarse por run o status, pero contradicciones entre
fuentes son Fail/Unknown. `pending`, ausencia de check, conclusión fallida, SHA
distinto, permiso insuficiente o respuesta truncada nunca son Pass. La lectura
de un successful check sin regla host obligatoria tampoco satisface
`RequiredCi`. Sin permiso para inspeccionar el estado o el enforcement, el
resultado es `Blocked`/`Unknown` según la causa.

## Versiones y releases

`release-contract` obtiene la versión del proyecto del `Cargo.toml` del root
(versión de paquete/workspace) o de `package.json`; si ambos existen deben
coincidir. Manifiestos ausentes, inválidos, demasiado grandes o no regulares
dejan la versión `Unknown`; una divergencia conocida es `Fail`. No se consulta
`SkillManifest.semantic_version`. El tag de release debe ser exactamente el
SemVer del proyecto, con o sin prefijo `v`.

La lectura de `GET /repos/{owner}/{repo}/releases?per_page=100` separa releases
publicadas de Git refs. Un listado completo que no contiene la versión requerida
es `Fail`; respuesta de 100 elementos, 403/rate limit es `Blocked`, y 404,
respuesta inválida o permisos ambiguos son `Unknown`. Releases draft o prerelease
no satisfacen el contrato. La presencia de un tag sin release publicada nunca
cuenta como publicación.

Si `require_changelog=true`, `CHANGELOG.md` debe ser un archivo regular acotado
con un heading Markdown cuyo SemVer coincida con el proyecto y contenido de
notas no vacío bajo ese heading. Si
`require_checksums=true`, la release debe tener assets y cada asset debe incluir
un `digest` con formato `sha256:<64 hex>`; digest nulo o inválido es `Fail`,
campo ausente/truncado es `Unknown`. Esto es metadata SHA-256 informada por
GitHub, no una descarga ni un re-hash local del artefacto.

Si `require_signature=true`, el driver resuelve `refs/tags/<tag>` y requiere un
tag anotado cuyo endpoint de tag informe `verification.verified=true` y
`reason=valid`. Tag lightweight/unsigned o firma conocida inválida es `Fail`;
falta de permisos, clave/servicio no verificable, metadata contradictoria o
respuesta incompleta es `Blocked`/`Unknown`. No se guarda firma, payload,
identidad ni explicación cruda; `verified` no expresa por sí solo que la app
confíe en una identidad de firmante concreta.

Las llamadas GitHub usan solo `gh` aprobado/fingerprinted, GET fijo, salida y
tiempo acotados, sin paginación automática ni descarga. Un check satisfactorio
es evidencia read-only (`LocalCheck`), no crea tag/release, no prueba que el
artefacto se haya descargado/reproducido y no afirma autorización para publicar.

Todas las observaciones son read-only, seriales, acotadas por salida/timeout y
TTL monotónico. No se sigue paginación ilimitada ni se aplican reglas remotas.
