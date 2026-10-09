use gpui_kit::base::Disableable as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render,
    StatefulInteractiveElement as _, Styled, Subscription, TestSupportExt as _, Window,
    component::{
        Icon,
        button::{Button, ButtonVariants as _},
        input::{Input, InputState},
        sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
        theme::ActiveTheme as _,
    },
    div, px,
};

use crate::services::DesktopServices;
use crate::views::library::{LibraryLoadState, LibraryOperationPhase, LibraryQueryRequest};
use crate::{
    bridge::{
        CommandEnvelope, LibraryActionKind, LibraryQueryFailure, UiBridge, UiCommand, UiEvent,
        apply_event, dispatch_command,
    },
    routes::Route,
    state::AppState,
};
use jameskills_core::{
    application::ExportRequest,
    domain::{
        CreateSkill, ImportClassification, ImportResolution, RestoreRevisionRequest,
        import::{ImportScanStatus, ImportSourceKind},
    },
};
use std::{path::PathBuf, sync::Arc, time::Duration};

const LIBRARY_SEARCH_DEBOUNCE: Duration = Duration::from_millis(250);

/// Shell real de la aplicación: rail de navegación con primitivos del
/// catálogo, contenido por ruta con estados vacíos honestos y barra de
/// estado. Navegación y búsqueda del catálogo despachan por el bridge; las
/// consultas corren fuera del hilo de render mediante RuntimeServices.
pub struct Shell {
    state: AppState,
    bridge: UiBridge,
    search: Entity<InputState>,
    create_slug: Entity<InputState>,
    create_name: Entity<InputState>,
    restore_version: Entity<InputState>,
    select_after_load: Option<jameskills_core::domain::SkillId>,
    export_destination: Option<PathBuf>,
    _search_subscription: Subscription,
    _services: Arc<DesktopServices>,
}

