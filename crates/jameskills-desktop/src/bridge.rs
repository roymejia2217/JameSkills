use std::collections::{HashMap, VecDeque};

use crate::{routes::Route, state::AppState};

/// Tope del diario de actividad según ARCHITECTURE (cola acotada del bridge).
/// Al llenarse se expulsa lo más antiguo y se cuenta en `dropped_activity`;
/// ningún comando se pierde en silencio.
pub const MAX_ACTIVITY: usize = 64;

/// Sobre con la identidad de un comando de UI (ARCHITECTURE: `CommandEnvelope`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandEnvelope {
    /// Id monótono acuñado por [`UiBridge::next_request_id`]; 0 está reservado.
    pub request_id: u64,
    /// Generación de ruta que veía la vista al despachar.
    pub route_generation: u64,
    pub command: UiCommand,
}

/// Comandos que la shell puede despachar hoy. Los de biblioteca, instalación
/// y sync llegan con sus servicios (T046/T050/cloud-sync); no se declaran
/// variantes sin productor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiCommand {
    Navigate(Route),
}

/// Resultado de aplicar un comando o de un cómputo asíncrono. Los estados
/// `Failed` y `Cancelled` se representan como tales: jamás se convierten en
/// éxito y jamás mutan la ruta por sí solos.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiEvent {
    NavigationCommitted {
        request_id: u64,
        route: Route,
        generation: u64,
    },
    NavigationFailed {
        request_id: u64,
        route: Route,
        generation: u64,
        reason: String,
    },
    NavigationCancelled {
        request_id: u64,
        route: Route,
        generation: u64,
    },
    CommandRejected {
        request_id: u64,
        reason: RejectReason,
    },
}

/// Motivo de rechazo en el despacho, antes de tocar el estado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectReason {
    /// Id repetido o anterior al máximo ya visto (replay).
    StaleRequestId,
    /// La vista despachó con una generación que ya no es la actual.
    StaleGeneration,
}

/// Estado terminal conocido de un comando.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandStatus {
    Committed,
    Failed,
    Cancelled,
    Rejected,
}

/// Puente entre la UI y los servicios: acuña ids, valida sobres y reduce
/// eventos sobre [`AppState`]. Estructuras de datos planas y `Send`, aptas
/// para cruzar al executor de fondo.
pub struct UiBridge {
    next_request: u64,
    highest_seen_request: u64,
    statuses: HashMap<u64, CommandStatus>,
    activity: VecDeque<UiEvent>,
    dropped_activity: u64,
    last_notice: Option<String>,
}

impl UiBridge {
    pub fn new() -> Self {
        Self {
            next_request: 1,
            highest_seen_request: 0,
            statuses: HashMap::new(),
            activity: VecDeque::new(),
            dropped_activity: 0,
            last_notice: None,
        }
    }

    /// Acuña el siguiente id de comando. Monótono por construcción.
    pub fn next_request_id(&mut self) -> u64 {
        let id = self.next_request;
        self.next_request += 1;
        id
    }

    /// Estado terminal conocido de un comando, si lo hay.
    pub fn status_of(&self, request_id: u64) -> Option<CommandStatus> {
        self.statuses.get(&request_id).copied()
    }

    /// Diario acotado, del más antiguo al más reciente.
    pub fn activity(&self) -> impl Iterator<Item = &UiEvent> {
        self.activity.iter()
    }

    /// Eventos expulsados del diario por el tope. Siempre 0 sería sospechoso
    /// si la sesión despachó más de [`MAX_ACTIVITY`] comandos.
    pub fn dropped_activity(&self) -> u64 {
        self.dropped_activity
    }

    /// Último aviso para el panel persistente (`Failed`); `None` tras un
    /// éxito o una cancelación a petición. La shell lo pinta en T046+.
    pub fn last_notice(&self) -> Option<&str> {
        self.last_notice.as_deref()
    }

