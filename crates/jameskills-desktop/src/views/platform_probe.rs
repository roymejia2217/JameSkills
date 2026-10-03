use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render,
    Styled, TestSupportExt as _, Window,
    component::{
        Icon, IconName,
        button::{Button, ButtonVariants as _},
        input::{Input, InputState},
        theme::ActiveTheme as _,
    },
    div, px,
};

pub struct PlatformProbe {
    search: Entity<InputState>,
    interaction_checked: bool,
}

impl PlatformProbe {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            search: cx.new(|cx| InputState::new(window, cx).placeholder("Busca una skill")),
            interaction_checked: false,
        }
    }
}
impl Render for PlatformProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let background = cx.theme().background;
        let surface = cx.theme().popover;
        let foreground = cx.theme().foreground;
        let muted = cx.theme().muted_foreground;
        let interaction_checked = self.interaction_checked;

        div()
            .size_full()
            .flex()
            .bg(background)
            .text_color(foreground)
            .child(
                div()
                    .w(px(248.))
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .p_6()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .child(div().text_size(px(20.)).font_weight(gpui_kit::FontWeight::BOLD).child("JameSkills"))                    .child(div().text_color(muted).child("Suite local de skills")),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .p_8()
                    .child(
                        div()
                            .text_size(px(24.))
                            .font_weight(gpui_kit::FontWeight::BOLD)
                            .child("JameSkills se inició correctamente"),
                    )
                    .child(div().text_color(muted).child(
                        "Esta vista comprueba la ventana nativa y los controles GPUI Kit. La biblioteca se conectará en los siguientes incrementos.",
                    ))
                    .child(
                        div()
                            .w(px(560.))
                            .p_6()
                            .flex()
                            .flex_col()                            .gap_4()
                            .bg(surface)
                            .rounded_lg()
                            .child(div().child("Buscar skills"))
                            .child(
                                div()
                                    .id("probe-search")
                                    .test_support()
                                    .child(Input::new(&self.search).aria_label("Busca una skill")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(Icon::new(IconName::Search))
                                    .child(div().text_color(muted).child("Iconos de GPUI Kit activos")),
                            )
                            .child(
                                div()
                                    .id("probe-action")
                                    .test_support()
                                    .child(
                                        Button::new("probe-action-button")
                                            .primary()                                            .label("Validar interacción")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.interaction_checked = true;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .id("probe-status")
                                    .test_support()
                                    .text_color(muted)
                                    .child(if interaction_checked {
                                        "La interacción nativa respondió."
                                    } else {
                                        "Sin operaciones de biblioteca conectadas todavía."
                                    }),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{AppContext as _, TestAppContext, test::TestWindowExt as _};

    use super::PlatformProbe;

    #[gpui_kit::test]
    fn platform_probe_renders_the_search_control(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| PlatformProbe::new(window, cx))
            })
            .expect("open probe window")
        });
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert!(window.find("probe-search").visible());
            assert!(window.find("probe-action").visible());
            window.click("probe-action", cx);
            window.draw(cx).clear(cx);
            assert!(view.read(cx).interaction_checked);
        })
        .unwrap();
    }
}
