use gpui_kit::{App, AppContext as _, Entity, Window, WindowOptions, assets::Assets};
use jameskills_desktop::views::shell::Shell;

use crate::theme;

/// Arranca la ventana nativa de JameSkills con la shell real.
///
/// `gpui_kit::open_window` ya envuelve la vista en `base::Root`; no añadir
/// otro `Root` aquí (ver `open_window` en gpui-kit 0.7.0, `src/lib.rs`).
pub fn bootstrap_desktop() {
    gpui_kit::application().with_assets(Assets).run(|cx| {
        gpui_kit::init(cx);
        let options: WindowOptions = theme::window_options(cx);
        if gpui_kit::open_window(options, cx, render_shell).is_err() {
            eprintln!("JameSkills: no se pudo abrir la ventana; consulte doctor --json.");
            cx.quit();
        }
    });
}

fn render_shell(_window: &mut Window, cx: &mut App) -> Entity<Shell> {
    cx.new(|_| Shell::new())
}
