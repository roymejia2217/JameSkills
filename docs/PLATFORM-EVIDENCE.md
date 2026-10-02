# Evidencia de plataforma — bootstrap T001

Fecha de comprobación: 2026-10-02 UTC. Este documento registra lo observado en
el entorno de implementación y separa esos datos de los requisitos publicados
para una máquina Windows. Un requisito documentado no equivale a un smoke test.

## Toolchain y GPUI Kit

| Elemento | Evidencia | Estado |
|---|---|---|
| OS de ejecución | Debian GNU/Linux 13 (trixie), contenedor Linux x86_64 | observado |
| Target Rust host | `x86_64-unknown-linux-gnu` | observado |
| Rust | `rustc 1.92.0 (ded5c06cf 2025-12-08)` | instalado y ejecutado |
| Cargo | `cargo 1.92.0 (344c4567c 2025-10-21)` | instalado y ejecutado |
| Git | `git version 2.52.0` | observado |
| GPUI Kit | `gpui-kit 0.7.0`, Apache-2.0, edición 2024 | descargado con `cargo info`; no yanked en índice crates.io local |
| GPUI facade | dependencia `gpui = "=0.3.7"` en Cargo.toml normalizado del crate | inspeccionada |
| MSRV publicado del Kit | crates.io/Cargo informa `rust-version: unknown` | no declarado; la compilación resolverá compatibilidad efectiva |
| Hash del crate | `8edb2a8eafdb6e65ad1a80625347b93f8e54cbf3fac82a584e4924cda674d5c2` | local; Cargo.lock fijará la entrada antes de compilar el workspace |

El índice consultado reporta `gpui-kit 0.7.0 yanked=false` y `0.6.5
yanked=true`. El tag upstream `v0.7.0` resuelve a
`0c830f4d257e69fdd17200650533ab4ca9a40cc0`. En su `src/lib.rs` se verificaron
`application`, `init`, `open_window`, los reexports de GPUI/assets y la
creación de `base::Root` dentro de `open_window`.

## Linux observado

| Dato | Resultado |
|---|---|
| Kernel | Linux x86_64; entorno contenedor, no estación física identificada |
| Display | `DISPLAY`, `WAYLAND_DISPLAY` y `XDG_SESSION_TYPE` vacíos |
| GPU | `/dev/dri` no existe en el contenedor; renderer no comprobable |
| Compilador nativo | GCC/G++ 14.2 instalados |
| Dependencias detectadas | `libssl-dev`, `libzstd-dev`, `libvulkan1` instalados; `pkg-config`, CMake, fontconfig/Wayland/X11 dev y driver/render no verificados como disponibles |
| Compilación de ventana | no ejecutada; no hay sesión gráfica/GPU |

El contenedor sirve para crates de dominio y CLI. No demuestra que la aplicación
nativa abra ni que el renderer Vulkan funcione. Debian 13 no se convertirá en
Ubuntu 24.04 por inferencia; T004 debe imprimir instrucciones específicas y no
ejecutar `sudo`.

## Windows documentado, ejecución pendiente

| Dato | Resultado |
|---|---|
| Sistema disponible para prueba | no hay runner ni host Windows en esta ejecución |
| Target que debe validar CI/QA | `x86_64-pc-windows-msvc` |
| Display/GPU | no observados; requieren sesión/runner Windows con renderer apto |
| Prerrequisitos publicados | Windows 10+, Visual Studio 2022 Build Tools con Desktop C++, Windows SDK, CMake en PATH y toolchain Rust MSVC |
| Compilación/smoke nativo | pendiente de runner Windows; no afirmado |

Fuentes Windows y Linux upstream: [instalación GPUI Kit](https://gpui-kit.com/docs/installation)
y [documentación de release 0.7.0](https://github.com/longbridge/gpui-kit/releases/tag/v0.7.0).
Los requisitos son una guía para preparar el runner, no evidencia de instalación.

## Resultado T001

Pin exacto y toolchain quedan registrados. La ausencia de display/GPU en Linux y
de un host Windows es una limitación observada y explícita. Esos recorridos
nativos deben quedar pendientes en las tareas de spike/QA correspondientes;
ningún check del producto podrá presentar esas plataformas como `Pass` por esta
tabla documental.