    fn record(&mut self, event: UiEvent) {
        let status = match &event {
            UiEvent::NavigationCommitted { request_id, .. } => {
                (*request_id, CommandStatus::Committed)
            }
            UiEvent::NavigationFailed { request_id, .. } => (*request_id, CommandStatus::Failed),
            UiEvent::NavigationCancelled { request_id, .. } => {
                (*request_id, CommandStatus::Cancelled)
            }
            UiEvent::CommandRejected { request_id, .. } => (*request_id, CommandStatus::Rejected),
        };
        self.statuses.insert(status.0, status.1);
        if self.activity.len() >= MAX_ACTIVITY {
            self.activity.pop_front();
            self.dropped_activity += 1;
        }
        self.activity.push_back(event);
    }
}

impl Default for UiBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Valida el sobre y aplica el comando sobre el estado. Devuelve los eventos
/// producidos (hoy, exactamente uno). No toca la ruta ante sobres rancios.
pub fn dispatch_command(
    state: &mut AppState,
    bridge: &mut UiBridge,
    envelope: CommandEnvelope,
) -> Vec<UiEvent> {
    let rejected = if envelope.request_id <= bridge.highest_seen_request {
        Some(RejectReason::StaleRequestId)
    } else if envelope.route_generation != state.route_generation {
        Some(RejectReason::StaleGeneration)
    } else {
        None
    };
    if let Some(reason) = rejected {
        let event = UiEvent::CommandRejected {
            request_id: envelope.request_id,
            reason,
        };
        bridge.record(event.clone());
        return vec![event];
    }
    bridge.highest_seen_request = envelope.request_id;
    let event = match envelope.command {
        UiCommand::Navigate(route) => {
            state.navigate(route);
            bridge.last_notice = None;
            UiEvent::NavigationCommitted {
                request_id: envelope.request_id,
                route,
                generation: state.route_generation,
            }
        }
    };
    bridge.record(event.clone());
    vec![event]
}

/// Reductor puro para eventos de cómputos asíncronos (servicios en T046+).
/// Una completitud tardía no mueve la ruta —la ruta nueva se preserva— pero
/// sí queda en el diario: ningún recibo se pierde porque la vista cambiara.
pub fn apply_event(state: &mut AppState, bridge: &mut UiBridge, event: UiEvent) {
    match &event {
        UiEvent::NavigationCommitted {
            route, generation, ..
        } => {
            if *generation >= state.route_generation {
                state.route = *route;
                state.route_generation = *generation;
            }
            bridge.last_notice = None;
        }
        UiEvent::NavigationFailed { reason, .. } => {
            bridge.last_notice = Some(reason.clone());
        }
        UiEvent::NavigationCancelled { .. } => {
            bridge.last_notice = None;
        }
        UiEvent::CommandRejected { .. } => {}
    }
    bridge.record(event);
}

#[cfg(test)]
mod tests {
    use crate::routes::Route;

    use super::{
        CommandEnvelope, CommandStatus, MAX_ACTIVITY, RejectReason, UiBridge, UiCommand, UiEvent,
        apply_event, dispatch_command,
    };
    use crate::state::AppState;

    fn envelope(bridge: &mut UiBridge, state: &AppState, command: UiCommand) -> CommandEnvelope {
        CommandEnvelope {
            request_id: bridge.next_request_id(),
            route_generation: state.route_generation,
            command,
        }
    }

    #[test]
    fn dispatch_navigate_commits_and_bumps_generation() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();
        let env = envelope(&mut bridge, &state, UiCommand::Navigate(Route::Agents));

        let events = dispatch_command(&mut state, &mut bridge, env);

