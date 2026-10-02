# Spec: agent-adapters, detección e instalación

R04/R05/I07. Los contratos observados están en SOURCES. Fuente común siempre Bundle; AgentExport contiene solo un árbol de archivos y capability notes. Copias reales por defecto para evitar privilegios de symlink Windows. Los archivos extra de suite (policies, guidance) acompañan al SKILL.md, pero cada agente puede tratarlos solo como referencias.

## Matriz v1

| AgentId / CLI | Project | User | Verificación y particularidades |
|---|---|---|---|
| Codex / codex | <repo>/.agents/skills/<slug> | <home>/.agents/skills/<slug> | SKILL.md requerido, openai.yaml opcional generado desde metadata validada; no ~/.codex/skills por asumir docs viejas |
| OpenCode / opencode | <repo>/.opencode/skills/<slug> | ~/.config/opencode/skills/<slug> | .agents también reconocido; XDG_CONFIG_HOME override solo cuando versión/implementación consultada confirma |
| Pi / pi | <repo>/.pi/skills/<slug> | <PI_CODING_AGENT_DIR o ~/.pi/agent>/skills/<slug> | Puede leer .agents; /reload verifica discovery; Windows shell propio Pi puede exigir Bash |
| Antigravity / agy | Unsupported standalone mientras contrato proyecto CLI no documentado | Vendor plugin install; ~/.gemini/antigravity-cli/plugins/<plugin-name>/ | plugin name jameskills-<slug>; plugin.json + skills/<slug>/; no instalar CLI en ruta IDE .agent |
| Grok / grok | <repo>/.grok/skills/<slug> | <GROK_HOME o ~/.grok>/skills/<slug> | grok version; grok inspect --json permite comprobar discovery sin iniciar una tarea de modelo |

HOME significa home user obtenido OS, no literal ni APPDATA común a todos. En Windows join por PathBuf, preservando Unicode/espacios. Linux app desktop puede no heredar PATH de shell; mirar únicamente roots instalador documentadas y overrides declarados, sin recorrer disco completo. PATH+Known Folders+config presences sirven para detectar candidatos, no verificar autenticación ni capacidades.

Antigravity plugin.json exacto mínimo:
~~~json
{"name":"jameskills-repository-foundation","description":"Estándares Git y CI de repository-foundation"}
~~~
Docs vendor muestran $schema y additionalProperties=false ambiguos sobre $schema: v1 emitir solo name/description, confirmado por validator/CLI fixture. No generar hooks.json ni MCP/agents como efecto secundario.

## Detección

platform::detect_environment -> OS/arch/native-vs-WSL/toolpaths user-home/known folders. Config override permite executable+profile-root separados; confirmar fingerprint y límites path.
find_candidates: orden explicit user mapping -> PATH -> vendor documented local roots; dedup same canonical executable. Windows distinguir .exe, .cmd, .ps1 y aliases.
probe_version: ApprovedExecutable y registry validado, timeout3s, salida64KiB, sin network/auth/interactive flags. Probe command diferente por agent según --help fixture; Grok usa version.
classify_capabilities: defaults NeedsVerification, supported solo con contrato formato+version fixture+paths comprobados. Un número versión nuevo no cabe automáticamente en un rango que no se probó.
Detect Presence sin binary = Candidate/needs local guidance. CLI ausente no ocultar card, mostrar "Instalar agente" con docs official y Reintentar.
No leer auth.json/settings completos ni env tokens. Leer claves públicas necesarias mediante parser restringido y redaction; settings rutas privadas no entran logs/sync.
Si paquete tiene firma/installer provenance verificable, probe automático según autorización de producto; si executable arbitrario user-picked, aprobación concreta a ejecutable fingerprint antes probe.
Windows npm shims: resolver entrypoint a node aprobado de paquete instalado identificado; no pasar string arbitraria a cmd.exe/powershell ni tratar .cmd como ejecutable nativo. Test con paths espacios, &,$,unicode y wrapper modificado.
Arquitectura unsupported produce guía oficial del agente; no ejecutar emulación/bajar binario automática.

