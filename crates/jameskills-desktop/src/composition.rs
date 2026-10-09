use gpui_kit::{AppContext as _, WindowOptions, assets::Assets};
use jameskills_desktop::services::DesktopServices;
use jameskills_desktop::views::shell::Shell;

use crate::theme;
use std::sync::Arc;

/// Arranca la ventana nativa de JameSkills con la shell real.
///
/// `gpui_kit::open_window` ya envuelve la vista en `base::Root`; no añadir
/// otro `Root` aquí (ver `open_window` en gpui-kit 0.7.0, `src/lib.rs`).
pub fn bootstrap_desktop() {
    let services = match DesktopServices::build() {
        Ok(services) => Arc::new(services),
        Err(_) => {
            eprintln!(
                "JameSkills: no se pudo iniciar la biblioteca local; consulte doctor --json."
            );
            return;
        }
    };
    gpui_kit::application().with_assets(Assets).run(move |cx| {
        gpui_kit::init(cx);
        let options: WindowOptions = theme::window_options(cx);
        let window_services = services.clone();
        if gpui_kit::open_window(options, cx, move |window, cx| {
            cx.new(|cx| Shell::new(window, cx, window_services.clone()))
        })
        .is_err()
        {
            eprintln!("JameSkills: no se pudo abrir la ventana; consulte doctor --json.");
            cx.quit();
        }
    });
}