        assert_eq!(state.route, Route::Agents);
        assert_eq!(state.route_generation, 1);
        assert_eq!(
            events,
            vec![UiEvent::NavigationCommitted {
                request_id: 1,
                route: Route::Agents,
                generation: 1,
            }]
        );
        assert_eq!(bridge.status_of(1), Some(CommandStatus::Committed));
    }

    #[test]
    fn dispatch_rejects_replayed_request_ids() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();
        let env = envelope(&mut bridge, &state, UiCommand::Navigate(Route::Agents));
        dispatch_command(&mut state, &mut bridge, env);

        let replay = dispatch_command(
            &mut state,
            &mut bridge,
            CommandEnvelope {
                request_id: 1,
                route_generation: 0,
                command: UiCommand::Navigate(Route::Library),
            },
        );

        assert_eq!(
            replay,
            vec![UiEvent::CommandRejected {
                request_id: 1,
                reason: RejectReason::StaleRequestId,
            }]
        );
        assert_eq!(state.route, Route::Agents);
        assert_eq!(bridge.status_of(1), Some(CommandStatus::Rejected));
    }

    #[test]
    fn dispatch_rejects_stale_generations() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();
        let env = envelope(&mut bridge, &state, UiCommand::Navigate(Route::Agents));
        dispatch_command(&mut state, &mut bridge, env);

        let stale_id = bridge.next_request_id();
        let stale_view = dispatch_command(
            &mut state,
            &mut bridge,
            CommandEnvelope {
                request_id: stale_id,
                route_generation: 0,
                command: UiCommand::Navigate(Route::Sync),
            },
        );

        assert!(matches!(
            stale_view[0],
            UiEvent::CommandRejected {
                reason: RejectReason::StaleGeneration,
                ..
            }
        ));
        assert_eq!(state.route, Route::Agents);
    }

    #[test]
    fn apply_ignores_late_completion_but_keeps_receipt() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();
        let first = envelope(&mut bridge, &state, UiCommand::Navigate(Route::Agents));
        dispatch_command(&mut state, &mut bridge, first);
        let second = envelope(&mut bridge, &state, UiCommand::Navigate(Route::Settings));
        dispatch_command(&mut state, &mut bridge, second);
        assert_eq!(state.route_generation, 2);

        apply_event(
            &mut state,
            &mut bridge,
            UiEvent::NavigationCommitted {
                request_id: 1,
                route: Route::Agents,
                generation: 1,
            },
        );

        assert_eq!(state.route, Route::Settings);
        assert_eq!(state.route_generation, 2);
        assert!(bridge.activity().any(|event| matches!(
            event,
            UiEvent::NavigationCommitted {
                route: Route::Agents,
                ..
            }
        )));
    }

    #[test]
    fn failed_sets_notice_without_touching_route() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();

        apply_event(
            &mut state,
            &mut bridge,
            UiEvent::NavigationFailed {
                request_id: 7,
                route: Route::Sync,
                generation: 0,
                reason: "transporte no disponible".to_string(),
            },
        );

        assert_eq!(state.route, Route::Library);
        assert_eq!(bridge.last_notice(), Some("transporte no disponible"));
        assert_eq!(bridge.status_of(7), Some(CommandStatus::Failed));
    }

    #[test]
    fn cancelled_is_represented_without_success() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();

        apply_event(
            &mut state,
            &mut bridge,
            UiEvent::NavigationCancelled {
                request_id: 7,
                route: Route::Sync,
                generation: 0,
            },
        );

        assert_eq!(state.route, Route::Library);
        assert_eq!(bridge.last_notice(), None);
        assert_eq!(bridge.status_of(7), Some(CommandStatus::Cancelled));
    }

    #[test]
    fn activity_evicts_oldest_beyond_bound() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();
        for _ in 0..=MAX_ACTIVITY {
            let env = envelope(&mut bridge, &state, UiCommand::Navigate(Route::Agents));
            dispatch_command(&mut state, &mut bridge, env);
            let back = CommandEnvelope {
                request_id: bridge.next_request_id(),
                route_generation: state.route_generation,
                command: UiCommand::Navigate(Route::Library),
            };
            dispatch_command(&mut state, &mut bridge, back);
        }

        assert_eq!(bridge.activity().count(), MAX_ACTIVITY);
        assert!(bridge.dropped_activity() > 0);
    }
}
