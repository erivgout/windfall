//! The IPC commands of docs/ARCHITECTURE.md, each a thin wrapper around one
//! [`Session`] method.
//!
//! Commands whose order matters and that are quick, such as edits and the
//! transport, are plain functions: Tauri runs those on the main thread, one
//! at a time, in the order the UI sent them, so two moves of one fader
//! cannot overtake each other. Commands that read files, decode audio or
//! wait for a device are `async` and do their work on a blocking-task
//! thread.

use tauri::ipc::{Channel, Invoke};
use tauri::{State, WebviewWindow};
use windfall_ipc::{
    AudioHost, AudioSettings, BrowserEntry, BrowserRoot, EngineStatus, ExportOptions,
    FlpImportOptions, FlpImportPreview, RealtimeFrame, SampleInfo, TransportPatch, TransportState,
};
use windfall_project::{
    AutomationTarget, ChannelId, Command, DispatchResult, DocumentSnapshot, PlaylistTrackId,
    ProjectPatch, SampleId, TrackId,
};

use crate::session::{ClipPlace, Session};

/// Runs slow work off the async runtime's own threads.
async fn blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| format!("The task stopped unexpectedly: {error}"))?
}

#[tauri::command]
fn document_snapshot(session: State<'_, Session>) -> DocumentSnapshot {
    session.document_snapshot()
}

#[tauri::command]
fn dispatch(
    session: State<'_, Session>,
    command: Command,
    gesture: Option<u64>,
) -> Result<DispatchResult, String> {
    session.dispatch(command, gesture)
}

#[tauri::command]
fn automate(
    session: State<'_, Session>,
    target: AutomationTarget,
) -> Result<DispatchResult, String> {
    session.automate(target)
}

#[tauri::command]
fn undo(session: State<'_, Session>) -> Option<ProjectPatch> {
    session.undo()
}

#[tauri::command]
fn redo(session: State<'_, Session>) -> Option<ProjectPatch> {
    session.redo()
}

#[tauri::command]
fn history_jump(session: State<'_, Session>, cursor: u32) -> ProjectPatch {
    session.history_jump(cursor)
}

#[tauri::command]
async fn project_new(session: State<'_, Session>) -> Result<DocumentSnapshot, String> {
    let session = session.inner().clone();
    blocking(move || session.project_new()).await
}

#[tauri::command]
async fn project_open(
    session: State<'_, Session>,
    path: String,
) -> Result<DocumentSnapshot, String> {
    let session = session.inner().clone();
    blocking(move || session.project_open(&path)).await
}

#[tauri::command]
async fn flp_preview(
    session: State<'_, Session>,
    path: String,
    options: FlpImportOptions,
) -> Result<FlpImportPreview, String> {
    let session = session.inner().clone();
    blocking(move || session.flp_preview(&path, &options)).await
}

#[tauri::command]
fn flp_open(session: State<'_, Session>, token: u64) -> Result<DocumentSnapshot, String> {
    session.flp_open(token)
}

#[tauri::command]
fn flp_cancel(session: State<'_, Session>, token: u64) {
    session.flp_cancel(token);
}
#[tauri::command]
async fn project_save(session: State<'_, Session>, path: Option<String>) -> Result<String, String> {
    let session = session.inner().clone();
    blocking(move || session.project_save(path.as_deref())).await
}

#[tauri::command]
async fn recent_projects(session: State<'_, Session>) -> Result<Vec<String>, String> {
    let session = session.inner().clone();
    blocking(move || Ok(session.recent_projects())).await
}

#[tauri::command]
fn transport_play(session: State<'_, Session>) -> Result<TransportState, String> {
    session.transport_play()
}

#[tauri::command]
fn transport_stop(session: State<'_, Session>) -> TransportState {
    session.transport_stop()
}

#[tauri::command]
fn transport_toggle(session: State<'_, Session>) -> Result<TransportState, String> {
    session.transport_toggle()
}

#[tauri::command]
fn transport_seek(session: State<'_, Session>, tick: f64) {
    session.transport_seek(tick);
}

#[tauri::command]
fn transport_set(
    session: State<'_, Session>,
    patch: TransportPatch,
) -> Result<TransportState, String> {
    session.transport_set(patch)
}

#[tauri::command]
fn transport_state(session: State<'_, Session>) -> TransportState {
    session.transport_state()
}

#[tauri::command]
fn realtime_subscribe(
    session: State<'_, Session>,
    window: WebviewWindow,
    channel: Channel<RealtimeFrame>,
) {
    session.subscribe_realtime(
        window.label(),
        Box::new(move |frame| channel.send(frame.clone()).is_ok()),
    );
}

#[tauri::command]
fn engine_status(session: State<'_, Session>) -> EngineStatus {
    session.engine_status()
}

#[tauri::command]
async fn engine_devices(session: State<'_, Session>) -> Result<Vec<AudioHost>, String> {
    let session = session.inner().clone();
    blocking(move || Ok(session.engine_devices())).await
}

#[tauri::command]
async fn engine_settings(session: State<'_, Session>) -> Result<AudioSettings, String> {
    let session = session.inner().clone();
    blocking(move || Ok(session.engine_settings())).await
}