impl Shell {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        services: Arc<DesktopServices>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Buscar skills"));
        let create_slug = cx.new(|cx| InputState::new(window, cx).placeholder("portable-slug"));
        let create_name = cx.new(|cx| InputState::new(window, cx).placeholder("Nombre visible"));
        let restore_version =
            cx.new(|cx| InputState::new(window, cx).placeholder("Nueva versión SemVer"));
        let search_subscription = cx.observe(&search, |this, search, cx| {
            let query = search.read(cx).value().to_string();
            this.schedule_library_search(query, cx);
        });
        let mut this = Self {
            state: AppState::new(),
            bridge: UiBridge::new(),
            search,
            create_slug,
            create_name,
            restore_version,
            select_after_load: None,
            export_destination: None,
            _search_subscription: search_subscription,
            _services: services,
        };
        this.schedule_library_search(String::new(), cx);
        this
    }

    fn schedule_library_search(&mut self, search: String, cx: &mut Context<Self>) {
        if self.state.route != Route::Library {
            return;
        }
        let request = match self.state.library.begin_search(search) {
            Ok(request) => request,
            Err(_) => {
                let generation = self.state.library.generation();
                self.state.library.apply_failure(generation);
                cx.notify();
                return;
            }
        };
        self.dispatch_library_query(request, cx);
    }

    fn schedule_adjacent_page(&mut self, next: bool, cx: &mut Context<Self>) {
        if self.state.route != Route::Library {
            return;
        }
        let result = if next {
            self.state.library.begin_next_page()
        } else {
            self.state.library.begin_previous_page()
        };
        match result {
            Ok(Some(request)) => self.dispatch_library_query(request, cx),
            Ok(None) => {}
            Err(_) => {
                let generation = self.state.library.generation();
                self.state.library.apply_failure(generation);
                cx.notify();
            }
        }
    }

    fn dispatch_library_action(
        &mut self,
        operation_generation: u64,
        action: LibraryActionKind,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        let request_id = self.bridge.next_request_id();
        let route_generation = self.state.route_generation;
        let events = dispatch_command(
            &mut self.state,
            &mut self.bridge,
            CommandEnvelope {
                request_id,
                route_generation,
                command: UiCommand::LibraryAction {
                    operation_generation,
                    action,
                },
            },
        );
        if matches!(events.first(), Some(UiEvent::LibraryActionStarted { .. })) {
            cx.notify();
            Some(request_id)
        } else {
            self.state.library_operation.fail(operation_generation);
            cx.notify();
            None
        }
    }

    fn create_skill(&mut self, cx: &mut Context<Self>) {
        let generation = self.state.library_operation.generation();
        let create = match CreateSkill::new(
            self.create_slug.read(cx).value().to_string(),
            self.create_name.read(cx).value().to_string(),
        ) {
            Ok(create) => create,
            Err(_) => {
                self.state.library_operation.fail(generation);
                cx.notify();
                return;
            }
        };
        if !self.state.library_operation.begin_create_apply(generation) {
            return;
        }
        let Some(request_id) =
            self.dispatch_library_action(generation, LibraryActionKind::Create, cx)
        else {
            return;
        };
        let route_generation = self.state.route_generation;
        let services = self._services.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    services.run_io(services.runtime_services().library().create_skill(create))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let event = match result {
                    Ok(draft) => {
                        this.select_after_load = Some(draft.skill_id());
                        UiEvent::LibraryActionCompleted {
                            request_id,
                            operation_generation: generation,
                            action: LibraryActionKind::Create,
                        }
                    }
                    Err(_) => UiEvent::LibraryActionFailed {
                        request_id,
                        operation_generation: generation,
                        action: LibraryActionKind::Create,
                        failure: LibraryQueryFailure::Unavailable,
                    },
                };
                apply_event(&mut this.state, &mut this.bridge, event);
                if this.state.route == Route::Library
                    && this.state.route_generation == route_generation
                    && matches!(
                        this.state.library_operation.phase(),
                        LibraryOperationPhase::Complete | LibraryOperationPhase::Error
                    )
                {
                    let query = this.state.library.search_query().to_owned();
                    this.schedule_library_search(query, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn start_import_picker(&mut self, folder: bool, cx: &mut Context<Self>) {
        let generation = match self.state.library_operation.begin_import_selection() {
            Ok(generation) => generation,
            Err(_) => return,
        };
        self.export_destination = None;
        let services = self._services.clone();
        let dialog = if folder {
            services.file_dialog().pick_bundle_folder()
        } else {
            services.file_dialog().pick_bundle_file()
        };
        cx.notify();
        cx.spawn(async move |this, cx| {
            let path = cx
                .background_executor()
                .spawn(async move { services.run_io(dialog) })
                .await;
            let Some(path) = path else {
                let _ = this.update(cx, |this, cx| {
                    if this.state.library_operation.generation() == generation {
                        let _ = this.state.library_operation.cancel();
                    }
                    cx.notify();
                });
                return;
            };
            let source_kind = if folder {
                ImportSourceKind::Directory
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jskill"))
            {
                ImportSourceKind::Archive
            } else {
                ImportSourceKind::PlainSkill
            };
            let mut request_id = None;
            let started = this.update(cx, |this, cx| {
                if this
                    .state
                    .library_operation
                    .begin_import_preview(generation)
                {
                    request_id = this.dispatch_library_action(
                        generation,
                        LibraryActionKind::PreviewImport,
                        cx,
                    );
                }
            });
            if started.is_err() || request_id.is_none() {
                return;
            }
            let request_id = request_id.unwrap_or_default();
            let services = this.update(cx, |this, _| this._services.clone()).ok();
            let Some(services) = services else {
                return;
            };
            let result = cx
                .background_executor()
                .spawn(async move {
                    services.run_io(
                        services
                            .runtime_services()
                            .library()
                            .preview_import(&path, source_kind),
                    )
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let event = match result {
                    Ok(preview) => {
                        this.state
                            .library_operation
                            .set_import_preview(generation, preview);
                        UiEvent::LibraryActionCompleted {
                            request_id,
                            operation_generation: generation,
                            action: LibraryActionKind::PreviewImport,
                        }
                    }
                    Err(_) => UiEvent::LibraryActionFailed {
                        request_id,
                        operation_generation: generation,
                        action: LibraryActionKind::PreviewImport,
                        failure: LibraryQueryFailure::Unavailable,
                    },
                };
                apply_event(&mut this.state, &mut this.bridge, event);
                cx.notify();
            });
        })
        .detach();
    }

    fn apply_import(&mut self, resolution: ImportResolution, cx: &mut Context<Self>) {
        let generation = self.state.library_operation.generation();
        let digest = match self
            .state
            .library_operation
            .select_import_resolution(generation, resolution)
        {
            Ok(digest) => digest,
            Err(_) => return,
        };
        let Some((preview, resolution, confirmed_digest)) =
            self.state.library_operation.begin_import_apply(generation)
        else {
            return;
        };
        let digest_matches = preview
            .confirmation_digest(resolution)
            .is_ok_and(|expected| expected == confirmed_digest);
        if digest != confirmed_digest || !digest_matches {
            self.state.library_operation.fail(generation);
            cx.notify();
            return;
        }
        let skill_id = preview.skill_id();
        let Some(request_id) =
            self.dispatch_library_action(generation, LibraryActionKind::ApplyImport, cx)
        else {
            return;
        };
        let route_generation = self.state.route_generation;
        let services = self._services.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    services.run_io(
                        services
                            .runtime_services()
                            .library()
                            .apply_import(preview, resolution),
                    )
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let event = match result {
                    Ok(_) => {
                        this.select_after_load = Some(skill_id);
                        UiEvent::LibraryActionCompleted {
                            request_id,
                            operation_generation: generation,
                            action: LibraryActionKind::ApplyImport,
                        }
                    }
                    Err(_) => UiEvent::LibraryActionFailed {
                        request_id,
                        operation_generation: generation,
                        action: LibraryActionKind::ApplyImport,
                        failure: LibraryQueryFailure::Unavailable,
                    },
                };
                apply_event(&mut this.state, &mut this.bridge, event);
                if this.state.route == Route::Library
                    && this.state.route_generation == route_generation
                    && matches!(
                        this.state.library_operation.phase(),
                        LibraryOperationPhase::Complete | LibraryOperationPhase::Error
                    )
                {
                    let query = this.state.library.search_query().to_owned();
                    this.schedule_library_search(query, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn start_export_picker(
        &mut self,
        skill_id: jameskills_core::domain::SkillId,
        revision_id: Option<jameskills_core::domain::RevisionId>,
        cx: &mut Context<Self>,
    ) {
        let suggested_name = self
            .state
            .library
            .items()
            .iter()
            .find(|item| item.skill_id() == skill_id)
            .map(|item| format!("{}.jskill", item.slug()))
            .unwrap_or_else(|| "skill.jskill".to_owned());
        let generation = match self.state.library_operation.begin_export_selection() {
            Ok(generation) => generation,
            Err(_) => return,
        };
        self.export_destination = None;
        let services = self._services.clone();
        let dialog = services.file_dialog().save_bundle(suggested_name);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let destination = cx
                .background_executor()
                .spawn(async move { services.run_io(dialog) })
                .await;
            let Some(destination) = destination else {
                let _ = this.update(cx, |this, cx| {
                    if this.state.library_operation.generation() == generation {
                        let _ = this.state.library_operation.cancel();
                    }
                    cx.notify();
                });
                return;
            };
            let mut request_id = None;
            let started = this.update(cx, |this, cx| {
                if this
                    .state
                    .library_operation
                    .begin_export_preview(generation)
                {
                    request_id = this.dispatch_library_action(
                        generation,
                        LibraryActionKind::PreviewExport,
                        cx,
                    );
                }
            });
            if started.is_err() || request_id.is_none() {
                return;
            }
            let request_id = request_id.unwrap_or_default();
            let request = ExportRequest::new(skill_id, revision_id);
            let services = this.update(cx, |this, _| this._services.clone()).ok();
            let Some(services) = services else {
                return;
            };
            let preview_destination = destination.clone();
            let result = cx
                .background_executor()
                .spawn(async move {
                    services.run_io(
                        services
                            .runtime_services()
                            .library()
                            .preview_export(request, preview_destination),
                    )
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let event = match result {
                    Ok(preview) => {
                        if this
                            .state
                            .library_operation
                            .set_export_preview(generation, preview)
                        {
                            this.export_destination = Some(destination);
                        }
                        UiEvent::LibraryActionCompleted {
                            request_id,
                            operation_generation: generation,
                            action: LibraryActionKind::PreviewExport,
                        }
                    }
                    Err(_) => UiEvent::LibraryActionFailed {
                        request_id,
                        operation_generation: generation,
                        action: LibraryActionKind::PreviewExport,
                        failure: LibraryQueryFailure::Unavailable,
                    },
                };
                apply_event(&mut this.state, &mut this.bridge, event);
                cx.notify();
            });
        })
        .detach();
    }

    fn apply_export(&mut self, overwrite: bool, cx: &mut Context<Self>) {
        let generation = self.state.library_operation.generation();
        if self
            .state
            .library_operation
            .select_export_overwrite(generation, overwrite)
            .is_err()
        {
            return;
        }
        let Some((preview, overwrite, confirmation_digest)) =
            self.state.library_operation.begin_export_apply(generation)
        else {
            return;
        };
        let Some(request_id) =
            self.dispatch_library_action(generation, LibraryActionKind::ApplyExport, cx)
        else {
            return;
        };
        let services = self._services.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    services.run_io(services.runtime_services().library().apply_export(
                        preview,
                        overwrite,
                        &confirmation_digest,
                    ))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let event = match result {
                    Ok(_) => UiEvent::LibraryActionCompleted {
                        request_id,
                        operation_generation: generation,
                        action: LibraryActionKind::ApplyExport,
                    },
                    Err(_) => UiEvent::LibraryActionFailed {
                        request_id,
                        operation_generation: generation,
                        action: LibraryActionKind::ApplyExport,
                        failure: LibraryQueryFailure::Unavailable,
                    },
                };
                apply_event(&mut this.state, &mut this.bridge, event);
                cx.notify();
            });
        })
        .detach();
    }

    fn cancel_library_operation(&mut self, cx: &mut Context<Self>) {
        if self.state.library_operation.cancel().is_ok() {
            self.export_destination = None;
            cx.notify();
        }
    }

    fn load_history(&mut self, cx: &mut Context<Self>) {
        let Some(skill_id) = self.state.library.selected_skill() else {
            return;
        };
        let (generation, query) = match self.state.library_operation.begin_history(skill_id) {
            Ok(request) => request,
            Err(_) => return,
        };
        self.dispatch_history_query(generation, query, cx);
    }

    fn load_next_history_page(&mut self, cx: &mut Context<Self>) {
        let request = match self.state.library_operation.begin_next_history_page() {
            Ok(Some(request)) => request,
            Ok(None) | Err(_) => return,
        };
        self.dispatch_history_query(request.0, request.1, cx);
    }

    fn dispatch_history_query(
        &mut self,
        operation_generation: u64,
        query: jameskills_core::ports::LibraryHistoryQuery,
        cx: &mut Context<Self>,
    ) {
        let request_id = self.bridge.next_request_id();
        let route_generation = self.state.route_generation;
        let event = dispatch_command(
            &mut self.state,
            &mut self.bridge,
            CommandEnvelope {
                request_id,
                route_generation,
                command: UiCommand::LoadLibraryHistory {
                    operation_generation,
                    query: query.clone(),
                },
            },
        );
        if !matches!(event.first(), Some(UiEvent::LibraryHistoryStarted { .. })) {
            self.state.library_operation.fail(operation_generation);
            cx.notify();
            return;
        }
        let services = self._services.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    services.run_io(services.runtime_services().library().load_history(query))
                })
                .await;
            let event = match result {
                Ok(page) => UiEvent::LibraryHistoryLoaded {
                    request_id,
                    route_generation,
                    operation_generation,
                    page,
                },
                Err(_) => UiEvent::LibraryHistoryFailed {
                    request_id,
                    route_generation,
                    operation_generation,
                    failure: LibraryQueryFailure::Unavailable,
                },
            };
            let _ = this.update(cx, |this, cx| {
                apply_event(&mut this.state, &mut this.bridge, event);
                cx.notify();
            });
        })
        .detach();
    }

    fn begin_delete_confirmation(&mut self, cx: &mut Context<Self>) {
        let Some(skill_id) = self.state.library.selected_skill() else {
            return;
        };
        let expected_heads = self
            .state
            .library
            .items()
            .iter()
            .find(|item| item.skill_id() == skill_id)
            .map(|item| {
                item.heads()
                    .iter()
                    .map(|head| head.revision_id().clone())
                    .collect()
            });
        let Some(expected_heads) = expected_heads else {
            return;
        };
        if self
            .state
            .library_operation
            .begin_delete_confirmation(skill_id, expected_heads)
            .is_ok()
        {
            cx.notify();
        }
    }

    fn begin_restore_confirmation(&mut self, cx: &mut Context<Self>) {
        let Some(skill_id) = self.state.library_operation.history_skill() else {
            return;
        };
        let Some(source_revision_id) = self
            .state
            .library_operation
            .history_selected_revision()
            .cloned()
        else {
            return;
        };
        let Some(item) = self
            .state
            .library
            .items()
            .iter()
            .find(|item| item.skill_id() == skill_id)
        else {
            return;
        };
        let expected_heads = item
            .heads()
            .iter()
            .map(|head| head.revision_id().clone())
            .collect();
        let deleted_only = item.deleted();
        let next_version = self.restore_version.read(cx).value().to_string();
        let request = match RestoreRevisionRequest::new(
            skill_id,
            source_revision_id,
            expected_heads,
            next_version,
        ) {
            Ok(request) => request,
            Err(_) => {
                let _ = self
                    .state
                    .library_operation
                    .fail(self.state.library_operation.generation());
                cx.notify();
                return;
            }
        };
        if self
            .state
            .library_operation
            .begin_restore_confirmation(request, deleted_only)
            .is_ok()
        {
            cx.notify();
        }
    }

    fn apply_delete(&mut self, cx: &mut Context<Self>) {
        let generation = self.state.library_operation.generation();
        let Some(request) = self.state.library_operation.begin_delete_apply(generation) else {
            return;
        };
        let skill_id = request.skill_id();
        let Some(request_id) =
            self.dispatch_library_action(generation, LibraryActionKind::Delete, cx)
        else {
            return;
        };
        let route_generation = self.state.route_generation;
        let services = self._services.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    services.run_io(services.runtime_services().library().delete_skill(request))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let succeeded = result.is_ok();
                let event = if succeeded {
                    UiEvent::LibraryActionCompleted {
                        request_id,
                        operation_generation: generation,
                        action: LibraryActionKind::Delete,
                    }
                } else {
                    UiEvent::LibraryActionFailed {
                        request_id,
                        operation_generation: generation,
                        action: LibraryActionKind::Delete,
                        failure: LibraryQueryFailure::Unavailable,
                    }
                };
                apply_event(&mut this.state, &mut this.bridge, event);
                if succeeded {
                    this.select_after_load = Some(skill_id);
                }
                if this.state.route == Route::Library
                    && this.state.route_generation == route_generation
                    && matches!(
                        this.state.library_operation.phase(),
                        LibraryOperationPhase::Complete | LibraryOperationPhase::Error
                    )
                {
                    let query = this.state.library.search_query().to_owned();
                    this.schedule_library_search(query, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn apply_restore(&mut self, cx: &mut Context<Self>) {
        let generation = self.state.library_operation.generation();
        let Some((request, deleted_only)) =
            self.state.library_operation.begin_restore_apply(generation)
        else {
            return;
        };
        let skill_id = request.skill_id();
        let action = LibraryActionKind::Restore;
        let Some(request_id) = self.dispatch_library_action(generation, action, cx) else {
            return;
        };
        let route_generation = self.state.route_generation;
        let services = self._services.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let library = services.runtime_services().library();
                    services.run_io(async move {
                        if deleted_only {
                            library.restore_deleted_skill(request).await
                        } else {
                            library.restore_revision_as_new(request).await
                        }
                    })
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let succeeded = result.is_ok();
                let event = if succeeded {
                    UiEvent::LibraryActionCompleted {
                        request_id,
                        operation_generation: generation,
                        action,
                    }
                } else {
                    UiEvent::LibraryActionFailed {
                        request_id,
                        operation_generation: generation,
                        action,
                        failure: LibraryQueryFailure::Unavailable,
                    }
                };
                apply_event(&mut this.state, &mut this.bridge, event);
                if succeeded {
                    this.select_after_load = Some(skill_id);
                }
                if this.state.route == Route::Library
                    && this.state.route_generation == route_generation
                    && matches!(
                        this.state.library_operation.phase(),
                        LibraryOperationPhase::Complete | LibraryOperationPhase::Error
                    )
                {
                    let query = this.state.library.search_query().to_owned();
                    this.schedule_library_search(query, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn dispatch_library_query(&mut self, request: LibraryQueryRequest, cx: &mut Context<Self>) {
        let query_generation = request.generation();
        let query = request.query().clone();
        let route_generation = self.state.route_generation;
        let request_id = self.bridge.next_request_id();
        let envelope = CommandEnvelope {
            request_id,
            route_generation,
            command: UiCommand::SearchLibrary {
                generation: query_generation,
                query: query.clone(),
            },
        };
        let events = dispatch_command(&mut self.state, &mut self.bridge, envelope);
        if matches!(events.first(), Some(UiEvent::CommandRejected { .. })) {
            return;
        }
        let services = self._services.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(LIBRARY_SEARCH_DEBOUNCE)
                .await;
            let still_current = this
                .update(cx, |this, _| {
                    this.state.route == Route::Library
                        && this.state.route_generation == route_generation
                        && this.state.library.generation() == query_generation
                })
                .unwrap_or(false);
            if !still_current {
                return;
            }
            let result = cx
                .background_executor()
                .spawn(async move {
                    services.run_io(services.runtime_services().library().list_skills(query))
                })
                .await;
            let event = match result {
                Ok(page) => UiEvent::LibraryPageLoaded {
                    request_id,
                    route_generation,
                    query_generation,
                    page,
                },
                Err(_) => UiEvent::LibraryPageFailed {
                    request_id,
                    route_generation,
                    query_generation,
                    failure: LibraryQueryFailure::Unavailable,
                },
            };
            let _ = this.update(cx, |this, cx| {
                let page_loaded = matches!(&event, UiEvent::LibraryPageLoaded { .. });
                apply_event(&mut this.state, &mut this.bridge, event);
                if page_loaded
                    && this.state.route == Route::Library
                    && this.state.route_generation == route_generation
                    && this.state.library.generation() == query_generation
                    && let Some(skill_id) = this.select_after_load.take()
                {
                    this.state.library.select_skill(skill_id);
                }
                cx.notify();
            });
        })
        .detach();
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
                                    let request_id = this.bridge.next_request_id();
                                    let envelope = CommandEnvelope {
                                        request_id,
                                        route_generation: this.state.route_generation,
                                        command: UiCommand::Navigate(route),
                                    };
                                    let _events = dispatch_command(
                                        &mut this.state,
                                        &mut this.bridge,
                                        envelope,
                                    );
                                    if route != Route::Library {
                                        this.cancel_library_operation(cx);
                                    }
                                    if route == Route::Library {
                                        let query = this.search.read(cx).value().to_string();
                                        this.schedule_library_search(query, cx);
                                    }
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
                "Biblioteca local. Busca, selecciona una skill o recorre sus páginas.",
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
        let library_state = self.state.library.load_state();
        let library_message = match library_state {
            LibraryLoadState::Idle | LibraryLoadState::Loading => "Cargando biblioteca…".to_owned(),
            LibraryLoadState::Empty => {
                "No hay skills que coincidan. Prueba otra búsqueda.".to_owned()
            }
            LibraryLoadState::Ready => {
                format!("{} skills en esta página", self.state.library.items().len())
            }
            LibraryLoadState::Error => "No se pudo cargar la biblioteca local.".to_owned(),
        };
        let selected_skill = self.state.library.selected_skill();
        let selected_item = self
            .state
            .library
            .items()
            .iter()
            .find(|item| Some(item.skill_id()) == selected_skill);
        let can_export_selected = selected_item
            .is_some_and(|item| !item.deleted() && !item.conflicted() && item.heads().len() == 1);
        let can_delete_selected =
            selected_item.is_some_and(|item| !item.deleted() && !item.heads().is_empty());
        let can_previous_page = self.state.library.can_previous();
        let can_next_page = self.state.library.can_next();
        let page_number = self.state.library.page_number();
        let muted = cx.theme().muted_foreground;
        let library_rows: Vec<_> = self
            .state
            .library
            .items()
            .iter()
            .map(|item| {
                let skill_id = item.skill_id();
                let row_id = format!("library-item-{}", skill_id.as_uuid());
                let selected = selected_skill == Some(skill_id);
                let name = item.display_name().to_owned();
                let slug = item.slug().to_owned();
                let version = item
                    .heads()
                    .first()
                    .map(|head| head.semantic_version().to_owned())
                    .unwrap_or_else(|| "Borrador".to_owned());
                let item_state = if item.conflicted() {
                    format!("Conflicto · {} revisiones", item.heads().len())
                } else if item.deleted() {
                    "Eliminada · disponible en historial".to_owned()
                } else {
                    version
                };
                let on_click = cx.listener(move |this, _, _, cx| {
                    this.state.library.select_skill(skill_id);
                    cx.notify();
                });
                let button = Button::new(row_id)
                    .label(format!("{name} · {slug} · {item_state}"))
                    .on_click(on_click);
                if selected {
                    button.primary().into_any_element()
                } else {
                    button.ghost().into_any_element()
                }
            })
            .collect();
        let pagination = div()
            .flex()
            .items_center()
            .gap_3()
            .child(div().child(format!("Página {page_number}")))
            .when(can_previous_page, |pager| {
                pager.child(
                    Button::new("library-page-previous")
                        .outline()
                        .label("Anterior")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.schedule_adjacent_page(false, cx);
                        })),
                )
            })
            .when(can_next_page, |pager| {
                pager.child(
                    Button::new("library-page-next")
                        .outline()
                        .label("Siguiente")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.schedule_adjacent_page(true, cx);
                        })),
                )
            });
        let operation_phase = self.state.library_operation.phase();
        let operation_busy = !matches!(
            operation_phase,
            LibraryOperationPhase::Idle
                | LibraryOperationPhase::Complete
                | LibraryOperationPhase::Error
        );
        let operations = div()
            .id("library-actions")
            .test_support()
            .flex()
            .items_center()
            .gap_2()
            .when(route != Route::Library, |actions| {
                actions.h_0().overflow_hidden()
            })
            .child(
                Button::new("library-create")
                    .primary()
                    .label("Crear skill")
                    .disabled(operation_busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.state.library_operation.begin_create().is_ok() {
                            this.export_destination = None;
                            cx.notify();
                        }
                    })),
            )
            .child(
                Button::new("library-import-file")
                    .outline()
                    .label("Importar archivo")
                    .disabled(operation_busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.start_import_picker(false, cx);
                    })),
            )
            .child(
                Button::new("library-import-folder")
                    .outline()
                    .label("Importar carpeta")
                    .disabled(operation_busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.start_import_picker(true, cx);
                    })),
            )
            .child(
                Button::new("library-export")
                    .outline()
                    .label("Exportar seleccionada")
                    .disabled(operation_busy || !can_export_selected)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(skill_id) = this.state.library.selected_skill() {
                            this.start_export_picker(skill_id, None, cx);
                        }
                    })),
            )
            .child(
                Button::new("library-history")
                    .outline()
                    .label("Historial")
                    .disabled(operation_busy || selected_skill.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.load_history(cx))),
            )
            .child(
                Button::new("library-delete")
                    .danger()
                    .label("Eliminar")
                    .disabled(operation_busy || !can_delete_selected)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.begin_delete_confirmation(cx);
                    })),
            );
        let operation_panel = match operation_phase {
            LibraryOperationPhase::EditingCreate => Some(
                div()
                    .id("library-create-form")
                    .test_support()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .id("library-create-slug")
                            .test_support()
                            .child(Input::new(&self.create_slug).aria_label("Slug portable de la skill")),
                    )
                    .child(
                        div()
                            .id("library-create-name")
                            .test_support()
                            .child(Input::new(&self.create_name).aria_label("Nombre visible de la skill")),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("library-create-confirm")
                                    .primary()
                                    .label("Crear borrador")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.create_skill(cx);
                                    })),
                            )
                            .child(
                                Button::new("library-create-cancel")
                                    .ghost()
                                    .label("Cancelar")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cancel_library_operation(cx);
                                    })),
                            ),
                    )
                    .into_any_element(),
            ),
            LibraryOperationPhase::ChoosingImport
            | LibraryOperationPhase::PreviewingImport
            | LibraryOperationPhase::ApplyingCreate
            | LibraryOperationPhase::ApplyingImport
            | LibraryOperationPhase::ChoosingExport
            | LibraryOperationPhase::PreviewingExport
            | LibraryOperationPhase::ApplyingExport
            | LibraryOperationPhase::LoadingHistory
            | LibraryOperationPhase::ApplyingDelete
            | LibraryOperationPhase::ApplyingRestore => Some(
                div()
                    .id("library-operation-progress")
                    .test_support()
                    .role(gpui_kit::Role::Status)
                    .aria_label("Operación de biblioteca en curso")
                    .text_color(muted)
                    .child("Operación de biblioteca en curso…")
                    .into_any_element(),
            ),
            LibraryOperationPhase::HistoryReady => {
                let generation = self.state.library_operation.generation();
                let history_skill = self.state.library_operation.history_skill();
                let selected_revision = self
                    .state
                    .library_operation
                    .history_selected_revision()
                    .cloned();
                let entries = self
                    .state
                    .library_operation
                    .history_page()
                    .map(|page| {
                        page.entries()
                            .iter()
                            .map(|entry| {
                                let revision = entry.revision_id().clone();
                                let selected = selected_revision.as_ref() == Some(&revision);
                                let revision_text = revision.as_str().to_owned();
                                let version = entry.semantic_version().to_owned();
                                let status = if entry.deleted() {
                                    "Tombstone · soft delete".to_owned()
                                } else {
                                    format!(
                                        "Contenido · SHA-256 {}",
                                        entry
                                            .bundle_hash()
                                            .map(|hash| hash.as_str())
                                            .unwrap_or("desconocido")
                                    )
                                };
                                let label = format!("v{version} · {status} · {revision_text}");
                                let button = Button::new(format!("library-history-{revision_text}"))
                                    .label(label)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.state
                                            .library_operation
                                            .select_history_revision(generation, &revision);
                                        cx.notify();
                                    }));
                                if selected {
                                    button.primary().into_any_element()
                                } else {
                                    button.ghost().into_any_element()
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let next_history = self.state.library_operation.history_next().is_some();
                let can_delete = selected_item
                    .is_some_and(|item| !item.deleted() && !item.heads().is_empty());
                Some(
                    div()
                        .id("library-history-panel")
                        .test_support()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().font_weight(gpui_kit::FontWeight::MEDIUM).child("Historial causal"))
                        .child(div().text_color(muted).child(
                            "Selecciona una revisión de contenido para restaurarla como una revisión nueva.",
                        ))
                        .child(
                            div()
                                .id("library-history-rows")
                                .test_support()
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .children(entries),
                        )
                        .child(
                            div()
                                .id("library-restore-version")
                                .test_support()
                                .child(Input::new(&self.restore_version).aria_label("Nueva versión SemVer de restauración")),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .when(next_history, |pager| {
                                    pager.child(
                                        Button::new("library-history-next")
                                            .outline()
                                            .label("Más historial")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.load_next_history_page(cx);
                                            })),
                                    )
                                })
                                .when(selected_revision.is_some(), |actions| {
                                    actions.child(
                                        Button::new("library-history-restore")
                                            .primary()
                                            .label("Restaurar selección")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.begin_restore_confirmation(cx);
                                            })),
                                    )
                                })
                                .when(
                                    selected_revision.is_some() && history_skill.is_some(),
                                    |actions| {
                                        let revision = selected_revision
                                            .clone()
                                            .expect("selected revision condition checked");
                                        let skill_id =
                                            history_skill.expect("history skill condition checked");
                                        actions.child(
                                            Button::new("library-history-export")
                                                .outline()
                                                .label("Exportar esta revisión")
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.start_export_picker(
                                                        skill_id,
                                                        Some(revision.clone()),
                                                        cx,
                                                    );
                                                })),
                                        )
                                    },
                                )
                                .when(can_delete, |actions| {
                                    actions.child(
                                        Button::new("library-history-delete")
                                            .danger()
                                            .label("Eliminar skill")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.begin_delete_confirmation(cx);
                                            })),
                                    )
                                })
                                .child(
                                    Button::new("library-history-close")
                                        .ghost()
                                        .label("Cerrar historial")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.cancel_library_operation(cx);
                                        })),
                                ),
                        )
                        .into_any_element(),
                )
            }
            LibraryOperationPhase::ConfirmDelete => {
                let request = self.state.library_operation.delete_request();
                let skill_id = request.map(|request| request.skill_id());
                let expected_heads = request
                    .map(|request| {
                        request
                            .expected_heads()
                            .iter()
                            .map(|head| head.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                Some(
                    div()
                        .id("library-delete-preview")
                        .test_support()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().font_weight(gpui_kit::FontWeight::MEDIUM).child("Eliminar skill de la biblioteca"))
                        .child(div().child(format!("ID {}", skill_id.map(|id| id.as_uuid()).unwrap_or_default())))
                        .child(div().child(format!("Heads que serán tombstone: {expected_heads}")))
                        .child(div().text_color(muted).child("El historial y los bytes se conservan. Una edición concurrente hará fallar el CAS."))
                        .child(
                            Button::new("library-delete-confirm")
                                .danger()
                                .label("Confirmar eliminación reversible")
                                .on_click(cx.listener(|this, _, _, cx| this.apply_delete(cx))),
                        )
                        .child(
                            Button::new("library-delete-cancel")
                                .ghost()
                                .label("Cancelar")
                                .on_click(cx.listener(|this, _, _, cx| this.cancel_library_operation(cx))),
                        )
                        .into_any_element(),
                )
            }
            LibraryOperationPhase::ConfirmRestore => {
                let request = self.state.library_operation.restore_request();
                let source = request
                    .map(|request| request.source_revision_id().as_str())
                    .unwrap_or("desconocida");
                let version = request.map(RestoreRevisionRequest::next_version).unwrap_or("desconocida");
                let deleted_only = self.state.library_operation.restore_deleted_only();
                Some(
                    div()
                        .id("library-restore-preview")
                        .test_support()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().font_weight(gpui_kit::FontWeight::MEDIUM).child("Restaurar como revisión nueva"))
                        .child(div().child(format!("Revisión de origen: {source}")))
                        .child(div().child(format!("Nueva versión: {version}")))
                        .child(div().child(format!("Modo: {}", if deleted_only { "restaurar skill eliminada" } else { "restaurar contenido histórico" })))
                        .child(div().text_color(muted).child("La operación desciende todas las heads observadas; no mueve la historia hacia atrás."))
                        .child(
                            Button::new("library-restore-confirm")
                                .primary()
                                .label("Confirmar restauración")
                                .on_click(cx.listener(|this, _, _, cx| this.apply_restore(cx))),
                        )
                        .child(
                            Button::new("library-restore-cancel")
                                .ghost()
                                .label("Cancelar")
                                .on_click(cx.listener(|this, _, _, cx| this.cancel_library_operation(cx))),
                        )
                        .into_any_element(),
                )
            }
            LibraryOperationPhase::ImportPreview => {
                self.state.library_operation.import_preview().map(|preview| {
                    let (classification, resolutions): (&str, Vec<(ImportResolution, &str)>) =
                        match preview.classification() {
                            ImportClassification::NewSkill
                                if preview.source_kind() == ImportSourceKind::PlainSkill => (
                                    "Instrucciones independientes; se crearán como borrador en cuarentena.",
                                    vec![(
                                        ImportResolution::CreateQuarantinedDraft,
                                        "Crear borrador en cuarentena",
                                    )],
                                ),
                            ImportClassification::NewSkill => (
                                "Skill nueva; se añadirá como raíz de historial concurrente.",
                                vec![(ImportResolution::AddConcurrentRoot, "Importar skill")],
                            ),
                            ImportClassification::Identical { .. } => (
                                "El contenido ya existe; no se duplicará.",
                                vec![(ImportResolution::KeepExisting, "Mantener existente")],
                            ),
                            ImportClassification::Conflict { .. } => (
                                "La identidad ya existe con contenido distinto.",
                                vec![
                                    (ImportResolution::KeepExisting, "Mantener existente"),
                                    (ImportResolution::AddConcurrentRoot, "Añadir raíz concurrente"),
                                ],
                            ),
                        };
                    let title = preview.display_name().to_owned();
                    let slug = preview.slug().to_owned();
                    let version = preview.semantic_version();
                    let content_hash = preview.content_hash().as_str().to_owned();
                    let scan_status = match preview.scan_status() {
                        ImportScanStatus::Unavailable => "No disponible",
                        ImportScanStatus::NoFindings => "Sin hallazgos",
                        ImportScanStatus::Findings => "Hallazgos detectados",
                        ImportScanStatus::Unknown => "Desconocido",
                        ImportScanStatus::Blocked => "Bloqueado",
                    };
                    let actions = resolutions.into_iter().map(|(resolution, label)| {
                        div().child(
                            Button::new(format!("library-import-resolution-{resolution:?}"))
                                .primary()
                                .label(label)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.apply_import(resolution, cx);
                                })),
                        )
                    });
                    div()
                        .id("library-import-preview")
                        .test_support()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().text_size(px(18.)).child(title))
                        .child(div().child(format!("{slug} · v{version}")))
                        .child(div().child(format!("SHA-256 {content_hash}")))
                        .child(div().child(format!("Escaneo de secretos: {scan_status}")))
                        .child(div().text_color(muted).child(classification))
                        .child(div().child("Confianza: en cuarentena hasta revisión explícita."))
                        .children(actions)
                        .child(
                            Button::new("library-import-cancel")
                                .ghost()
                                .label("Cancelar importación")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.cancel_library_operation(cx);
                                })),
                        )
                        .into_any_element()
                })
            }
            LibraryOperationPhase::ExportPreview => {
                self.state.library_operation.export_preview().map(|preview| {
                    let bundle = preview.bundle();
                    let destination_state = match preview.destination_state() {
                        jameskills_core::ports::ExportDestinationState::Missing => {
                            "El destino no existe; se creará un archivo nuevo."
                        }
                        jameskills_core::ports::ExportDestinationState::Existing(_) => {
                            "El destino ya existe; sobrescribir requiere confirmación explícita."
                        }
                    };
                    let destination = self
                        .export_destination
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "Destino seleccionado".to_owned());
                    let revision = bundle.revision_id().as_str().to_owned();
                    let hash = bundle.content_hash().as_str().to_owned();
                    let bytes = bundle.archive_bytes().len();
                    let overwrite = matches!(
                        preview.destination_state(),
                        jameskills_core::ports::ExportDestinationState::Existing(_)
                    );
                    let confirm_label = if overwrite {
                        "Confirmar sobrescritura"
                    } else {
                        "Exportar archivo"
                    };
                    div()
                        .id("library-export-preview")
                        .test_support()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().child(format!("Skill {}", bundle.skill_id().as_uuid())))
                        .child(div().child(format!("Revisión {revision} · SHA-256 {hash}")))
                        .child(div().child(format!("Archivo portable: {bytes} bytes")))
                        .child(div().child(format!("Destino: {destination}")))
                        .child(div().text_color(muted).child(destination_state))
                        .child(
                            Button::new("library-export-confirm")
                                .primary()
                                .label(confirm_label)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.apply_export(overwrite, cx);
                                })),
                        )
                        .child(
                            Button::new("library-export-cancel")
                                .ghost()
                                .label("Cancelar exportación")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.cancel_library_operation(cx);
                                })),
                        )
                        .into_any_element()
                })
            }
            LibraryOperationPhase::Complete => Some(
                div()
                    .id("library-operation-complete")
                    .test_support()
                    .child(
                        self.export_destination
                            .as_ref()
                            .map(|path| format!("Exportado a {}", path.display()))
                            .unwrap_or_else(|| "Operación completada.".to_owned()),
                    )
                    .child(
                        Button::new("library-operation-close")
                            .ghost()
                            .label("Cerrar")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel_library_operation(cx);
                            })),
                    )
                    .into_any_element(),
            ),
            LibraryOperationPhase::Error => Some(
                div()
                    .id("library-operation-error")
                    .test_support()
                    .role(gpui_kit::Role::Alert)
                    .aria_label("La operación de biblioteca falló")
                    .text_color(muted)
                    .child("La operación falló o el preview quedó obsoleto. No se confirmó ningún cambio no verificado.")
                    .child(
                        Button::new("library-operation-dismiss")
                            .ghost()
                            .label("Cerrar")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel_library_operation(cx);
                            })),
                    )
                    .into_any_element(),
            ),
            LibraryOperationPhase::Idle => None,
        };
        let content = div()
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
                    .test_support()
                    .role(gpui_kit::Role::Heading)
                    .aria_level(1)
                    .aria_label(route.label())
                    .text_size(px(24.))
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(title),
            )
            .child(div().text_color(muted).child(body));
        let content = content.child(
            div()
                .id("library-search")
                .test_support()
                .when(route != Route::Library, |search| {
                    search.h_0().overflow_hidden()
                })
                .child(Input::new(&self.search).aria_label("Buscar skills")),
        );
        let content = content.child(operations).children(operation_panel);
        let content = content.child(
            div()
                .id("library-export-guidance")
                .test_support()
                .when(
                    route != Route::Library || selected_skill.is_none() || can_export_selected,
                    |guidance| guidance.h_0().overflow_hidden(),
                )
                .text_color(muted)
                .child("Exportar requiere una skill activa con una única revisión publicada."),
        );
        let content = content.child(
            div()
                .id("library-catalog")
                .test_support()
                .when(route != Route::Library, |catalog| {
                    catalog.h_0().overflow_hidden()
                })
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .id("library-catalog-status")
                        .test_support()
                        .role(gpui_kit::Role::Status)
                        .aria_label(library_message.clone())
                        .text_color(muted)
                        .child(library_message)
                        .when(library_state == LibraryLoadState::Error, |status| {
                            status.child(
                                Button::new("library-retry")
                                    .outline()
                                    .label("Reintentar")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let search = this.state.library.search_query().to_owned();
                                        this.schedule_library_search(search, cx);
                                    })),
                            )
                        }),
                )
                .child(pagination)
                .child(
                    div()
                        .id("library-results")
                        .test_support()
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(library_rows),
                ),
        );
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
                    .child(content)
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
    use crate::file_dialog::{FileDialogFuture, FileDialogPort};
    use crate::services::DesktopServices;
    use crate::views::library::{LibraryLoadState, LibraryOperationPhase};
    use gpui_kit::{AppContext as _, TestAppContext, test::TestWindowExt as _};
    use jameskills_core::ports::{LibraryItemState, LibraryQuery};
    use jameskills_core::{
        application::PublishDraft,
        domain::{CreateSkill, SkillId, import::ImportSourceKind},
    };
    use jameskills_infra::{composition::build_services, platform::UserDirectories};
    use std::time::Duration;
    use std::{path::PathBuf, sync::Arc};

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

    struct ShellTestServices {
        services: Arc<DesktopServices>,
        root: PathBuf,
    }

    impl Drop for ShellTestServices {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn service_host(dialog: Arc<dyn FileDialogPort>) -> ShellTestServices {
        service_host_with(|_| dialog)
    }

    fn service_host_with(
        make_dialog: impl FnOnce(&std::path::Path) -> Arc<dyn FileDialogPort>,
    ) -> ShellTestServices {
        static NEXT_CASE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_CASE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("jameskills-shell-test-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let runtime = build_services(UserDirectories {
            config: root.join("config"),
            data: root.join("data"),
            cache: root.join("cache"),
        })
        .unwrap();
        let services =
            Arc::new(DesktopServices::from_runtime_services(runtime, make_dialog(&root)).unwrap());
        ShellTestServices { services, root }
    }

    fn services() -> (ShellTestServices, SkillId) {
        let fixture = service_host(Arc::new(crate::file_dialog::NativeFileDialog));
        let services = fixture.services.clone();
        let mut first_id = None;
        for index in 0..51 {
            let draft = services
                .run_io(
                    services.runtime_services().library().create_skill(
                        CreateSkill::new(
                            format!("rust-helper-{index:02}"),
                            format!("Rust helper {index:02}"),
                        )
                        .unwrap(),
                    ),
                )
                .unwrap();
            if index == 0 {
                first_id = Some(draft.skill_id());
            }
        }
        (fixture, first_id.expect("first fixture skill"))
    }

    struct CancelledDialogs;

    impl FileDialogPort for CancelledDialogs {
        fn pick_bundle_file(&self) -> FileDialogFuture {
            Box::pin(async { None })
        }

        fn pick_bundle_folder(&self) -> FileDialogFuture {
            Box::pin(async { None })
        }

        fn save_bundle(&self, _suggested_name: String) -> FileDialogFuture {
            Box::pin(async { None })
        }
    }

    struct SelectedPaths {
        import_folder: PathBuf,
        export_file: PathBuf,
    }

    impl FileDialogPort for SelectedPaths {
        fn pick_bundle_file(&self) -> FileDialogFuture {
            Box::pin(async { None })
        }

        fn pick_bundle_folder(&self) -> FileDialogFuture {
            let path = self.import_folder.clone();
            Box::pin(async move { Some(path) })
        }

        fn save_bundle(&self, _suggested_name: String) -> FileDialogFuture {
            let path = self.export_file.clone();
            Box::pin(async move { Some(path) })
        }
    }

    #[gpui_kit::test]
    fn shell_navigates_between_routes(cx: &mut TestAppContext) {
        let (fixture, skill_id) = services();
        let services = fixture.services.clone();
        cx.update(gpui_kit::init);
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, move |window, cx| {
                cx.new(|cx| Shell::new(window, cx, services.clone()))
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
            assert!(window.find("library-search").visible());
            assert_eq!(window.find("shell-route-title").label(), Some("Biblioteca"));
            assert_eq!(view.read(cx).state.route, Route::Library);
        })
        .unwrap();
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                view.read(cx).state.library.load_state(),
                LibraryLoadState::Ready
            );
            assert_eq!(view.read(cx).state.library.items().len(), 50);
            assert!(
                window
                    .find(format!("library-item-{}", skill_id.as_uuid()))
                    .visible()
            );
            window.click(format!("library-item-{}", skill_id.as_uuid()), cx);
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).state.library.selected_skill(), Some(skill_id));
            assert!(window.find("library-page-next").visible());
            assert_eq!(
                window.find("library-catalog-status").label(),
                Some("50 skills en esta página")
            );
            window.click("library-page-next", cx);
        })
        .unwrap();
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).state.library.page_number(), 2);
            assert_eq!(view.read(cx).state.library.items().len(), 1);
            assert_eq!(view.read(cx).state.library.selected_skill(), Some(skill_id));
            assert!(window.find("library-page-previous").visible());
            window.click("library-page-previous", cx);
        })
        .unwrap();
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).state.library.page_number(), 1);
            assert_eq!(view.read(cx).state.library.selected_skill(), Some(skill_id));
            window.click("library-search", cx);
        })
        .unwrap();
        cx.simulate_input(window, "rust");
        cx.update(|cx| assert_eq!(view.read(cx).state.library.search_query(), "rust"));
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                view.read(cx).state.library.load_state(),
                LibraryLoadState::Ready
            );
            assert!(
                window
                    .find(format!("library-item-{}", skill_id.as_uuid()))
                    .visible()
            );
            window.click(nav_item_id(2), cx);
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).state.route, Route::Agents);
            assert_eq!(window.find("shell-route-title").label(), Some("Agentes"));
            assert!(!window.find("library-search").visible());
            assert!(window.find("shell-content").visible());
            window.click(nav_item_id(2), cx);
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).state.route, Route::Agents);
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn cancelling_import_and_export_pickers_does_not_mutate_the_library(cx: &mut TestAppContext) {
        let fixture = service_host(Arc::new(CancelledDialogs));
        let services = fixture.services.clone();
        let created = services
            .run_io(services.runtime_services().library().create_skill(
                CreateSkill::new("cancel-demo".to_owned(), "Cancel demo".to_owned()).unwrap(),
            ))
            .unwrap();
        let skill_id = created.skill_id();
        cx.update(gpui_kit::init);
        let window_services = services.clone();
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, move |window, cx| {
                cx.new(|cx| Shell::new(window, cx, window_services.clone()))
            })
            .expect("open shell window")
        });
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.click(format!("library-item-{}", skill_id.as_uuid()), cx);
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).state.library.selected_skill(), Some(skill_id));
            window.click("library-import-file", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(
                view.read(cx).state.library_operation.phase(),
                LibraryOperationPhase::Idle
            );
        });
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.click("library-export", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(
                view.read(cx).state.library_operation.phase(),
                LibraryOperationPhase::Idle
            );
        });
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.click("library-create", cx);
            window.draw(cx).clear(cx);
            assert!(window.find("library-create-form").visible());
            window.click("library-create-slug", cx);
        })
        .unwrap();
        cx.simulate_input(window, "new-demo");
        cx.update_window(window, |_, window, cx| {
            window.click("library-create-name", cx);
        })
        .unwrap();
        cx.simulate_input(window, "New demo");
        cx.update_window(window, |_, window, cx| {
            window.click("library-create-confirm", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                view.read(cx).state.library_operation.phase(),
                LibraryOperationPhase::Complete
            );
        })
        .unwrap();
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            let library = &view.read(cx).state.library;
            assert_eq!(library.items().len(), 2);
            let created = library
                .items()
                .iter()
                .find(|item| item.slug() == "new-demo")
                .expect("created skill appears in the real catalog query");
            assert_eq!(library.selected_skill(), Some(created.skill_id()));
        })
        .unwrap();
        let page = services
            .run_io(
                services.runtime_services().library().list_skills(
                    LibraryQuery::new(
                        None,
                        Vec::new(),
                        Vec::new(),
                        LibraryItemState::Any,
                        None,
                        50,
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        assert_eq!(page.items().len(), 2);
        assert!(page.items().iter().any(|item| item.skill_id() == skill_id));
        assert!(page.items().iter().any(|item| item.slug() == "new-demo"));
    }

    #[gpui_kit::test]
    fn import_preview_resolution_and_export_overwrite_use_real_library_service(
        cx: &mut TestAppContext,
    ) {
        let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("examples")
            .join("repository-foundation")
            .canonicalize()
            .unwrap();
        let prior_bytes = b"pre-existing user file".to_vec();
        let mut export_destination = None;
        let fixture = service_host_with(|root| {
            let export_directory = root.join("export");
            std::fs::create_dir_all(&export_directory).unwrap();
            let destination = export_directory.join("export-demo.jskill");
            std::fs::write(&destination, &prior_bytes).unwrap();
            export_destination = Some(destination.clone());
            Arc::new(SelectedPaths {
                import_folder: fixture_dir,
                export_file: destination,
            })
        });
        let destination = export_destination.expect("dialog target is rooted in fixture storage");
        let services = fixture.services.clone();
        let draft = services
            .run_io(services.runtime_services().library().create_skill(
                CreateSkill::new("export-demo".to_owned(), "Export demo".to_owned()).unwrap(),
            ))
            .unwrap();
        let skill_id = draft.skill_id();
        let published = services
            .run_io(
                services
                    .runtime_services()
                    .library()
                    .publish(PublishDraft::new(skill_id, draft.generation(), Vec::new()).unwrap()),
            )
            .unwrap();
        let published_revision_id = published.revision().id().clone();

        cx.update(gpui_kit::init);
        let window_services = services.clone();
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, move |window, cx| {
                cx.new(|cx| Shell::new(window, cx, window_services.clone()))
            })
            .expect("open shell window")
        });
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert!(window.find("library-import-folder").visible());
            window.click("library-import-folder", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                view.read(cx).state.library_operation.phase(),
                LibraryOperationPhase::ImportPreview
            );
            assert!(window.find("library-import-preview").visible());
            assert!(
                window
                    .find("library-import-resolution-AddConcurrentRoot")
                    .visible()
            );
            window.click("library-import-resolution-AddConcurrentRoot", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(
                view.read(cx).state.library_operation.phase(),
                LibraryOperationPhase::Complete
            );
        });
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).state.library.items().len(), 2);
            window.click(format!("library-item-{}", skill_id.as_uuid()), cx);
            window.draw(cx).clear(cx);
            window.click("library-history", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.click(
                format!("library-history-{}", published_revision_id.as_str()),
                cx,
            );
            window.draw(cx).clear(cx);
            window.click("library-history-export", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                view.read(cx).state.library_operation.phase(),
                LibraryOperationPhase::ExportPreview
            );
            assert_eq!(
                view.read(cx)
                    .state
                    .library_operation
                    .export_preview()
                    .unwrap()
                    .bundle()
                    .revision_id(),
                &published_revision_id
            );
            assert_eq!(
                window.find("library-export-confirm").label(),
                Some("Confirmar sobrescritura")
            );
            window.click("library-export-confirm", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(
                view.read(cx).state.library_operation.phase(),
                LibraryOperationPhase::Complete
            );
        });

        let exported = std::fs::read(&destination).unwrap();
        assert_ne!(exported, prior_bytes);
        let roundtrip = services
            .run_io(
                services
                    .runtime_services()
                    .library()
                    .preview_import(&destination, ImportSourceKind::Archive),
            )
            .unwrap();
        assert_eq!(roundtrip.skill_id(), skill_id);
        assert_eq!(
            roundtrip.bundle().content_hash(),
            published.revision().bundle_hash()
        );
    }

    #[gpui_kit::test]
    fn history_delete_and_restore_use_causal_heads_and_new_semver_revisions(
        cx: &mut TestAppContext,
    ) {
        let fixture = service_host(Arc::new(CancelledDialogs));
        let services = fixture.services.clone();
        let library = services.runtime_services().library();
        let draft = services
            .run_io(library.create_skill(
                CreateSkill::new("history-demo".to_owned(), "History demo".to_owned()).unwrap(),
            ))
            .unwrap();
        let skill_id = draft.skill_id();
        let initial_revision = services
            .run_io(
                library
                    .publish(PublishDraft::new(skill_id, draft.generation(), Vec::new()).unwrap()),
            )
            .unwrap()
            .revision()
            .id()
            .clone();

        cx.update(gpui_kit::init);
        let window_services = services.clone();
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, move |window, cx| {
                cx.new(|cx| Shell::new(window, cx, window_services.clone()))
            })
            .expect("open shell window")
        });
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.click(format!("library-item-{}", skill_id.as_uuid()), cx);
            window.draw(cx).clear(cx);
            window.click("library-history", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert!(window.find("library-history-panel").visible());
            window.click(format!("library-history-{}", initial_revision.as_str()), cx);
            window.draw(cx).clear(cx);
            window.click("library-restore-version", cx);
        })
        .unwrap();
        cx.simulate_input(window, "1.0.0");
        cx.update_window(window, |_, window, cx| {
            window.click("library-history-restore", cx);
            window.draw(cx).clear(cx);
            assert!(window.find("library-restore-preview").visible());
            window.click("library-restore-confirm", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            let item = view
                .read(cx)
                .state
                .library
                .items()
                .iter()
                .find(|item| item.skill_id() == skill_id)
                .unwrap();
            assert_eq!(item.heads()[0].semantic_version(), "1.0.0");
            window.click("library-delete", cx);
            window.draw(cx).clear(cx);
            assert!(window.find("library-delete-preview").visible());
            window.click("library-delete-confirm", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            let item = view
                .read(cx)
                .state
                .library
                .items()
                .iter()
                .find(|item| item.skill_id() == skill_id)
                .unwrap();
            assert!(item.deleted());
            window.click("library-history", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.click(format!("library-history-{}", initial_revision.as_str()), cx);
            window.draw(cx).clear(cx);
            window.click("library-restore-version", cx);
        })
        .unwrap();
        cx.simulate_keystrokes(window, "ctrl-a");
        cx.simulate_input(window, "2.0.0");
        cx.update_window(window, |_, window, cx| {
            window.click("library-history-restore", cx);
            window.draw(cx).clear(cx);
            window.click("library-restore-confirm", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
            let item = view
                .read(cx)
                .state
                .library
                .items()
                .iter()
                .find(|item| item.skill_id() == skill_id)
                .unwrap();
            assert!(!item.deleted());
            assert_eq!(item.heads()[0].semantic_version(), "2.0.0");
        })
        .unwrap();
    }
}
