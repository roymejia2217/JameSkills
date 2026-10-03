use gpui_kit::{App, AppContext as _, Entity, Window, WindowOptions, assets::Assets};

use crate::{platform_probe::PlatformProbe, theme};

/// Arranca la ventana nativa de JameSkills con la sonda de plataforma.
///
/// `gpui_kit::open_window` ya envuelve la vista en `base::Root`; no añadir
/// otro `Root` aquí (ver `open_window` en gpui-kit 0.7.0, `src/lib.rs`).
pub fn bootstrap_desktop() {
    gpui_kit::application().with_assets(Assets).run(|cx| {
        gpui_kit::init(cx);
        let options: WindowOptions = theme::window_options(cx);
        if gpui_kit::open_window(options, cx, render_platform_probe).is_err() {
            eprintln!("JameSkills: no se pudo abrir la ventana; consulte doctor --json.");
            cx.quit();
        }
    });
}

fn render_platform_probe(window: &mut Window, cx: &mut App) -> Entity<PlatformProbe> {
    cx.new(|cx| PlatformProbe::new(window, cx))
}