#[tauri::command]
async fn engine_configure(
    session: State<'_, Session>,
    settings: AudioSettings,
) -> Result<EngineStatus, String> {
    let session = session.inner().clone();
    blocking(move || Ok(session.engine_configure(settings))).await
}

#[tauri::command]
fn audition_note_on(session: State<'_, Session>, channel: ChannelId, key: u8, velocity: f32) {
    session.audition_note_on(channel, key, velocity);
}

#[tauri::command]
fn audition_note_off(session: State<'_, Session>, channel: ChannelId, key: u8) {
    session.audition_note_off(channel, key);
}

#[tauri::command]
async fn preview_play(session: State<'_, Session>, path: String) -> Result<(), String> {
    let session = session.inner().clone();
    blocking(move || session.preview_play(&path)).await
}

#[tauri::command]
fn preview_stop(session: State<'_, Session>) {
    session.preview_stop();
}

#[tauri::command]
async fn browser_roots(session: State<'_, Session>) -> Result<Vec<BrowserRoot>, String> {
    let session = session.inner().clone();
    blocking(move || Ok(session.browser_roots())).await
}

#[tauri::command]
async fn browser_add_root(
    session: State<'_, Session>,
    path: String,
) -> Result<Vec<BrowserRoot>, String> {
    let session = session.inner().clone();
    blocking(move || session.browser_add_root(&path)).await
}

#[tauri::command]
async fn browser_remove_root(
    session: State<'_, Session>,
    path: String,
) -> Result<Vec<BrowserRoot>, String> {
    let session = session.inner().clone();
    blocking(move || session.browser_remove_root(&path)).await
}

#[tauri::command]
async fn browser_list(
    session: State<'_, Session>,
    path: String,
) -> Result<Vec<BrowserEntry>, String> {
    let session = session.inner().clone();
    blocking(move || session.browser_list(&path)).await
}

#[tauri::command]
async fn sample_info(session: State<'_, Session>, path: String) -> Result<SampleInfo, String> {
    let session = session.inner().clone();
    blocking(move || session.sample_info(&path)).await
}

#[tauri::command]
async fn sample_info_by_id(
    session: State<'_, Session>,
    sample: SampleId,
) -> Result<SampleInfo, String> {
    let session = session.inner().clone();
    blocking(move || session.sample_info_by_id(sample)).await
}

#[tauri::command]
async fn samples_reload(session: State<'_, Session>) -> Result<u32, String> {
    let session = session.inner().clone();
    blocking(move || Ok(session.samples_reload())).await
}

#[tauri::command]
async fn add_channel_from_file(
    session: State<'_, Session>,
    path: String,
    index: Option<u32>,
) -> Result<DispatchResult, String> {
    let session = session.inner().clone();
    blocking(move || session.add_channel_from_file(&path, index)).await
}

#[tauri::command]
async fn set_channel_sample_from_file(
    session: State<'_, Session>,
    channel: ChannelId,
    path: String,
) -> Result<DispatchResult, String> {
    let session = session.inner().clone();
    blocking(move || session.set_channel_sample_from_file(channel, &path)).await
}

#[tauri::command]
async fn add_audio_clip_from_file(
    session: State<'_, Session>,
    path: String,
    track: Option<PlaylistTrackId>,
    start: u32,
    mixer_track: Option<TrackId>,
) -> Result<DispatchResult, String> {
    let session = session.inner().clone();
    let place = ClipPlace {
        track,
        start,
        mixer_track,
    };
    blocking(move || session.add_audio_clip_from_file(&path, place)).await
}

#[tauri::command]
fn add_audio_clip_from_sample(
    session: State<'_, Session>,
    sample: SampleId,
    track: Option<PlaylistTrackId>,
    start: u32,
    mixer_track: Option<TrackId>,
) -> Result<DispatchResult, String> {
    let place = ClipPlace {
        track,
        start,
        mixer_track,
    };
    session.add_audio_clip_from_sample(sample, place)
}

#[tauri::command]
async fn export_audio(session: State<'_, Session>, options: ExportOptions) -> Result<(), String> {
    let session = session.inner().clone();
    blocking(move || session.export_audio(options)).await
}

#[tauri::command]
fn export_cancel(session: State<'_, Session>) {
    session.export_cancel();
}

/// The handler for every command above.
pub fn handler() -> impl Fn(Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        document_snapshot,
        dispatch,
        automate,
        undo,
        redo,
        history_jump,
        project_new,
        project_open,
        flp_preview,
        flp_open,
        flp_cancel,
        project_save,
        recent_projects,
        transport_play,
        transport_stop,
        transport_toggle,
        transport_seek,
        transport_set,
        transport_state,
        realtime_subscribe,
        engine_status,
        engine_devices,
        engine_settings,
        engine_configure,
        audition_note_on,
        audition_note_off,
        preview_play,
        preview_stop,
        browser_roots,
        browser_add_root,
        browser_remove_root,
        browser_list,
        sample_info,
        sample_info_by_id,
        samples_reload,
        add_channel_from_file,
        set_channel_sample_from_file,
        add_audio_clip_from_file,
        add_audio_clip_from_sample,
        export_audio,
        export_cancel,
    ]
}
