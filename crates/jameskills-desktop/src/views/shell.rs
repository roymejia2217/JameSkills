use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement, Render,
    StatefulInteractiveElement as _, Styled, TestSupportExt as _, Window,
    component::{
        Icon,
        sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
        theme::ActiveTheme as _,
    },
    div, px,
};

use crate::{routes::Route, state::AppState};

/// Shell real de la aplicación: rail de navegación con primitivos del
/// catálogo, contenido por ruta con estados vacíos honestos y barra de
/// estado. Sin E/S en el render; el bridge con servicios llega en T008.b.
#[derive(Default)]
pub struct Shell {
    state: AppState,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
        }
    }

    fn menu(&self, cx: &mut Context<Self>) -> SidebarMenu {
        let weak = cx.entity().downgrade();
        Route::all()
            .into_iter()
            .fold(SidebarMenu::new(), |menu, route| {
                let active = self.state.route == route;
                let weak = weak.clone();
                menu.child(
                    SidebarMenuItem::new(route.label())
                        .icon(Icon::new(route.icon()))
                        .active(active)
                        .on_click(move |_, _, cx| {
                            if let Some(view) = weak.upgrade() {
                                view.update(cx, |this: &mut Self, cx| {
                                    this.state.navigate(route);
                                    cx.notify();
                                });
                            }
                        }),
                )
            })
    }

    fn empty_state(&self, route: Route) -> (&'static str, &'static str) {
        match route {
            Route::Library => (
                "Biblioteca",
                "Todavía no hay biblioteca conectada. Crear e importar skills llega con la biblioteca local.",
            ),
            Route::Checks => (
                "Proyectos y checks",
                "Sin proyecto vinculado. La guía de requisitos llega con el motor de políticas.",
            ),
            Route::Agents => (
                "Agentes",
                "Sin detección ejecutada. Los cinco adaptadores llegan con sus perfiles documentados.",
            ),
            Route::Sync => (
                "Sincronización",
                "Drive desconectado. La nube cifrada llega con la sincronización.",
            ),
            Route::Settings => (
                "Ajustes",
                "Los ajustes llegan con la configuración validada del bridge.",
            ),
        }
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let route = self.state.route;
        let (title, body) = self.empty_state(route);
        let sidebar = Sidebar::<SidebarMenu>::new("shell-nav")
            .w(px(200.))
            .header(div().px_3().pt_3().child("JameSkills"))
            .child(self.menu(cx));

        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(div().id("shell-nav").test_support().child(sidebar))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id("shell-content")
                            .test_support()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_8()
                            .child(
                                div()
                                    .id("shell-route-title")
                                    .aria_label(route.label())
                                    .text_size(px(24.))
                                    .font_weight(gpui_kit::FontWeight::BOLD)
                                    .child(title),
                            )
                            .child(div().text_color(cx.theme().muted_foreground).child(body)),
                    )
                    .child(
                        div()
                            .id("shell-status")
                            .test_support()
                            .h(px(28.))
                            .px_3()
                            .flex()
                            .items_center()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .text_color(cx.theme().muted_foreground)
                            .child("Drive: desconectado"),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{AppContext as _, TestAppContext, test::TestWindowExt as _};

    use crate::routes::Route;

    use super::Shell;

    /// ElementId del item de navegación según su posición en el menú.
    ///
    /// `SidebarMenu` genera los ids como `"{menu}-{ix}"` y la shell usa un
    /// solo menú, así que el item N vive en `"0-N"`. Si el Kit cambia el
    /// esquema, el test falla en voz alta en vez de pasar en silencio.
    fn nav_item_id(index: usize) -> String {
        format!("0-{index}")
    }

    #[gpui_kit::test]
    fn shell_navigates_between_routes(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| Shell::new())
            })
            .expect("open shell window")
        });
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            for index in 0..Route::all().len() {
                assert!(window.find(nav_item_id(index)).visible());
            }
            assert!(window.find("shell-content").visible());
            assert!(window.find("shell-status").visible());
            assert_eq!(view.read(cx).state.route, Route::Library);
            window.click(nav_item_id(2), cx);
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).state.route, Route::Agents);
            assert!(window.find("shell-content").visible());
            window.click(nav_item_id(2), cx);
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).state.route, Route::Agents);
        })
        .unwrap();
    }
}
