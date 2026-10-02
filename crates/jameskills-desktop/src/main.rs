use gpui_kit::{
    AppContext, Context, IntoElement, ParentElement, Render, Styled, Window, WindowOptions,
    assets::Assets, div,
};

struct ShellView;

impl Render for ShellView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("JameSkills")
    }
}

fn main() {
    gpui_kit::application().with_assets(Assets).run(|cx| {
        gpui_kit::init(cx);
        if gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| ShellView))
            .is_err()
        {
            eprintln!("JameSkills: no se pudo abrir la ventana; consulte doctor --json.");
            cx.quit();
        }
    });
}
