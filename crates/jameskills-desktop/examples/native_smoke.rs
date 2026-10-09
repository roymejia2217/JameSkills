#[cfg(feature = "test-support")]
use gpui_kit::{AppContext as _, WindowOptions, assets::Assets};
#[cfg(feature = "test-support")]
use jameskills_core::{
    application::{ExportRequest, PublishDraft},
    domain::CreateSkill,
};
#[cfg(feature = "test-support")]
use jameskills_desktop::{
    file_dialog::NativeFileDialog, services::DesktopServices, views::shell::Shell,
};
#[cfg(feature = "test-support")]
use jameskills_infra::{composition::build_services, platform::UserDirectories};
#[cfg(feature = "test-support")]
use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "test-support")]
fn main() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let root = std::env::var_os("JAMESKILLS_NATIVE_SMOKE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "jameskills-native-smoke-{}-{unique}",
                std::process::id()
            ))
        });
    assert!(root.is_absolute(), "native smoke root must be absolute");
    assert!(
        !root.exists(),
        "native smoke root must not exist before the harness starts"
    );
    std::fs::create_dir_all(&root).expect("create isolated native smoke root");
    let directories = UserDirectories {
        config: root.join("config"),
        data: root.join("data"),
        cache: root.join("cache"),
    };
    let runtime =
        build_services(directories.clone()).expect("temporary library services should initialize");
    let services = Arc::new(
        DesktopServices::from_runtime_services(runtime, Arc::new(NativeFileDialog))
            .expect("desktop background runtime should initialize"),
    );
    let fixture_directory = root.join("fixtures");
    std::fs::create_dir_all(&fixture_directory).expect("create temporary fixture directory");
    let library = services.runtime_services().library();
    seed_published_skill(&services, "native-smoke-seed", "Native Smoke Seed");
    let (import_skill, import_revision) = seed_published_skill(
        &services,
        "native-smoke-import",
        "Native Smoke Import Fixture",
    );
    let import_fixture = fixture_directory.join("native-smoke-import.jskill");
    let preview = services
        .run_io(library.preview_export(
            ExportRequest::new(import_skill, Some(import_revision)),
            import_fixture.clone(),
        ))
        .expect("prepare import fixture export");
    let confirmation = preview
        .confirmation_digest(false)
        .expect("new fixture destination does not require overwrite")
        .clone();
    services
        .run_io(library.apply_export(preview, false, &confirmation))
        .expect("write temporary import fixture");
    eprintln!(
        "JameSkills native smoke data: {}",
        directories.data.display()
    );
    eprintln!(
        "JameSkills native smoke import fixture: {}",
        import_fixture.display()
    );

    gpui_kit::application().with_assets(Assets).run(move |cx| {
        gpui_kit::init(cx);
        let window_services = services.clone();
        if gpui_kit::open_window(WindowOptions::default(), cx, move |window, cx| {
            cx.new(|cx| Shell::new(window, cx, window_services.clone()))
        })
        .is_err()
        {
            eprintln!("JameSkills native smoke: no se pudo abrir la ventana.");
            cx.quit();
        }
    });
}

#[cfg(feature = "test-support")]
fn seed_published_skill(
    services: &DesktopServices,
    slug: &str,
    display_name: &str,
) -> (
    jameskills_core::domain::SkillId,
    jameskills_core::domain::RevisionId,
) {
    let library = services.runtime_services().library();
    let draft = services
        .run_io(
            library.create_skill(
                CreateSkill::new(slug.to_owned(), display_name.to_owned())
                    .expect("valid native smoke skill metadata"),
            ),
        )
        .expect("create temporary native smoke skill");
    let result = services
        .run_io(
            library.publish(
                PublishDraft::new(draft.skill_id(), draft.generation(), Vec::new())
                    .expect("valid initial publish request"),
            ),
        )
        .expect("publish temporary native smoke skill");
    (draft.skill_id(), result.revision().id().clone())
}

#[cfg(not(feature = "test-support"))]
fn main() {
    eprintln!("Use --features test-support to run the isolated native UI smoke host.");
}
