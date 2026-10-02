# Spec: skill-format

Proveedor del contrato Bundle, PortablePath, hashes y schema1. Consumers policy, library, adapters y sync. Referencias: REQUIREMENTS R03/I01/I03; CONTRACTS.

## Formato común mínimo

~~~
repository-foundation/
  SKILL.md
  jameskills.toml
  policies/repository.toml
  guidance/repository.toml
  references/{standards,environment}.md
  templates/{README.md,.gitignore,ci-rust.yml}
  assets/optional-brand.svg
~~~

SKILL.md es la superficie estándar portable. jameskills.toml enriquece validación/checks y no debe exigir un agente específico. Todo resource se referencia relativo a raíz. scripts/ importados son material inerte; la UI advierte que el agente podría ejecutarlos después de instalar y requiere review de trust; v1 suites oficiales no los necesitan.

Manifest TOML:
~~~
schema_version = 1
id = "f9c0199f-c4ce-4b04-85dd-ae12a7db292b"
slug = "repository-foundation"
display_name = "Repositorio seguro y estandarizado"
version = "1.0.0"
description = "Aplica y verifica estándares Git, seguridad, CI y releases."
license = "Apache-2.0"
minimum_app_version = "0.1.0"
policy_files = ["policies/repository.toml"]
guidance_files = ["guidance/repository.toml"]
tags = ["git", "ci", "security"]
~~~

Semver version parse estricto; UUID estable al editar/cambiar display, nuevo al clonar suite distinta. Manifest description coincide frontmatter? Debe tener mismo significado; v1 exigir igualdad exacta al publicar para evitar dos routing descriptions. slug = frontmatter.name = directorio export; internamente storage no depende de ese directorio. Version no sustituye RevisionId. metadata propia frontmatter jameskills-id/jameskills-version opcionales y verificadas si existen.

## Validación

- YAML frontmatter solo string scalars y metadata string map; sin tags custom, alias/anchors, merge keys; parser restringido, límite frontmatter 16KiB.
- Usar parser YAML mantenido seleccionado T010 y lock. `serde-saphyr` deserializa a DTO cerrado y configura budget de profundidad/eventos/documentos/bytes, aliases y anchors cero, duplicados/merge como error, tags custom rechazadas y snippets desactivados; no parsear fórmulas por regex.
- name ASCII regex ^[a-z0-9]+(?:-[a-z0-9]+)*$, 1–64; description Unicode longitud chars <=1024, no blanco; compatibility <=500 si presente.
- Schema futuro -> UnsupportedVersion; no abrir/escribir como schema1 ni descartar keys. Para schema1 manifest/policy unknown fields error salvo [extensions] metadata de strings inerte.
- Bundle máximo 20MiB, 2.000 archivos, cada texto máximo 2MiB, SKILL.md máximo 256KiB, Markdown render sin HTML activo. Advertir >500 líneas, sin rechazo si no excede bytes.
- Names/path: UTF-8 NFC obligatorio; separador `/`; componentes no vacíos, `.` ni `..`; rutas relativas solamente; rechazar backslash, prefijos de unidad, colon, caracteres no válidos en Win32 (`< > : " / \\ | ? *`), UNC, trailing dot/space, controles y NUL. Rechazar CON/PRN/AUX/NUL/COM1–9/LPT1–9 incluso con extensión; longitud portable <=240 bytes.
- Detectar colisiones tras case-fold Unicode; Windows no se resuelve sobrescribiendo. Symlink/hardlink/reparse point rechazados en import/export/target.
- Validar primero un inventario lógico independiente de IO: máximo 2.000 archivos, 20MiB descomprimidos, 2MiB por archivo de texto y 256KiB para SKILL.md; cada path es PortablePath, colisiones usan full case-fold Unicode vigente normalizado a NFC, y solo se aceptan entradas de archivos regulares. Directory archive entries se convierten a paths padres implícitos, nunca a archivos publicados.
- Assets SVG sin script/event/foreignObject/external href; render usar selección vetted icons, no SVG arbitrary dentro privileged GUI. Import conserva bytes en cuarentena.
- policies/guidance deben existir, referencias validadas y DAG sin ciclos.
- Safe import archive .jskill ZIP: comprobar central+entry real, tamaño/límites streaming, sin nested archives automáticos, ZIP symlink bit, rutas por PortablePath; extraer staging privado. Este paso requiere verificación nativa no-follow/reparse en cada OS.
- Trust review muestra scripts, remote links, security-sensitive templates. Nunca instala desde una carpeta temporal proporcionada por archivo externo sin valida completo.

## Canonical hashing

hash_bundle independiente orden ZIP, timestamp, compression y path OS. Entradas ordenadas por bytes UTF8 de PortablePath; incluir TODOS los archivos fuente, sin derived receipts.

Bytes digest:
1. ASCII "JAMESKILLS-BUNDLE-V1\0".
2. Para cada path: u32be path.len + path bytes; u64be file.len + raw bytes.
3. SHA256 hex lower. No convertir CRLF a LF automáticamente; cambio bytes = revisión distinta. Export conserva contenido.
4. Revision hash sobre "JAMESKILLS-REVISION-V1\0", UUID16, kind byte, bundlehash32 (cero en tombstone), version UTF8 length-prefixed, parents hashes32 ordenados, count u32; tombstone incluye observed_heads ordenados count+hashes. Version tombstone usa versión última content o "0.0.0" si no existe. created_at NO entra hash.
5. Content revision parents [] para primera; todo parent mismo skill; parent unknown permitido solo cuarentena hasta validar ancestralidad del snapshot.

## Import sin manifest

Una skill estándar con SKILL.md puede importarse. Asistente crea manifest UUID+version0.1.0 y sin policies; estado "Solo instrucciones". Nunca inventar políticas ni marcar una suite con enforcement inexistente. Import de suite completa verifica hash/revisión y permite nuevo UUID mediante Clone si autor desea divergencia. Dos mismo UUID mismo version con contenido diferente crean conflicto, no rewrite silencioso.

## Funciones/archivos/pruebas

domain/skill.rs parsing y validate_bundle; domain/ids.rs validated newtypes; infra/fs.rs archive safe and deterministic export.
Tests format.rs: valid golden, invalid slug, unknown schema, inconsistent descriptions, missing references, policy cycle, Unicode, CRLF hash differs, ZIP order hash equal.
fs_security.rs: ../../escape, absolute, device name, case collision, symlink, reparse point, bomb, oversized, duplicate entry; no bytes fuera staging.
Acceptance: roundtrip import/export mantiene hash; suite estándar mantiene SKILL.md reconocible; futuro schema protegido. CLI validate emite path/line/code sin crash.
