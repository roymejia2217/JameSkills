use std::collections::{HashMap, VecDeque};

use crate::views::library::LibraryPageDisposition;
use crate::{routes::Route, state::AppState};
use jameskills_core::ports::{LibraryHistoryPage, LibraryHistoryQuery, LibraryPage, LibraryQuery};

/// Tope del diario de actividad según ARCHITECTURE (cola acotada del bridge).
/// Al llenarse se expulsa lo más antiguo y se cuenta en `dropped_activity`;
/// ningún comando se pierde en silencio.
pub const MAX_ACTIVITY: usize = 64;

/// Sobre con la identidad de un comando de UI (ARCHITECTURE: `CommandEnvelope`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandEnvelope {
    /// Id monótono acuñado por [`UiBridge::next_request_id`]; 0 está reservado.
    pub request_id: u64,
    /// Generación de ruta que veía la vista al despachar.
    pub route_generation: u64,
    pub command: UiCommand,
}

/// Comandos conectados de navegación/búsqueda/acciones de biblioteca. Cada
/// mutación despachada representa una operación UI real con generation guard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiCommand {
    Navigate(Route),
    SearchLibrary {
        generation: u64,
        query: LibraryQuery,
    },
    LibraryAction {
        operation_generation: u64,
        action: LibraryActionKind,
    },
    LoadLibraryHistory {
        operation_generation: u64,
        query: LibraryHistoryQuery,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryActionKind {
    Create,
    PreviewImport,
    ApplyImport,
    PreviewExport,
    ApplyExport,
    Delete,
    Restore,
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
    LibrarySearchStarted {
        request_id: u64,
        query_generation: u64,
    },
    LibraryActionStarted {
        request_id: u64,
        operation_generation: u64,
        action: LibraryActionKind,
    },
    LibraryActionCompleted {
        request_id: u64,
        operation_generation: u64,
        action: LibraryActionKind,
    },
    LibraryActionFailed {
        request_id: u64,
        operation_generation: u64,
        action: LibraryActionKind,
        failure: LibraryQueryFailure,
    },
    LibraryHistoryStarted {
        request_id: u64,
        operation_generation: u64,
    },
    LibraryHistoryLoaded {
        request_id: u64,
        route_generation: u64,
        operation_generation: u64,
        page: LibraryHistoryPage,
    },
    LibraryHistoryFailed {
        request_id: u64,
        route_generation: u64,
        operation_generation: u64,
        failure: LibraryQueryFailure,
    },
    LibraryPageLoaded {
        request_id: u64,
        route_generation: u64,
        query_generation: u64,
        page: LibraryPage,
    },
    LibraryPageFailed {
        request_id: u64,
        route_generation: u64,
        query_generation: u64,
        failure: LibraryQueryFailure,
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
    /// A library command was sent after the view had left the library route.
    WrongRoute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryQueryFailure {
    Unavailable,
}

/// Estado terminal conocido de un comando.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandStatus {
    Pending,
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
            UiEvent::LibrarySearchStarted { request_id, .. } => {
                (*request_id, CommandStatus::Pending)
            }
            UiEvent::LibraryActionStarted { request_id, .. } => {
                (*request_id, CommandStatus::Pending)
            }
            UiEvent::LibraryActionCompleted { request_id, .. } => {
                (*request_id, CommandStatus::Committed)
            }
            UiEvent::LibraryActionFailed { request_id, .. } => (*request_id, CommandStatus::Failed),
            UiEvent::LibraryHistoryStarted { request_id, .. } => {
                (*request_id, CommandStatus::Pending)
            }
            UiEvent::LibraryHistoryLoaded { request_id, .. } => {
                (*request_id, CommandStatus::Committed)
            }
            UiEvent::LibraryHistoryFailed { request_id, .. } => {
                (*request_id, CommandStatus::Failed)
            }
            UiEvent::LibraryPageLoaded { request_id, .. } => {
                (*request_id, CommandStatus::Committed)
            }
            UiEvent::LibraryPageFailed { request_id, .. } => (*request_id, CommandStatus::Failed),
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
    } else if matches!(
        &envelope.command,
        UiCommand::SearchLibrary { .. }
            | UiCommand::LibraryAction { .. }
            | UiCommand::LoadLibraryHistory { .. }
    ) && state.route != Route::Library
    {
        Some(RejectReason::WrongRoute)
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
        UiCommand::SearchLibrary { generation, .. } => UiEvent::LibrarySearchStarted {
            request_id: envelope.request_id,
            query_generation: generation,
        },
        UiCommand::LibraryAction {
            operation_generation,
            action,
        } => UiEvent::LibraryActionStarted {
            request_id: envelope.request_id,
            operation_generation,
            action,
        },
        UiCommand::LoadLibraryHistory {
            operation_generation,
            ..
        } => UiEvent::LibraryHistoryStarted {
            request_id: envelope.request_id,
            operation_generation,
        },
    };
    bridge.record(event.clone());
    vec![event]
}

/// Reductor puro para eventos de cómputos asíncronos del catálogo.
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
        UiEvent::LibraryPageLoaded {
            route_generation,
            query_generation,
            page,
            ..
        } => {
            if state.route == Route::Library
                && *route_generation == state.route_generation
                && state.library.apply_page(*query_generation, page.clone())
                    == LibraryPageDisposition::Applied
            {
                bridge.last_notice = None;
            }
        }
        UiEvent::LibraryPageFailed {
            route_generation,
            query_generation,
            ..
        } => {
            if state.route == Route::Library
                && *route_generation == state.route_generation
                && state.library.apply_failure(*query_generation) == LibraryPageDisposition::Applied
            {
                bridge.last_notice =
                    Some("No se pudo cargar la biblioteca; inténtelo de nuevo.".to_owned());
            }
        }
        UiEvent::LibrarySearchStarted { .. } => {}
        UiEvent::LibraryHistoryStarted { .. } => {}
        UiEvent::LibraryActionStarted { .. } => {}
        UiEvent::LibraryActionCompleted {
            operation_generation,
            ..
        } => {
            state.library_operation.complete(*operation_generation);
            bridge.last_notice = None;
        }
        UiEvent::LibraryActionFailed {
            operation_generation,
            ..
        } => {
            state.library_operation.fail(*operation_generation);
            bridge.last_notice =
                Some("No se pudo completar la operación de biblioteca.".to_owned());
        }
        UiEvent::LibraryHistoryLoaded {
            route_generation,
            operation_generation,
            page,
            ..
        } => {
            if state.route == Route::Library
                && *route_generation == state.route_generation
                && state
                    .library_operation
                    .set_history_page(*operation_generation, page.clone())
            {
                bridge.last_notice = None;
            }
        }
        UiEvent::LibraryHistoryFailed {
            route_generation,
            operation_generation,
            ..
        } => {
            if state.route == Route::Library
                && *route_generation == state.route_generation
                && state.library_operation.fail(*operation_generation)
            {
                bridge.last_notice = Some("No se pudo cargar el historial de la skill.".to_owned());
            }
        }
        UiEvent::CommandRejected { .. } => {}
    }
    bridge.record(event);
}

