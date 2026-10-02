# Trazabilidad y auditoría del dossier

Estado: revisión documental del plan, 2026-10-02. Ninguna aplicación compilada o probada aquí. Todas las tareas de implementación permanecen pendientes.

## Requisitos → especificaciones → ejecución

| Requisito | Documento normativo | Tareas principales | Evidencia futura requerida |
|---|---|---|---|
| R01 nativa Windows/Linux | SPEC-desktop-app, GUI, OPERATIONS | T001–T009,T076,T078–T080 | Build+GUI+paquetes nativos ambos OS |
| R02 biblioteca offline | SPEC-skill-library, ARCHITECTURE | T037–T047 | SQLite real+CRUD/import/export/UI |
| R03 suite portable | SPEC-skill-format, CONTRACTS | T010–T015,T041–T043 | Golden roundtrip/hash y archive adversarial |
| R04 cinco agentes | SPEC-agent-adapters | T029–T036,T050 | Probes/version/source fixture y user install cinco; project cuatro |
| R05 instalación segura | SPEC-agent-adapters, SECURITY | T035–T036,T073–T075 | Failpoints journal/rename/restart, owner hashes |
| R06 políticas estándares | SPEC-policy-engine | T016–T028,T048 | Checks+authority+strict CLI+remote SHA/rulesets |
| R07 guía dinámica | SPEC-policy-engine, GUI | T025–T027,T049,T054 | Ramas OS/tool/permisos, recheck sin manual Pass |
| R08 Drive | SPEC-cloud-sync | T054–T061,T065–T067 | OAuth PKCE +fake HTTP y account live opt-in |
| R09 cifrado+restore | SPEC-cloud-sync, SECURITY | T051–T053,T063–T064 | Header176 tamper/KDF limits, perfil nuevo otroOS |
| R10 conflictos | SPEC-cloud-sync | T044,T058–T063,T067 | DAG causal/delete-vs-edit y dos dispositivos |
| R11 TDD+seguridad | TESTING, SECURITY, OPERATIONS | Todas; T074,T077 | RED/GREEN significativo, corpus seguridad, CI |
| R12 distribución | OPERATIONS | T078–T084 | Install/uninstall/upgrade/checksum/licencias/firma |

Los números no implican orden numérico: el DAG explícito en tasks/todo.md decide. T015 depende de SQLite; T028 espera import/lookup UUID; T035 espera persistencia real. Bloqueos de cuenta/OS solo impiden dependientes, no trabajo independiente.

## Coherencia revisada

- Seis módulos y seis especificaciones presentes.
- Contratos en CONTRACTS fijan tipos/funciones/CLI; el plan introduce interfaces incrementalmente con proveedores reales.
- Enlaces Markdown locales resuelven; anchors explícitos de tareas.
- IDs principales únicos y dependencias acíclicas; orden topológico incluye todos.
- Suites de ejemplo TOML parseables; diez requisitos, diez planes de guía, dependencias e IDs válidos.
- Ejemplo SKILL.md name/description/UUID/version coinciden manifest y rutas de recursos existen.
- Tamaño/offset/AAD header revisados; material de sesión incluye master+wrapping key para wrap fresh; contraseña no cacheada.
- Autoridad exigida y observada se distinguen: mensaje válido no prueba hook, workflow no prueba regla host.
- Rutas actuales CLI verificadas fuentes: Antigravity usa vendor plugin, Grok Build perfil propio, Codex .agents actual.
- Fuente GPUI Kit resuelve discrepancy documentación0.6 vs registro0.7, no usar release retirada.
- Plantilla CI de ejemplo falla explícitamente hasta completar Actions/toolchain desde fuente; no workflow operativo.
- Estado inicial checklist no marca tareas aplicación completadas.

## Límites de esta revisión

No compila blueprint Rust ni prueba MSRV/resolver, GPU, accesibilidad OS, installed CLIs, Google API o firma. Eso tiene tasks+tests+gates explícitos. Ninguna fixture o documento se presenta como evidencia de esos sistemas. Una ejecución futura debe sustituir observación provisional por evidence real sin debilitar criterio.

## Cambios de contrato durante implementación

Antes de cambiar un tipo/port/flag: editar CONTRACTS + spec proveedora + tarea y tests afectados; registrar motivo/source. Añadir subdivisión con dependencies si supera presupuesto de5 archivos. No usar decisión nueva sin documento ni status de éxito simulado. Si hardware/permisos impiden aceptación, conservar pendiente y guía exacta.

## Resultado final de auditoría automática

84 tareas principales, 47 subtareas explícitas, 28 checkpoints y 247 dependencias. DAG/orden, enlaces, presupuestos declarados y ejemplos TOML revisados sin errores. Todas las tareas de aplicación pendientes.
