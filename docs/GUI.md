# GUI y estructura lógica — JameSkills

Diseño minimalista con controles GPUI Kit, jerarquía clara y feedback verificable. Plan de interfaz nativa, no un mock de web. Todos los números/estados provienen del dominio. Documento asociado SPEC-desktop-app.

## Layout maestro

Ventana1280x800 inicial, mínimo1000x680. Rail izquierdo200px, encabezado56px, contenido adaptable; panel de requisitos320px opcional en skill detail; status bar28px. Spacing base4px y controles mínimo32px altura. Content max sin forzar stretch al texto.

~~~
+--------------------------------------------------------------------------------+
| JameSkills                         Buscar skills...        [Crear skill]        |
+---------------------+----------------------------------------------------------+
| Biblioteca          | Biblioteca                         [Importar] [Filtros]  |
| Proyectos y checks  | Estado: Todas  Agentes: Todos                            |
| Agentes             +----------------------------------------------------------+
| Sincronización      | Skill                 Versión  Control       Instalación |
|                     | Repositorio seguro    1.0.0    2 pendientes  Codex, Pi   |
|                     | Revisar código        1.2.0    Validada      OpenCode    |
|                     | ... virtualizado ...                                    |
|                     |                                                         |
| Ajustes             | Sin skills: descripción + Crear + Importar              |
+---------------------+----------------------------------------------------------+
| Linux x86_64   Biblioteca local lista       Drive: desconectado [Conectar]      |
+--------------------------------------------------------------------------------+
~~~

Una fila seleccionada abre detalle, no muta skill. Menú contextual ofrece exportar/clonar/borrar con estado. Botón primario único por pantalla principal (Crear, Publicar, Instalar, Sincronizar según ruta).

## Componentes, tokens y assets

| Necesidad | Kit | Propiedad de producto |
|---|---|---|
| Acciones | Button variantes primary/outline/ghost/danger | Destructivas con texto y preview |
| Texto/búsqueda | Input con retained InputState | Labels/focus/errors, debounce |
| Navegación | Tabs/Sidebar primitives reales catálogo | keyboard seleccionado y route |
| Catálogo | Table o List virtualizada | Page50 + selection UUID, no index |
| Requisitos | Card/Badge/Tag/Icon | CheckStatus+autoridad+acción |
| Modales | Dialog/Sheet | focus trap, Escape cancel antes commit |
| Progreso | Progress/Spinner | indeterminate solo si backend no porcentaje |
| Feedback | Notification/Toast + persistent panel | no secret details, copy diagnosis redacted |
| Tooltips | Tooltip | complementa label, no info indispensable solo hover |

Theme tokens: background/surface/foreground/muted/border/primary/destructive/success/warning; tomar ActiveTheme del Kit. Paleta propuesta neutra (dark/slate y light/gris), acento azul del theme. No hardcodear estados por color sin texto. Texto14px base, títulos20/24px, texto auxiliar12px mínimo; mono para paths/hashes y editor16px ajustable.
Iconos Kit basados Lucide/Isocons: Biblioteca, Search, Plus, Shield, GitBranch, Cloud, Settings, Check, Alert, Download, Upload, Folder si enum0.7 los ofrece; cada nombre se verifica en assets fijados. Sin logo externo inventado ni copiar librería de iconos web. Assets default se registra y un test verifica no fallback missing.

## Onboarding

Primera ejecución ofrece ruta datos default con explicar persistencia local, botón Continuar, detección background de agentes candidatos. Selección "Trabajar localmente" completa onboarding; sync opcional en su propia ruta. Mostrar permisos realmente necesarios. Herramientas ausentes ofrecen guía y retry, no instaladores mágicos.

Si perfil SO/arch unsupported: banner detalle compatible y ruta CLI headless, links official; no claim compatible falsa. Windows app y WSL son tarjetas separadas para scope actual. No auth cloud obligatorio.

## Biblioteca y editor

Tabs overview/instrucciones/políticas/requisitos/archivos/versiones/instalar.
Header display name, slug, versión publicada, draft indicator. Acciones Guardar borrador, Publicar revisión, Validar.
Overview: descripción, tags, source/trust status, tools requeridas, coverage summary con link a report. "Solo instrucciones" visible cuando policies vacías.
Instructions: editor plain Markdown seguro y preview sin HTML activo; frontmatter fields mediante formulario asociado. Validación línea+columna y link file.
Policies: formulario checks tipados y lista requisito/severity/phase/enforcement; TOML experto con validación live pero draft puede guardar inválido.
Requirements: panel lateral grafo/facts/current checks; "Verificar" llama service, progress cancelable; cada row tiene motivo y fecha/fuente.
Assets: árbol normalizado, preview text bounded, scripts icono riesgo trust, links no automáticos. Añadir archivo importa safe copy, no privilegios.
Versions: parent graph/status conflict, hash corta, versión/summary, restore crea nueva. No permitir snapshot hash edit manual.
Draft autosave text "Guardado" solo tras evento persistido; error ofrece retry+export draft emergency safe. Publish disabled invalid muestra reason, no bloquea editar.
Slug edit preview effects en export/agent installed names; identidad UUID estable. Cambiar publicado requires version bump.

## Panel guía dinámica

