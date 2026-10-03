use gpui_kit::component::IconName;

/// Destinos de primer nivel del rail de navegación (GUI: Biblioteca,
/// Proyectos y checks, Agentes, Sincronización, Ajustes).
///
/// Las rutas contextuales (detalle de skill, onboarding) llegan con sus
/// slices (T008.b/T046); no se declaran variantes sin productor ni
/// consumidor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Route {
    #[default]
    Library,
    Checks,
    Agents,
    Sync,
    Settings,
}

impl Route {
    /// Destinos del rail en orden de presentación.
    pub fn all() -> [Route; 5] {
        [
            Route::Library,
            Route::Checks,
            Route::Agents,
            Route::Sync,
            Route::Settings,
        ]
    }

    /// Etiqueta visible en español.
    pub fn label(self) -> &'static str {
        match self {
            Route::Library => "Biblioteca",
            Route::Checks => "Proyectos y checks",
            Route::Agents => "Agentes",
            Route::Sync => "Sincronización",
            Route::Settings => "Ajustes",
        }
    }

    /// Icono Lucide del bundle por defecto (`Assets`) de gpui-kit-assets 0.7.0.
    /// Solo nombres presentes en `default-icons.txt` del crate publicado:
    /// nada fuera del bundle renderiza sin registrar `AllAssets`, y ese
    /// registro vive en `composition.rs` (fuera del alcance de T008.a).
    pub fn icon(self) -> IconName {
        match self {
            Route::Library => IconName::BookOpen,
            Route::Checks => IconName::Folder,
            Route::Agents => IconName::Bot,
            Route::Sync => IconName::RefreshCw,
            Route::Settings => IconName::Settings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Route;

    #[test]
    fn default_route_is_the_library() {
        assert_eq!(Route::default(), Route::Library);
    }

    #[test]
    fn every_route_has_a_distinct_spanish_label() {
        let labels: Vec<&str> = Route::all().into_iter().map(Route::label).collect();
        assert!(labels.iter().all(|label| !label.is_empty()));
        let mut distinct = labels.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), labels.len());
    }
}
