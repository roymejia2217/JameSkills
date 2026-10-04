---
name: repository-foundation
description: Aplica y verifica estándares Git, seguridad, CI y releases.
compatibility: Requiere Git y herramientas del perfil de proyecto. Las reglas remotas requieren acceso al proveedor.
metadata:
  jameskills-id: f9c0199f-c4ce-4b04-85dd-ae12a7db292b
  jameskills-version: 1.0.0
---

# Repositorio seguro y estandarizado

Usa esta suite al iniciar un repositorio, hacer cambios, configurar CI o preparar una release.

1. Lee [estándares](references/standards.md) y [requisitos del entorno](references/environment.md).
2. Detecta SO, arquitectura, stack, rama y proveedor. Comprueba herramientas; si faltan, guía al usuario con la rama de guidance aplicable y vuelve a verificar.
3. Inspecciona el trabajo existente; conserva archivos ajenos. Propón un diff concreto antes de aplicar configuración o permisos.
4. Trabaja en una rama para cambios, usa Conventional Commits y PR hacia main. No publiques secretos.
5. Verifica README, gitignore y escaneo de secretos con las herramientas registradas de JameSkills.
6. Usa RED/GREEN/REFACTOR para nuevos comportamientos; configura CI del stack y conserva sus checks obligatorios.
7. Para main, PR, CI y releases, comprueba reglas efectivas y evidencia del SHA actual en el host. Si faltan permisos, pide al usuario completar ese requisito y revalida.
8. Publica una release solo con versionado, changelog, artefactos y checks previstos, y autorización de publicación.

[Políticas tipadas](policies/repository.toml) definen los gates verificables. [Guía](guidance/repository.toml) define pasos adaptables. Las instrucciones solas no acreditan que exista protección remota. Informa Pass, Fail, Blocked, Unknown o Unsupported con evidencia y la acción necesaria; nunca conviertas "ya lo hice" en un check aprobado sin comprobar.

Plantillas: [README](templates/README.md), [gitignore](templates/.gitignore), [CI Rust](templates/ci-rust.yml). Adáptalas al proyecto; no sustituyas contenido existente automáticamente.