#[cfg(test)]
mod tests {
    use crate::routes::Route;
    use jameskills_core::{
        domain::{RevisionId, SkillId},
        ports::{LibraryHistoryPage, LibraryPage, LibrarySkillSummary},
    };

    use super::{
        CommandEnvelope, CommandStatus, LibraryActionKind, LibraryQueryFailure, MAX_ACTIVITY,
        RejectReason, UiBridge, UiCommand, UiEvent, apply_event, dispatch_command,
    };
    use crate::state::AppState;

    fn envelope(bridge: &mut UiBridge, state: &AppState, command: UiCommand) -> CommandEnvelope {
        CommandEnvelope {
            request_id: bridge.next_request_id(),
            route_generation: state.route_generation,
            command,
        }
    }

    fn summary(skill_id: SkillId) -> LibrarySkillSummary {
        LibrarySkillSummary::new(
            skill_id,
            "demo".to_owned(),
            "Demo".to_owned(),
            Vec::new(),
            Vec::new(),
            vec![jameskills_core::ports::LibraryHeadSummary::new(
                RevisionId::from_digest([1; 32]),
                "1.0.0".to_owned(),
                false,
            )],
        )
    }

    #[test]
    fn library_search_is_a_pending_command_and_is_rejected_off_route() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();
        let query = state.library.begin_search("demo".to_owned()).unwrap();
        let search = envelope(
            &mut bridge,
            &state,
            UiCommand::SearchLibrary {
                generation: query.generation(),
                query: query.query().clone(),
            },
        );
        let queued = dispatch_command(&mut state, &mut bridge, search);
        assert_eq!(bridge.status_of(1), Some(CommandStatus::Pending));
        assert_eq!(
            queued,
            vec![UiEvent::LibrarySearchStarted {
                request_id: 1,
                query_generation: query.generation(),
            }]
        );

