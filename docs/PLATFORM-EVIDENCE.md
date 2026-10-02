# Evidencia de plataforma — bootstrap T001

Fecha de comprobación: 2026-10-02 UTC. Este documento registra lo observado en
el entorno de implementación y separa esos datos de los requisitos publicados
para una máquina Windows. Un requisito documentado no equivale a un smoke test.

## Toolchain y GPUI Kit

| Elemento | Evidencia | Estado |
|---|---|---|
| OS de ejecución | Debian GNU/Linux 13 (trixie), contenedor Linux x86_64 | observado |
| Target Rust host | `x86_64-unknown-linux-gnu` | observado |
| Rust fijado | `rustc 1.95.0 (59807616e 2026-04-14)` | instalado y ejecutado |
| Cargo fijado | Cargo 1.95.0 | instalado y ejecutado |
| Git | `git version 2.52.0` | observado |
| GPUI Kit | `gpui-kit 0.7.0`, Apache-2.0, edición 2024 | descargado con `cargo info`; no yanked en índice crates.io local |
| GPUI facade | dependencia `gpui = "=0.3.7"` en Cargo.toml normalizado del crate | inspeccionada |
| MSRV publicado del Kit | crates.io/Cargo informa `rust-version: unknown` | no declarado; fuentes GPUI requieren como mínimo las APIs estables hasta 1.95 |
| Hash del crate | `8edb2a8eafdb6e65ad1a80625347b93f8e54cbf3fac82a584e4924cda674d5c2` | local; Cargo.lock fijará la entrada antes de compilar el workspace |

El índice consultado reporta `gpui-kit 0.7.0 yanked=false` y `0.6.5
yanked=true`. El tag upstream `v0.7.0` resuelve a
`0c830f4d257e69fdd17200650533ab4ca9a40cc0`. En su `src/lib.rs` se verificaron
`application`, `init`, `open_window`, los reexports de GPUI/assets y la
creación de `base::Root` dentro de `open_window`.

La primera resolución con Rust1.92.0 falló en `gpui-pre-util 0.3.7` porque
`slice::as_array` todavía no era estable. Rust1.93.0 pasó ese crate, pero
`gpui-pre 0.3.7` usa `std::hint::cold_path`, que el compilador rechazó en 1.93 y
1.94. El código fuente oficial de Rust fija `slice::as_array` desde 1.93.0 y
`cold_path` desde 1.95.0. Por ello `rust-toolchain.toml` y `rust-version`
subieron conjuntamente a 1.95.0. Comando GREEN:
`RUST_FONTCONFIG_DLOPEN=1 cargo +1.95.0 check -p jameskills-desktop --locked`;
terminó correctamente. El override solo habilita el dlopen que el crate upstream
`yeslogic-fontconfig-sys` documenta para compilar sin `fontconfig.pc`; la
biblioteca runtime sí está instalada. Compilación normal requiere paquete de
desarrollo según `docs/OPERATIONS.md`.

## Linux observado

| Dato | Resultado |
|---|---|
| Kernel | Linux x86_64; entorno contenedor, no estación física identificada |
| Display | `DISPLAY`, `WAYLAND_DISPLAY` y `XDG_SESSION_TYPE` vacíos |
| GPU | `/dev/dri` no existe en el contenedor; renderer no comprobable |
| Compilador nativo | GCC/G++ 14.2 instalados |
| Dependencias detectadas | runtime `fontconfig`, `freetype`, Wayland y XKB presentes; faltan development libs `xcb`, `xkbcommon` y `xkbcommon-x11` para enlazar. `RUST_FONTCONFIG_DLOPEN=1` evita necesitar `fontconfig.pc` durante check |
| Compilación | `cargo check -p jameskills-desktop --locked` pasó con `RUST_FONTCONFIG_DLOPEN=1`; `cargo build` llegó al linker y falló por `-lxcb`, `-lxkbcommon`, `-lxkbcommon-x11` ausentes |
| Ventana visible | no probada; no hay sesión gráfica ni `/dev/dri` |

El contenedor sirve para crates de dominio y CLI. No demuestra que la aplicación
nativa abra ni que el renderer Vulkan funcione. Debian 13 no se convertirá en
Ubuntu 24.04 por inferencia; el doctor ofrece instrucciones específicas según
el sistema detectado y nunca ejecuta `sudo`.

## Windows: host check and MSVC validation 2026-10-02

