# Requisitos de producto y alcance v1 — JameSkills

Estado: dossier de implementación solicitado por el usuario. Autoriza redactar todas las especificaciones y el plan; la aplicación aún no existe. CAPABILITY-MAP.md es el índice de módulos. tasks/todo.md contiene trabajo pendiente, no progreso implementado.

## Objetivo y usuarios

Una persona mantiene suites de conocimiento y controles reutilizables, las instala en sus agentes de código, verifica repositorios y recupera la biblioteca entre sus equipos Windows/Linux. Una skill es una unidad portable de instrucciones, políticas tipadas, evidencias, guía de requisitos, referencias y templates. La UI comunica lo que está aplicado, lo que solo está recomendado y lo que no puede verificar.

## Alcance de entrega completo

| ID | Capacidad | Criterio de aceptación v1 |
|---|---|---|
| R01 | App nativa | Ventana GPUI Kit empaquetada Windows x86_64 y Linux x86_64, sin webview para el producto |
| R02 | Biblioteca offline | Crear, editar, validar, versionar, importar/exportar, buscar y eliminar/restaurar skills sin red |
| R03 | Suite portable | SKILL.md + manifest JameSkills + políticas + guía + assets; export sin rutas absolutas/tokens |
| R04 | Cinco perfiles | Codex, OpenCode, Pi, Antigravity CLI y Grok Build detectados por contrato; cada capability soportada tiene fixture real y aceptación local |
| R05 | Instalación segura | Plan previo, colisiones visibles, staging, receipt, recovery tras fallo; remover solo contenido gestionado no modificado |
| R06 | Políticas | Commit/README/PR-main/CI/releases/secret scanning/gitignore con evidence, severity y autoridad explícitas |
| R07 | Guía dinámica | Reprobar el entorno tras cada paso; no dar por cumplido un requisito porque el usuario pulsó Continuar |
| R08 | Drive | OAuth Desktop PKCE mínimo scope; sync explícito y periódico solo durante app activa; estado offline/reautenticación claros |
| R09 | Cifrado/restauración | Copia cifrada por defecto, passphrase portable, autenticación antes de restore, vista de diff y recovery local |
| R10 | Conflictos | Dos equipos offline producen dos revisiones conservadas; delete causal; merge explícito sin pérdida por reloj |
| R11 | Calidad/seguridad | TDD en comportamientos, CI Windows/Linux, ningún secreto en logs/backup/fixture, proceso limitado por allowlist |
| R12 | Distribución | Artefactos checksum, licencias, instalación/desinstalación documentada, evidencia de GUI real en ambos OS |

## Decisiones de alcance

- v1 single-user: la cuenta Drive y la passphrase no se comparten como sistema de equipos.
- Idioma inicial español, mensajes estructurados con IDs preparados para i18n; no traducir archivos propios del usuario.
- Windows/Linux x86_64 como release v1. Detectar ARM64 y explicar compatibilidad; no instalar binarios de arquitectura distinta. ARM64 queda capability experimental hasta CI y dispositivo nativo verificados; no prometido en release.
- Operación offline primero: Drive desconectado jamás bloquea biblioteca/validación local.
- Sin demonio ni instalación global de agentes automática; permitir enlazar un binario/perfil local con comprobación. WSL se trata como entorno distinto; no mezclar rutas Windows/WSL ni instalar desde una app en el otro entorno silenciosamente.
- Soporte remoto inicial GitHub. GitLab/otros se reportan Unsupported y presentan guía del proveedor; no devuelven Pass.
- Proyecto Rust y proyecto Node tienen perfiles CI estándar; otros stacks permiten instrucciones y reportan requisitos de CI no verificados hasta driver.
- Checks pueden bloquear instalación strict y CI. Los hooks y el texto son eludibles; la protección host se valida aparte, incluyendo bypass/admin si API permite.
- La guía no necesita un modelo remoto. Un grafo tipado evalúa capacidades, elige pasos, interpola valores no secretos y revalida. Un LLM futuro opcional produce sugerencias sin autoridad para ejecutar o confirmar cumplimiento.
- Aprobación de archivos/permisos requerida cuando la acción es explícitamente invasiva; el plan de producto la presenta como diff y consecuencia, no una sucesión de confirmaciones rutinarias.

## Invariantes

I01 Ningún import, preview o render ejecuta código contenido en la skill.
I02 Ningún Unknown/Blocked/Unsupported cuenta como Pass de un requisito obligatorio.
I03 Existe exactamente una fuente canónica por revisión; export agente es derivado.
I04 Guardar tiene expected-head; datos obsoletos no sobrescriben cambios.
I05 Un backup no contiene OAuth, tokens GitHub, secretos keyring, paths locales ni receipts de máquinas.
I06 Error/crash conserva último estado local consistente y backups anteriores.
I07 Instalar/remover no toca archivos no gestionados ni nombres/rutas fuera de destinos aprobados.
I08 Ninguna red va a endpoint configurado por una skill.
I09 Un conflicto nunca se resuelve por timestamp local.
I10 Todo handler GUI llega a un caso de uso real o se presenta como Unsupported explícito.

## Rendimiento — presupuestos, no resultados medidos

Entorno referencia: Windows11 o Ubuntu24.04 x86_64, 4 núcleos/8GB, SSD, GPU Vulkan compatible Linux; build release. Fixture 1.000 skills, texto 32KiB cada una, catálogo total <100MiB.

- Inicio a primera ventana útil p95 <=2s (exclude primer compilado/OAuth/KDF).
- Filtro local paginado 50 items p95 <=100ms; debounce entrada 150ms.
- Ninguna IO de archivo/red/crypto ni espera mutex en render; interacción local p95 <=50ms.
- UI lista virtualizada: frames p95 <=16.7ms en referencia 60Hz.
- Idle RSS objetivo <=180MiB con catálogo fixture; no se rechaza una implementación por número sin evidencia de hardware/medición.
- Validación 100 policies internas <=500ms; herramientas externas reportan progreso y cancelan.
- Copia 100MiB no duplica archivos sin límite en RAM; upload/download bounded y máximo de payload descomprimido 256MiB.
- Logs de rendimiento sin nombres/contenidos de skills, rutas ni tokens.

## Definition of Done del proyecto

Toda R01–R12 enlaza tests/evidencia. Ningún TODO/stub crítico, ningún capability falsamente marcado soportado; no se declara release multiplataforma con solo cross compile. OAuth y GUI real tienen pasos que requieren dueño de cuenta/dispositivo: se registran blocked con instrucciones; siguen ejecutables las pruebas deterministas offline. La entrega no puede marcarse íntegra hasta realizar esos pasos, pero el ejecutor puede seguir tareas independientes.

## Documento y código

Las firmas/código de docs/CONTRACTS.md son blueprint propio. Los snippets GPUI remiten a fuente fijada. Este dossier no afirma código compilado, tests de aplicación pasados, repo protegido ni Drive conectado. El ejecutor convierte blueprints en archivos, escribe el test que falla y registra la evidencia de TDD.
