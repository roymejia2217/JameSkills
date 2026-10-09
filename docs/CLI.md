# CLI: verificar una skill contra un repositorio

El comando `check` evalúa las policies de una skill que ya tenga una única
revisión activa en la biblioteca local. No modifica el repositorio y no ejecuta
las suites/tests declarados por la skill.

## Exportar una revisión

`library export` es preview-only por defecto. Selecciona un skill y un destino;
si hay conflicto de heads, especifica el `--revision` que quieres exportar:

```powershell
jameskills library export --skill <skill-uuid> --revision <revision-sha256> --output "<path.jskill>" --json
```

El preview incluye revision/hash/tamaño y el estado hash-bound del destino, sin
escribir ni revelar el path. Si el destino está ausente, aplica con el digest del
preview. Si ya existe, el preview indica `overwrite_required`; solo `--overwrite`
junto al digest exacto autoriza reemplazar ese archivo. Un destino editado después
del preview produce conflicto y permanece intacto. Cancelar consiste en no invocar
`--apply`.

```powershell
jameskills library export --skill <skill-uuid> --revision <revision-sha256> --output "<same-path.jskill>" --apply --overwrite --confirmation-digest <preview-digest> --json
```

Sin `--revision`, solo se exporta una única head content activa. Un conflicto,
tombstone o revisión ajena no se resuelve automáticamente. `--app-data-dir` aísla
el catálogo local en CI; el destino exportado es un archivo aparte.

El `.jskill` es un ZIP portable estándar: paths `PortablePath`, archivos en orden
byte UTF-8, entradas stored con fecha fija y CRC, sin carpeta root añadida ni
campos de trust/local metadata. Inventario expandido máximo 20 MiB/2.000 archivos;
el contenedor ZIP máximo 22 MiB para cubrir los headers acotados. Se incluyen los
bytes fuente del bundle, no SQLite, OAuth, paths absolutos ni recibos.

`--app-data-dir <absolute-dir>` es una opción global para elegir explícitamente
dónde viven config, SQLite y cache locales (en subdirectorios distintos). Sin
ella se usan los Known Folders/XDG del usuario. Es útil para CI y pruebas
aisladas; no se imprimen esos paths en el reporte.

```powershell
jameskills library list --json
jameskills check --repo . --skill <skill-uuid> --profile rust --json --strict
```

`--profile` es `rust`, `node` o `generic`. El CLI inspecciona manifiestos
acotados del root seleccionado y liga cualquier fact `stack` a esa observación;
el argumento no sustituye el stack detectado ni autoriza tools. Una policy que
requiera facts no observados permanece `unknown`.

Los drivers registrados solo se ejecutan cuando el usuario aprueba explícitamente
un ejecutable nativo que coincide con el perfil y SHA-256 actual. Se puede repetir
`--approve-tool <tool-id>=<sha256>` para `git`, `gitleaks`, `gh` o `commitlint`:

```powershell
$gitHash = (Get-FileHash (Get-Command git).Source -Algorithm SHA256).Hash.ToLowerInvariant()
jameskills check --repo . --skill <skill-uuid> --profile rust --approve-tool "git=$gitHash" --strict --json
```

En Linux, el hash se puede obtener con `sha256sum "$(command -v git)"`.
El CLI busca el candidato registrado en `PATH` sin ejecutarlo y rechaza un hash
distinto, entradas duplicadas y wrappers `.cmd`/shims. Solo el SHA confirmado
permite el spawn con argv/profile fijo y `SystemProcessPort`; los demás drivers
siguen `unknown`/`blocked`. No pasar tokens, variables de credenciales ni scripts
como approvals. `gh` puede leer su configuración de usuario a través de las rutas
de entorno allowlisted, pero nunca recibe tokens por argv o env.

Para Commitlint instalado como paquete Node en Windows, no se ejecuta el
`commitlint.cmd`: confirma también `--approve-tool node=<sha256-del-node.exe>` y
`--approve-tool commitlint=<sha256-de-node_modules/@commitlint/cli/cli.js>`. El
profile resuelve el entrypoint JS del package layout registrado; se verifican
ambos fingerprints antes del spawn. En instalaciones sin un entrypoint reconocido,
el check queda bloqueado en vez de invocar un shim.

`--json` devuelve el envelope estable v1. `data.results` enumera todos los
resultados y conserva status, severity, required, enforcement, guidance ID y
evidencia redacted. No incluye paths del root, descripciones arbitrarias del
bundle, stdout/stderr de tools ni findings de secretos. `data.required_passed`
se calcula desde `CheckReport::strict_exit`.

La salida de texto muestra por cada requisito su ID, estado, severidad,
required/recommended, autoridad observada y resumen app-authored de evidencia;
en strict con error también conserva la lista de resultados.

Con `--strict`, cualquier requisito required que no tenga `pass`/`not-applicable`
produce exit code 1 y el informe completo permanece en stdout JSON. Sin `--strict`,
el comando produce un reporte informativo y exit code 0; consultar siempre
`required_passed`, no inferir éxito a partir del código cero. Errores de input,
conflictos de heads y capabilities tienen los exit codes del contrato CLI.

La revisión debe ser única y no tombstone. Un skill ausente devuelve NotFound;
varios heads devuelven conflicto en vez de elegir uno. Los bytes se cargan y se
validan de nuevo antes de evaluar.

Cada driver de tool necesita selección/fingerprint explícitos según su contrato.
Sin aprobación no se ejecutan candidatos de PATH y sus checks quedan `blocked`
o `unknown`; los checks de filesystem que no requieren proceso pueden producir
evidencia real. `required_passed: false` es una salida esperable mientras falten
permisos, tools o evidencia host.

Para CI, importa primero la skill fixture al storage aislado del job, conserva el
UUID mostrado y luego invoca el binario CLI con `--strict --json`. Import/preview
es read-only; apply requiere resolución y digest exactos del preview. Un resultado
local nunca acredita protección de rama ni Required CI en el host.