| Dato | Resultado |
|---|---|
| Host observado | `DESKTOP-6PK09A2`, Windows 10 IoT Enterprise LTSC `10.0.19044`, x64 |
| Herramientas de ejecución | Git `2.55.0.windows.5`; Windows PowerShell `5.1.19041.7725`; `pwsh` no está instalado |
| Target que debe validar CI/QA | `x86_64-pc-windows-msvc` |
| Rust MSVC / target | Rust 1.95.0; `x86_64-pc-windows-msvc`; rustfmt 1.9.0; Clippy 0.1.95 |
| CMake / Visual Studio 2022 C++ | CMake 4.4.4; VS 2022 Build Tools 17.x con Desktop C++ |
| Windows SDK | 10.0.26100.0 instalado; headers detectados |
| Display/GPU | `unknown`; requieren smoke nativo con renderer apto |
| Prerrequisitos publicados | Windows 10+, Visual Studio 2022 Build Tools con Desktop C++, Windows SDK, CMake en PATH y toolchain Rust MSVC |
| Compilación | `cargo build -p jameskills-desktop --target x86_64-pc-windows-msvc --locked` pasó en el host |
| Ventana/renderer | smoke de ventana no ejecutado; display y GPU siguen `unknown` |

Instalación ejecutada con autorización del usuario: instalador oficial de VS2022
Build Tools Authenticode válido, CMake 4.4.4 MSI Authenticode válido de Kitware,
y rustup oficial cuyo SHA-256 coincidió con el manifiesto oficial. Rust se
instaló en perfil de usuario y VS/CMake a nivel de sistema con elevación; no hay
UAC pendiente.

Fuentes Windows y Linux upstream: [instalación GPUI Kit](https://gpui-kit.com/docs/installation)
y [documentación de release 0.7.0](https://github.com/longbridge/gpui-kit/releases/tag/v0.7.0).
Los requisitos son una guía para preparar el runner, no evidencia de instalación.

## Resultado T001

Pin exacto y toolchain quedan registrados. La ausencia de display/GPU y libs de
desarrollo de enlace en Linux, así como el smoke de ventana pendiente en Windows,
son limitaciones observadas y explícitas. Esos recorridos nativos deben quedar
pendientes en las tareas de spike/QA correspondientes;
ningún check del producto podrá presentar esas plataformas como `Pass` por esta
tabla documental.

## Diagnóstico T004 en Debian 13

`setup-linux.sh --check` es de solo lectura y emite JSON con estados separados
para toolchain, módulos pkg-config, display y device GPU. `--print-install-plan`
solo imprime los paquetes propuestos y sus fuentes; no ejecuta apt ni sudo. El
plan Debian/Ubuntu enumera los development packages que el linker pidió
(`xcb`, `xkbcommon`, `xkbcommon-x11`) además de fontconfig/freetype/Wayland.

`setup-windows.ps1` comprueba Rust MSVC, CMake, Visual Studio C++ y Windows SDK
con salida JSON; el plan no instala automáticamente. PowerShell 7 (`pwsh`) no
está instalado, así que las verificaciones usaron Windows PowerShell 5.1. Tras
la instalación, `scripts/setup-windows.ps1 -Check` terminó con código 0:
Rust/toolchain/target MSVC, CMake, VS C++ y SDK `pass`; display/GPU `unknown`.

Verificación ejecutada: `bash -n scripts/setup-linux.sh` pasó; `--check` emitió
JSON y código1 por CMake/Clang y módulos pkg-config de desarrollo ausentes,
marcando Vulkan loader `pass` y display/GPU `unknown`; `--print-install-plan`
emitió JSON apt con 14 paquetes sin ejecutar comandos privilegiados. PowerShell
7.6.6 oficial (SHA-256 validado contra `hashes.sha256`) parseó
`setup-windows.ps1`; `-PrintInstallPlan` emitió JSON y `-Check` en Linux devolvió
`unsupported`. En Windows, PowerShell 5.1.19041.7725 ejecutó el check con todos
los prerrequisitos `pass`. `cargo test -p jameskills-infra --locked` pasó 10/10;
la suite `cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli
--locked` pasó 53/53. Una regresión encontró que LocalAppData es a la vez base de
datos y de caché en Windows; se separaron como `JameSkills/Data` y
`JameSkills/Cache`, conservando la validación contra solapamiento. El build
Windows de desktop terminó correctamente tras compilar dependencias GPUI desde
cero (9m13s). No se afirma smoke de ventana ni renderer; display/GPU siguen
`unknown`.
