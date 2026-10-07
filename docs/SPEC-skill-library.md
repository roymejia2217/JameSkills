# Spec: skill-library

CRUD/versiones/export/import offline con dominio skill-format. SQLite/blobs relaciones ARCHITECTURE y CONTRACTS. Sin red necesaria, performance REQUIREMENTS.

## Casos de uso

Crear: UUID nuevo, slug validado, SKILL.md frontmatter válido y manifest por default; draft con plantilla textos editables. Guardar draft tolera errores de contenido para no perder trabajo; Publish/Install/Export-suite exige válido. GUI diferencia Guardar borrador/Publicar revisión. Autosave draft debounce500ms nunca publica.

Editar: load head única; si >1 abrir vista conflicto antes permitir Publish sobre única asumida. Editor guarda generation y expected base. Publish valida cambios y versión; no-op content hash igual devuelve existing revision sin duplicar. Misma semantic version diferente contenido warning+confirmation o exige bump para publicar; v1 exigir bump cuando contenido cambia después de publicación, regla semver de app propia. Merge conflict nueva revisión con dos parents y bump explícito.

Import: path directory/.jskill -> private staging -> archive safety -> validate -> secret scan si herramienta disponible (unavailable/unknown/findings/blocked se muestran redacted; importar en cuarentena permitido, nunca eleva trust) -> review capabilities/scripts/links -> crear revision. Gitleaks 8.30.1 solo recibe bytes validados en staging privado, con ejecutable y fingerprint aprobados; limpia staging/config al terminar. `NoFindings` no equivale a Reviewed. Existing UUID/hash idempotente; same UUID different content parents según manifest metadata? Archivo .jskill individual no conserva DAG salvo export metadata controlada: al import no conoce parent legítimo -> crear root concurrente, no fabricar descent. Export de suite preserva contenido, identidad+version; backup completo preserva DAG.
Trust reviewed = usuario examinó riesgos y checks; la app no asegura que instrucciones sean benignas. Install exige reviewed y checks requeridos pre-install.
Import plain skill sin manifest -> draft manifested "solo instrucciones", publicación tras review.

Versionar: content immutable blob y revision record. Historial lista change summary/hash/version sin timestamps para ancestry. Restore revision vieja crea NUEVA revision descendiente de heads actuales con contenido viejo; no mover head atrás ocultando historia. Blob anterior disponible offline si retenido; no garbage collect revision referenciada.
Clonar: UUID nuevo+slug nombre, frontmatter/manifest metadata coherente; no llevar installations/settings.
Delete: tombstone revision descendiente todas heads observadas, delete no borra bytes inmediato; UI Papelera con restore. Concurrent edit no observado se conserva conflicto según SPEC-cloud-sync.
Local purge (fuera v1 automático): eliminar permanentemente tras user confirmation y backup; si metadata sync referenciada no purge silencioso.

Search: query trimmed+case-fold; SQL parametrizado, FTS5 opcional solo si justificado. Cursor estable por (display_name normalized,id), page50; filtros tags/capabilities/state/conflict, no enumeración de blobs para lista.
List DTO metadata+statuses de evidence freshness; nunca descomprimir todos archivos render.
Editor navegación warning draft dirty/guardado, save errors persistent; watcher con debounce external changes genera conflicto expected-head, no silent reload dirty.
Export .jskill: selección revisión específica, reviewed state local no export como garantía, zip safe, excluir local paths/metadata; elegir output path overwrite con diff guard y permisos. Local encrypted backup a archivo distinto .jskills-backup.
Hacer templates de suite no escribe repo al visualizar; aplicar en proyecto va PolicyService plan/diff.

## Seguridad/persistencia

SQLite y blobs permisos usuario; no cifrado completo local v1. Mostrar esa propiedad al activar sync: cloud encrypted, local source legible; recomendar cifrado disco. Lock app crypto borra key pero no false promise oculta fuente al atacante local. Secrets detectados no se suben a cloud; reporta path+tipo redacted y permite remover primero, no ignore secrets default.
Los receipts/drafts/evidence sessions son locale y no siguen al otro OS.
Staging/journal tolera power loss con failpoints, never SQLite references missing published blob. Backup snapshot transaction captures revisions+heads consistent; writes blobs antes+read verify hashes al backup.

## Acceptance y tests

library.rs: create valid, invalid draft preserved, publish stale head Conflict, no-op idempotent, semver bump, imported UUID conflict, clone new ID, delete+restore keeps original.
storage.rs real temporary SQLite migrations+foreignkeys/transaction; SQL injection strings no daño; backup captura generation consistente.
Tests import-export hash unchanged, read-only permission error mantiene draft, no sync needed.
Vertical UI: crear -> guardar -> publicar -> list -> search -> export -> importar copia equivalente; CLI mismos servicios sin GPU.
