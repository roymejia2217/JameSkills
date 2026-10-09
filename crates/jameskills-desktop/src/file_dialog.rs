use std::{future::Future, path::PathBuf, pin::Pin};

pub type FileDialogFuture = Pin<Box<dyn Future<Output = Option<PathBuf>> + Send + 'static>>;

/// User-mediated native file selection. `None` is cancellation; implementations
/// never open the selected path or mutate it.
pub trait FileDialogPort: Send + Sync {
    fn pick_bundle_file(&self) -> FileDialogFuture;
    fn pick_bundle_folder(&self) -> FileDialogFuture;
    fn save_bundle(&self, suggested_name: String) -> FileDialogFuture;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeFileDialog;

impl FileDialogPort for NativeFileDialog {
    fn pick_bundle_file(&self) -> FileDialogFuture {
        Box::pin(async {
            rfd::AsyncFileDialog::new()
                .set_title("Importar skill portable")
                .add_filter("JameSkills o instrucciones", &["jskill", "md"])
                .pick_file()
                .await
                .map(|file| file.path().to_path_buf())
        })
    }

    fn pick_bundle_folder(&self) -> FileDialogFuture {
        Box::pin(async {
            rfd::AsyncFileDialog::new()
                .set_title("Importar carpeta de skill")
                .pick_folder()
                .await
                .map(|folder| folder.path().to_path_buf())
        })
    }

    fn save_bundle(&self, suggested_name: String) -> FileDialogFuture {
        Box::pin(async move {
            rfd::AsyncFileDialog::new()
                .set_title("Exportar skill portable")
                .set_file_name(suggested_name)
                .add_filter("JameSkills portable bundle", &["jskill"])
                .save_file()
                .await
                .map(|file| file.path().to_path_buf())
        })
    }
}