~~~
+------------------------------------+
| Falta proteger main                |
| Bloquea: publicación               |
| Observado: ruleset no visible       |
| Autoridad: proveedor Git           |
|                                    |
| 1 Vincular acceso GitHub [Verificar]|
| 2 Solicitar a admin requerir PR     |
|   [Abrir guía oficial]             |
| 3 Configurar check quality         |
| 4 Verificar branch actual          |
|                                    |
| [Ya lo completé: volver a verificar]|
| Evidence aún no verificada         |
+------------------------------------+
~~~

Seleccionar "hecho" no cambia Pass sin verificación. Pasos se adaptan Windows/Linux, tools/versions/permissions/host; la UI explica qué fact eligió rama. Cuando depende admin o API no accesible, deja Unknown+instructions, permite trabajar localmente y conserva bloqueo en gate obligatorio.
Preguntas choice no secretos: stack Rust/Node/generic, provider GitHub/other, repo path. Secret entry usa credencial aparte sin copiársela al agente/contexto.
Mostrar command y efecto antes copiar; user puede ejecutar external. "Verificar" distinto "Aplicar cambio", sin merge automático.

## Proyectos y checks

Project picker reciente local, no nombres sync. Cada project tiene perfil/rama/SHA y requirements. Header Validar + settings profile. Lista filtrable failed/blocked/unknown.
Resultado "Validada" requiere todos mandatory phase elegida Pass o NotApplicable legitimado; otra phase separada. Advertencia hooks eludibles en detalle autoridad, no asustar al usuario innecesariamente.
Diff preview aplica templates sin sustituir README ajeno; líneas nuevas/modified y backup recovery, button Aplicar selected. Main remote guidance no toggles que aparentan seguridad ya instalada.

## Agentes e instalación

Cards cinco agentes siempre visibles: missing/candidate/verified, executable path redacted solo usuario local, versión+scope capabilities. Re-detect button; elegir path manual abre native picker y verifica.
Seleccionar skill revisión -> scope -> project if needed -> Plan:
- lista archivos y destination;
- colisiones con owner/edits;
- diferencia instruction vs enforced;
- requerimientos pre-install y efectos vendor.
Instalar button enabled solo plan valid/current y reviewed trust. Operation progress no spinner sin cancel/fase. Final receipt "Archivos instalados" + discovery verified o "Verifica carga en el agente".
Antigravity User plugin supported, Project scope explicación ausencia contrato CLI. No fake project install en .agent.
Remove preview solo owned hashes; edited -> conservar y explicación. Nunca botón borrar global skill folder indiscriminado.

## Sincronización y restore

Disconnected: utilidad + Conectar Google Drive + Exportar copia local.
LinkedLocked: cuenta etiqueta local, vault UUID, "Desbloquear copia", export encryption; no email inventado.
Create vault: passphrase twice, recovery explanation, download encrypted copy, opt-in recordar llave en keyring. Enter passphrase nunca texto en preview/log.
Ready: pending count real+último sync UTC+size bounded, SyncNow, backup local, Restore, Disconnect.
Syncing progress fases list/download/verify/merge/upload; porcentaje bytes real solo transfer; cancel comunica operación commit boundary.
Error states: offline Pending; OAuth NeedsReauth; quota guía; wrong password safe; corruption quarantine; unsupported future envelope.
Conflicts: side-by-side canonical revisions source labels+parents+files diff, buttons Usar local/Usar remoto/Combinar/Conservar ambos; preview resultado/parent links. Cualquier elección crea revision resolutiva y luego sync; no borrado automático de otra branch.
Restore preview: número skills/revisions/conflicts/bytes, versión/timestamp, mode merge/replace-with-history, local recovery path; Aplicar then receipt. No reinstalar agentes del equipo anterior.
Password rotation v1 = nuevo vault y reencrypt, explicación copias viejas conservan password vieja.

## Ajustes

Theme system/light/dark, idioma inicial español, data paths visibles con Abrir carpeta; override solo al iniciar y migración guiada mediante export/restore, nunca mover DB abierta, tool paths trust, sync period>=5min, logs retention y export redacted.
No tokens en tabla settings; ver Estado keyring y guía SO. Reset remote backup acción aparte, destructiva con file list+export previo.
Sobre: versión, licenses, checksum metadata, diagnostic environment redacted.

## Accesibilidad y pruebas por pantalla

Tab/ShiftTab orden lógico, focus visible, Enter activation, Escape cancel seguro, close dialog restaura focus. Ctrl+K búsqueda, Ctrl+N nuevo, Ctrl+S save draft, Ctrl+Shift+S publish si valid; comandos documentados y no colisión OS.
Label todo Input; status announcement no flooding; contrast4.5:1 texto normal/3:1 large/icon info; probar temas sin que color sea único canal.
DPI100/150/200%, fonts sistema, window minimum, resize/long español/paths unicode, teclado completo.
Headless Kit interacción pointer+keyboard/state, accessible semantics desde API real. OS release evidencia NVDA Windows y Orca Linux si accessibility backend lo soporta; si no, documentar gap/bloquear barra accesibilidad prometida, no simular test.
Cada vista tiene failure/cancel/offline state screenshot real durante implementación. QA no pruebas navegador para UI GPUI.
