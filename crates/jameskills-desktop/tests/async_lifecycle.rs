use gpui_kit::{AppContext as _, TestAppContext, test::TestWindowExt as _};
use jameskills_desktop::{
    bridge::{CommandEnvelope, UiBridge, UiCommand, UiEvent, apply_event, dispatch_command},
    routes::Route,
    state::AppState,
    views::shell::Shell,
};

/// Ciclo de vida completo: despacho en el hilo de UI, cómputo del servicio
/// en el executor de fondo y aplicación tardía sin perder la ruta nueva.
#[gpui_kit::test]
async fn bridge_late_completion_keeps_newer_route(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (window, _) = cx.update(|cx| {
        gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Shell::new())
        })
        .expect("open shell window")
    });

    // La "respuesta del servicio" para la navegación a Agentes se calcula
    // fuera del hilo de UI, con la generación que vio al despacharse.
    let completion = cx.spawn(|_| async move {
        UiEvent::NavigationCommitted {
            request_id: 1,
            route: Route::Agents,
            generation: 1,
        }
    });

    // La persona usuaria pasa a Ajustes antes de que aterrice la carga.
    cx.update_window(window, |_, window, cx| {
        window.draw(cx).clear(cx);
        window.click("0-4", cx);
        window.draw(cx).clear(cx);
        assert_eq!(window.find("shell-route-title").label(), Some("Ajustes"));
        assert!(window.find("shell-content").visible());
        assert!(window.find("shell-status").visible());
    })
    .unwrap();

    // Misma historia visible, reproducida por el reductor del bridge.
    let mut state = AppState::new();
    let mut bridge = UiBridge::new();
    let first = bridge.next_request_id();
    let generation = state.route_generation;
    dispatch_command(
        &mut state,
        &mut bridge,
        CommandEnvelope {
            request_id: first,
            route_generation: generation,
            command: UiCommand::Navigate(Route::Agents),
        },
    );
    let second = bridge.next_request_id();
    let generation = state.route_generation;
    dispatch_command(
        &mut state,
        &mut bridge,
        CommandEnvelope {
            request_id: second,
            route_generation: generation,
            command: UiCommand::Navigate(Route::Settings),
        },
    );
    assert_eq!(state.route, Route::Settings);

    apply_event(&mut state, &mut bridge, completion.await);
    assert_eq!(state.route, Route::Settings);
    assert!(bridge.activity().any(|event| matches!(
        event,
        UiEvent::NavigationCommitted {
            route: Route::Agents,
            ..
        }
    )));
}

/// El despacho duplicado o con generación vieja se rechaza sin tocar ruta.
#[gpui_kit::test]
fn bridge_rejects_replays_without_touching_route(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut state = AppState::new();
    let mut bridge = UiBridge::new();

    let first = bridge.next_request_id();
    let generation = state.route_generation;
    let events = dispatch_command(
        &mut state,
        &mut bridge,
        CommandEnvelope {
            request_id: first,
            route_generation: generation,
            command: UiCommand::Navigate(Route::Agents),
        },
    );
    assert_eq!(events.len(), 1);
    assert_eq!(state.route, Route::Agents);

    let replay = dispatch_command(
        &mut state,
        &mut bridge,
        CommandEnvelope {
            request_id: first,
            route_generation: 0,
            command: UiCommand::Navigate(Route::Library),
        },
    );
    assert_eq!(replay.len(), 1);
    assert!(matches!(replay[0], UiEvent::CommandRejected { .. }));
    assert_eq!(state.route, Route::Agents);
}
