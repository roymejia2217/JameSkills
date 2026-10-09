use crate::file_dialog::{FileDialogPort, NativeFileDialog};
use jameskills_core::{AppError, AppResult};
use jameskills_infra::{
    composition::{RuntimeServices, build_services},
    platform::resolve_user_dirs,
};
use std::{
    future::Future,
    sync::{Arc, Mutex},
};

/// Background application services shared by every desktop view. Async storage
/// futures are driven by the retained Tokio runtime; callers must invoke
/// `run_io` from a GPUI background task, never from `Render`.
pub struct DesktopServices {
    runtime_services: RuntimeServices,
    tokio: tokio::runtime::Runtime,
    runtime_gate: Mutex<()>,
    file_dialog: Arc<dyn FileDialogPort>,
}

impl DesktopServices {
    pub fn build() -> AppResult<Self> {
        let directories = resolve_user_dirs().map_err(|_| AppError::CapabilityUnavailable {
            id: "desktop.platform.directories.unavailable".to_owned(),
            guidance_id: "desktop.platform.directories.setup".to_owned(),
        })?;
        let runtime_services = build_services(directories)?;
        Self::from_runtime_services(runtime_services, Arc::new(NativeFileDialog))
    }

    pub fn from_runtime_services(
        runtime_services: RuntimeServices,
        file_dialog: Arc<dyn FileDialogPort>,
    ) -> AppResult<Self> {
        let tokio = tokio::runtime::Builder::new_current_thread()
            .build()
            .map_err(|_| AppError::CapabilityUnavailable {
                id: "desktop.runtime.unavailable".to_owned(),
                guidance_id: "desktop.runtime.setup".to_owned(),
            })?;
        Ok(Self {
            runtime_services,
            tokio,
            runtime_gate: Mutex::new(()),
            file_dialog,
        })
    }

    pub fn runtime_services(&self) -> &RuntimeServices {
        &self.runtime_services
    }

    pub fn file_dialog(&self) -> &dyn FileDialogPort {
        self.file_dialog.as_ref()
    }

    /// Drive an application future on a GPUI background worker. A single Tokio
    /// runtime is reused, and concurrent callers serialize their `block_on`.
    pub fn run_io<F: Future>(&self, future: F) -> F::Output {
        let _guard = self
            .runtime_gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.tokio.block_on(future)
    }
}

#[cfg(test)]
mod tests {
    use super::DesktopServices;
    use crate::file_dialog::{FileDialogFuture, FileDialogPort};
    use jameskills_core::ports::{LibraryItemState, LibraryQuery};
    use jameskills_infra::{composition::build_services, platform::UserDirectories};
    use std::{
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
    };

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "jameskills-desktop-services-{}-{id}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
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

    fn services(root: &Path) -> jameskills_infra::composition::RuntimeServices {
        build_services(UserDirectories {
            config: root.join("config"),
            data: root.join("data"),
            cache: root.join("cache"),
        })
        .unwrap()
    }

    #[test]
    fn desktop_service_host_drives_sqlite_on_tokio_and_dialog_cancel_is_noop() {
        let root = TestRoot::new();
        let services =
            DesktopServices::from_runtime_services(services(&root.0), Arc::new(CancelledDialogs))
                .unwrap();
        assert!(
            services
                .run_io(services.file_dialog().pick_bundle_folder())
                .is_none()
        );
        let page = services
            .run_io(services.runtime_services().library().list_skills(
                LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, None, 20).unwrap(),
            ))
            .unwrap();
        assert!(page.items().is_empty());
    }
}
