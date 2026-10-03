use gpui_kit::{App, WindowBounds, WindowOptions, px, size};

/// Opciones de la ventana de la sonda de plataforma.
///
/// Tamaño inicial 1280x800 y mínimo 1000x680 según SPEC-desktop-app/GUI.
/// Los colores y tipografías vienen del tema semántico del Kit:
/// `gpui_kit::init` ya ejecuta `theme::init`, así que este módulo no
/// reinicia el tema ni crea raíces (`open_window` incorpora `base::Root`).
pub fn window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::centered(size(px(1280.), px(800.)), cx)),
        window_min_size: Some(size(px(1000.), px(680.))),
        ..WindowOptions::default()
    }
}