        let navigate = envelope(&mut bridge, &state, UiCommand::Navigate(Route::Agents));
        dispatch_command(&mut state, &mut bridge, navigate);
        let request = bridge.next_request_id();
        let route_generation = state.route_generation;
        let rejected = dispatch_command(
            &mut state,
            &mut bridge,
            CommandEnvelope {
                request_id: request,
                route_generation,
                command: UiCommand::SearchLibrary {
                    generation: query.generation(),
                    query: query.query().clone(),
                },
            },
        );
        assert!(matches!(
            rejected[0],
            UiEvent::CommandRejected {
                reason: RejectReason::WrongRoute,
                ..
            }
        ));
    }

    #[test]
    fn library_mutation_actions_are_route_guarded_and_record_completion_receipts() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();
        let generation = state.library_operation.begin_create().unwrap();
        assert!(state.library_operation.begin_create_apply(generation));
        let start_envelope = envelope(
            &mut bridge,
            &state,
            UiCommand::LibraryAction {
                operation_generation: generation,
                action: LibraryActionKind::Create,
            },
        );
        let started = dispatch_command(&mut state, &mut bridge, start_envelope);
        assert!(matches!(
            started.as_slice(),
            [UiEvent::LibraryActionStarted {
                action: LibraryActionKind::Create,
                ..
            }]
        ));
        let request_id = match &started[0] {
            UiEvent::LibraryActionStarted { request_id, .. } => *request_id,
            _ => unreachable!(),
        };
        assert_eq!(bridge.status_of(request_id), Some(CommandStatus::Pending));
        apply_event(
            &mut state,
            &mut bridge,
            UiEvent::LibraryActionCompleted {
                request_id,
                operation_generation: generation,
                action: LibraryActionKind::Create,
            },
        );
        assert_eq!(bridge.status_of(request_id), Some(CommandStatus::Committed));
        assert_eq!(
            state.library_operation.phase(),
            crate::views::library::LibraryOperationPhase::Complete
        );

        state.navigate(Route::Agents);
        let rejected_envelope = envelope(
            &mut bridge,
            &state,
            UiCommand::LibraryAction {
                operation_generation: generation,
                action: LibraryActionKind::Create,
            },
        );
        let rejected = dispatch_command(&mut state, &mut bridge, rejected_envelope);
        assert!(matches!(
            rejected[0],
            UiEvent::CommandRejected {
                reason: RejectReason::WrongRoute,
                ..
            }
        ));
    }

    #[test]
    fn history_queries_reduce_only_for_the_current_route_and_operation_generation() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();
        let skill_id = SkillId::new();
        let (generation, query) = state.library_operation.begin_history(skill_id).unwrap();
        let request = envelope(
            &mut bridge,
            &state,
            UiCommand::LoadLibraryHistory {
                operation_generation: generation,
                query,
            },
        );
        let started = dispatch_command(&mut state, &mut bridge, request);
        let request_id = match &started[0] {
            UiEvent::LibraryHistoryStarted { request_id, .. } => *request_id,
            other => panic!("unexpected event: {other:?}"),
        };
        let route_generation = state.route_generation;
        apply_event(
            &mut state,
            &mut bridge,
            UiEvent::LibraryHistoryLoaded {
                request_id,
                route_generation,
                operation_generation: generation,
                page: LibraryHistoryPage::new(Vec::new(), None),
            },
        );
        assert_eq!(bridge.status_of(request_id), Some(CommandStatus::Committed));
        assert_eq!(
            state.library_operation.phase(),
            crate::views::library::LibraryOperationPhase::HistoryReady
        );

        state.navigate(Route::Agents);
        let (stale_generation, stale_query) =
            state.library_operation.begin_history(skill_id).unwrap();
        let stale_request = envelope(
            &mut bridge,
            &state,
            UiCommand::LoadLibraryHistory {
                operation_generation: stale_generation,
                query: stale_query,
            },
        );
        let rejected = dispatch_command(&mut state, &mut bridge, stale_request);
        assert!(matches!(
            rejected[0],
            UiEvent::CommandRejected {
                reason: RejectReason::WrongRoute,
                ..
            }
        ));
    }

    #[test]
    fn stale_library_completion_keeps_uuid_selection_and_current_failure_is_visible() {
        let mut state = AppState::new();
        let mut bridge = UiBridge::new();
        let selected = SkillId::new();
        let previous = state.library.begin_search("first".to_owned()).unwrap();
        state.library.apply_page(
            previous.generation(),
            LibraryPage::new(vec![summary(selected)], None),
        );
        assert!(state.library.select_skill(selected));
        let current = state.library.begin_search("second".to_owned()).unwrap();

        let route_generation = state.route_generation;
        apply_event(
            &mut state,
            &mut bridge,
            UiEvent::LibraryPageLoaded {
                request_id: 1,
                route_generation,
                query_generation: previous.generation(),
                page: LibraryPage::new(vec![summary(SkillId::new())], None),
            },
        );
        assert_eq!(state.library.generation(), current.generation());
        assert_eq!(state.library.items()[0].skill_id(), selected);
        assert_eq!(state.library.selected_skill(), Some(selected));

        let route_generation = state.route_generation;
        apply_event(
            &mut state,
            &mut bridge,
            UiEvent::LibraryPageFailed {
                request_id: 2,
                route_generation,
                query_generation: current.generation(),
                failure: LibraryQueryFailure::Unavailable,
            },
        );
        assert_eq!(
            state.library.load_state(),
            crate::views::library::LibraryLoadState::Error
        );
        assert_eq!(state.library.items()[0].skill_id(), selected);
        assert!(bridge.last_notice().is_some());
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
