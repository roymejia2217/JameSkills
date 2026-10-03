# JameSkills

Suite nativa de skills portables con biblioteca offline, cinco agentes y backups cifrados.

## Table of Contents

- [Background](#background)
- [Install](#install)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## Background

JameSkills es una aplicación de escritorio nativa (Rust + GPUI Kit) para
Windows y Linux. Las skills son suites portables con instrucciones,
políticas verificables, guía dinámica del entorno y assets. La biblioteca
funciona offline, instala en cinco agentes de código y sincroniza backups
cifrados mediante Google Drive.

Este repositorio se desarrolla por tareas ejecutables (`tasks/todo.md`)
sobre un dossier de especificaciones. Para implementar, empezar por
[AGENTS.md](AGENTS.md), el [handoff](tasks/HANDOFF-LUNA.md), el
[plan](tasks/plan.md) y las [tareas](tasks/todo.md):

- [Requisitos y aceptación](docs/REQUIREMENTS.md)
- [Mapa de capacidades](CAPABILITY-MAP.md)
- [Arquitectura y estructura de archivos](docs/ARCHITECTURE.md)
- [Contratos, funciones y blueprints Rust/CLI](docs/CONTRACTS.md)
- [Formato común](docs/SPEC-skill-format.md)
- [Políticas y guía dinámica](docs/SPEC-policy-engine.md)
- [Agentes e instalación](docs/SPEC-agent-adapters.md)
- [Biblioteca y revisiones](docs/SPEC-skill-library.md)
- [Drive, cifrado, conflictos y restauración](docs/SPEC-cloud-sync.md)
- [Aplicación nativa](docs/SPEC-desktop-app.md)
- [Diseño de GUI y cada recorrido](docs/GUI.md)
- [Seguridad por frontera](docs/SECURITY.md)
- [TDD, XP y pruebas](docs/TESTING.md)
- [Puesta en marcha, Git, CI y distribución](docs/OPERATIONS.md)
- [Fuentes oficiales y discrepancias resueltas](docs/SOURCES.md)
- [Trazabilidad y auditoría documental](docs/TRACEABILITY.md)

La [suite de referencia](docs/examples/repository-foundation/SKILL.md) es
un fixture del esquema propuesto, para implementar y validar en TDD. Su
plantilla CI tiene valores por resolver y debe fallar validación de
producción hasta completarlos; no es un workflow activo.

GPUI Kit 0.7.0 fijado desde release/registry, OpenCode/Codex/Pi con
contratos documentados. Unsupported/Unknown siempre visibles. Una skill
puede guiar pasos del usuario, pero protección de main y gates CI dependen
de reglas y permisos reales del proveedor. Tokens y passphrase no forman
parte de la skill ni del backup.

## Install

Rust 1.95.0 mediante [rustup](https://rustup.rs); `rust-toolchain.toml`
fija el toolchain automáticamente. Windows requiere Build Tools de Visual
Studio 2022 con C++, Windows SDK y CMake. Linux requiere las bibliotecas
de desarrollo `pkg-config`, `x11`, `xcb`, `xkbcommon` y `fontconfig`.

```sh
cargo build --locked
```

## Usage

Abrir la aplicación de escritorio:

```sh
cargo run -p jameskills-desktop --locked
```

La ventana muestra el rail de navegación (Biblioteca, Proyectos y checks,
Agentes, Sincronización, Ajustes) y el contenido de la ruta activa con
sus estados vacíos hasta conectar cada capacidad.

### CLI

Diagnóstico del entorno en JSON:

```sh
cargo run -p jameskills-cli --locked -- doctor --json
```

## Contributing

Se aceptan pull requests desde ramas del propio repositorio con la
plantilla de seis secciones. Cada cambio pasa commitlint (mensajes
convencionales con cuerpo), la gobernanza de PR y la CI mínima antes del
squash-merge a `main`, que está protegida sin bypass. Las instrucciones
operativas para agentes viven en [AGENTS.md](AGENTS.md); las preguntas
van en los issues del repositorio.

## License

Apache-2.0. Ver [LICENSE](LICENSE). Copyright 2026 roymejia2217.