## Export

render_export toma bundle validado+reviewed, no transformaciones semánticas del cuerpo. Agrega instrucciones relativas de cómo verificar suite con CLI JameSkills si existe; si no existe presenta validación manual sin claiming applied.
Para agents que comparten .agents/skills, una copia es asset compartido, registro contiene asociaciones de agentes. El slug colisiona entre UUID: no instalar automáticamente suffixed name sin actualizar frontmatter+manifest y preview; usuario Clone/rename o elegir destino.
Metadata de UI opcional solo supported schema. Omitir optional feature y explicar si versión no soporta. No incluir tokens/config usuario/rutas absolutas.

## InstallPlan y transacción filesystem

Estado journal: Planned -> Approved -> Staged -> OldMoved -> NewMoved -> Verified -> Committed; Failed/RollbackPending/Recovered alternativas.
1. resolver destination dentro approved root; inspeccionar no symlink/reparse/hardlink, límites y hashes; descubrir contenido ajeno/managed.
2. plan files + expected current hashes + intended agent capabilities. strict local requeridos bloquean, remote pre-release no bloquea instalar salvo policy declara phase.
3. UI/CLI confirma digest; no plan firmado por archivo importado. Validar source revision/head y tool identity otra vez.
4. lock por canonical destination entre procesos app/CLI usando OS advisory lock (fs2 o alternativa comprobada); almacenar operation SQLite antes mutar.
5. stage sibling mismo filesystem con permisos privados; validar bytes/hash/source. Nada en temp global escribible.
6. destino nuevo -> rename staging a final. Destino owned no editado -> move old recovery, rename new. Dos renames no atomic juntos; journal con fsync file/directorios soportado y fases recoverable.
7. verify file hashes, discovery si API soporta; ausencia runtime discovery es NeedUserVerification, no "cargado". Persist receipt; borrar old después commit y conservar recovery por retention documentado.
8. si falla, rollback usando solo hashes que aún coinciden; si usuario editó durante error, no destruir, presentar recovery.
No tocar worktree git (stage/commit) ni config de agente. PermissionDenied guía permisos; jamás run-as-admin default.

## Vendor mutation Antigravity

stage export plugin -> journal -> approved agy plugin install <stage> -> verify list/package files -> receipt.
Antes comando guardar inventory y exigir plugin name ausente o JameSkills owned no modificado. Upgrade no safe vendor overwrite probado -> rechazar upgrade auto y guiar uninstall/install usando plan concreto.
Timeout no garantiza plugin no instalado: re-inspect vendor inventory y hashes antes repetir.
Compensación uninstall solo nombre recién creado cuya receipt corresponde y files sin edits; no borrar plugin ajeno o prior registry.
Vendor puede sync cross surfaces: aviso efecto en perfil/vendor nube; JameSkills no controla atomicidad interna vendor. Si CLI incompat no simular éxito por copiar carpeta.
Remove agy usa vendor uninstall con autorización concreta tras hashes; registra diferencias.

## Tests

agents.rs fixtures version probes cada cinco; source snapshot fixture metadata tested version/date; resolver OS paths matrix overrides, no creds read.
install_recovery.rs failpoints tras cada journal/rename/SQLite commit; abrir app nuevamente deja old o new coherente, no mezcla; permisos/dest editado/locks/interprocess.
Windows nativo tests reparse points sin requerir developer privileges cuando fixture posible; si no posible marcar manual evidence faltante, no omitirse en DoD.
Discovery contract: Grok inspect JSON; otras CLIs usar mecanismo real documentado y manual current session cuando no automatizable sin modelo/API. Conectar al agente no ejecuta task de modelo durante test.
Aceptación por capability: solo supported scopes verdes, unsupported explicados; mínimo user scopes cinco y project cuatro con instalación/desinstalación revisable y evidencia real para release.
