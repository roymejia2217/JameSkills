use crate::routes::Route;

/// Estado local de la shell según SPEC-desktop-app.
///
/// `operations`, `agent_detection`, `sync_state` y `notices` llegan con el
/// bridge y los servicios (T008.b); aquí solo vive lo que la navegación
/// necesita hoy: ruta, generación, búsqueda y selección.
pub struct AppState {
    pub route: Route,
    pub route_generation: u64,
    pub library_query: String,
    pub selected_skill: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            route: Route::default(),
            route_generation: 0,
            library_query: String::new(),
            selected_skill: None,
        }
    }

    /// Cambia de ruta e invalida la generación para que las respuestas
    /// asíncronas de la ruta anterior no pinten sobre la nueva.
    /// Repetir la ruta actual no invalida nada.
    pub fn navigate(&mut self, route: Route) {
        if self.route != route {
            self.route = route;
            self.route_generation += 1;
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::routes::Route;

    use super::AppState;

    #[test]
    fn new_state_starts_at_the_library() {
        let state = AppState::new();
        assert_eq!(state.route, Route::Library);
        assert_eq!(state.route_generation, 0);
        assert!(state.library_query.is_empty());
        assert_eq!(state.selected_skill, None);
    }

    #[test]
    fn navigate_changes_route_and_bumps_generation() {
        let mut state = AppState::new();
        state.navigate(Route::Agents);
        assert_eq!(state.route, Route::Agents);
        assert_eq!(state.route_generation, 1);
        state.navigate(Route::Sync);
        assert_eq!(state.route, Route::Sync);
        assert_eq!(state.route_generation, 2);
    }

    #[test]
    fn repeating_the_current_route_keeps_generation() {
        let mut state = AppState::new();
        state.navigate(Route::Library);
        assert_eq!(state.route_generation, 0);
    }
}
