# Spec: desktop-app

Native desktop Rust + GPUI Kit =0.7.0, Windows/Linux x86_64. Core/infrastructure desde ARCHITECTURE/CONTRACTS. GUI detallada en GUI; rendimiento/accesibilidad como criterios verificables. No app implementada todavía.

## Inicio y plataforma

Rust1.95.0 baseline, edición2024, Kit facade. Cargo.lock se valida con el toolchain fijado; GPUI snapshot 0.3.7 usa `std::hint::cold_path`, estable desde Rust1.95.0. El experimento con 1.92/1.94 falló por APIs inestables, por lo que no se conserva el baseline inicialmente propuesto. No usar stable flotante. Windows MSVC VS2022+CMake; Linux Wayland/X11 + Vulkan y dependencias oficiales (OPERATIONS).
GUI GPU incompatible/headless -> mensaje diagnóstico CLI/archivo redacted, no esconder crash ni cambiar stack de producto sin decisión.
Native app solo llama system browser para OAuth/enlaces y system file picker mediante API compatible Kit; nunca webview para main GUI.

Blueprint bootstrap source v0.7.0:
~~~rust
use gpui_kit::{assets::Assets, Context, IntoElement, Render, Window, WindowOptions};
use gpui_kit::{div, Styled};

struct ShellView;
impl Render for ShellView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("JameSkills")
    }
}
fn main() {
    gpui_kit::application()
        .with_assets(Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| ShellView)
            }).expect("No se pudo abrir la ventana de JameSkills");
        });
}
~~~
Este bootstrap es blueprint adaptado del facade upstream, aún sin ejecutar; dependencias/source consultadas, build spike lo convierte en evidencia. La app final usa AppError/redacted diagnostic en vez de expect para errores de entorno y crea services+bridge antes ShellView. No añadir un Root extra (open_window ya lo incorpora).

## Route/state

Route enum Onboarding | Library | Skill {id,tab} | Checks {project} | Agents | Sync | Settings.
SkillTab Overview | Instructions | Policies | Requirements | Assets | Versions | Install.
AppState { route, route_generation, library_query, selected_skill, operations, agent_detection, sync_state, notices }.
ViewState UI local no persiste tokens; DTO zero secretos. DraftState con dirty/autosave/revision/errors.
Query generation asegura resultados búsqueda antiguos no pintan sobre nueva; mutation completion actualiza activity even navigation changed.
Unsaved draft navegación prompt solo si persistencia falla/edits aún no guardados, normal autosave no bloques de rutina.
ConnectionState = Disconnected | Authorizing | LinkedLocked | Ready | Syncing | NeedsReauth | Error; al offline Pending todavía usable locale.
Check statuses from domain, colores+texto+icono. No botón deshabilitado sin reason/acción asociada.

## Handler -> usecase obligatorio

| UI intent | Service | Event + state update |
|---|---|---|
| New skill | library.create_skill | DraftCreated -> Skill editor |
| Edit/Autosave | library.save_draft | DraftSaved(generation) -> save indicator |
| Publish | library.publish(expected_heads) | RevisionSaved -> Overview+list statuses stale |
| Import | library.import_bundle | ImportReviewed/Quarantined -> preview and review |
| Export | library.export_bundle | ExportCompleted -> output disclosure |
| Detect agents | install.detect_agents | AgentsDetected -> per agent capabilities |
| Install | install.plan_install then apply_install | PlanReady preview -> Installed receipt |
| Remove | install.remove_installation | RemovalComplete -> associations refresh |
| Validate repo | policy.check | CheckProgress/CheckReport -> Requirements |
| Complete requirement | guidance.advance + recheck | GuidanceUpdated -> next or remains blocked |
| Link Drive | sync.connect | OAuthStatus -> LinkedLocked/Ready |
| Unlock | sync.unlock | VaultUnlocked -> Ready, no secret event |
| Sync now | sync.sync_once | SyncProgress/MergeConflict/SyncCompleted |
| Restore | sync.preview_restore + apply_restore | RestorePlan -> RestoreCompleted |
| Resolve conflict | library.publish with parents all | RevisionSaved -> SyncPending |
| Disconnect | sync.disconnect | Disconnected -> local retained |
| Settings | infrastructure config validated usecase | SettingsSaved -> theme/update schedule |

Toda vista tiene Loading/Empty/Ready/Error, y mutaciones Pending/Success/Error/Cancelled. No buttons fake, timer simulated success ni random metrics. El shell mínimo aparece temprano; cada slice conecta CLI y su UI antes abordar otra capacidad.

## State retention y pruebas

Entity<InputState> creada una vez constructor, retained Subscription; no recrear InputState render. Form validation accessible, focus first error; label association source APIs reales Kit tag.
Kit Button/Input/List/Table/Tab/Dialog/Tooltip/Notification según catálogo0.7; IconName del catálogo fijado; Assets único embed. Bounded tree/list virtualization conforme fuentes. Temas semánticos Kit, no CSS/shadcn web copiado a Rust.
Test-support desktop Cargo feature forwards gpui-kit/test-support, tests #[gpui_kit::test] con interacción real headless fuente recipes. Test rendering GPU real distinto headless. Para screen reader OS usar evidencia de accessibility tree/platform actual, no equiparar semantic labels a integración verificada.
Layout minimum1000x680, initial1280x800; DPI100/150/200%, teclado Tab/ShiftTab/Escape; no input invisible. Offline e idioma español estados completos.
DoD: Windows/Linux build+UI physical/vm GPU compatible + install package; tests headless no reemplazan input+assets actual platform.
