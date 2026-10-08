//! One owned take at a time. A WAV is streamed outside the audio callback,
//! then imported through the ordinary sample/clip command pipeline.
use super::{ClipPlace, Session};
use crate::sync::lock;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use windfall_codec::{WavSampleFormat, WavWriter};
use windfall_engine::recording::{Capture, recording_inputs};
use windfall_ipc::{RecordingInput, RecordingSource, RecordingState};
use windfall_project::{DispatchResult, MAX_SONG_TICKS, PlaylistTrackId};

pub(super) trait CaptureHandle: Send {
    fn frames(&self) -> u64;
    fn failed(&self) -> bool;
    fn finish(self: Box<Self>) -> Result<u64, String>;
}
impl CaptureHandle for Capture {
    fn frames(&self) -> u64 {
        Capture::frames(self)
    }
    fn failed(&self) -> bool {
        Capture::failed(self)
    }
    fn finish(self: Box<Self>) -> Result<u64, String> {
        Capture::finish(*self)
    }
}
pub(super) type Sink = Box<dyn FnMut(&[f32]) -> Result<(), String> + Send>;
static SERIAL: AtomicU64 = AtomicU64::new(0);
pub(super) struct Take {
    capture: Option<Box<dyn CaptureHandle>>,
    writer: Arc<Mutex<Option<WavWriter>>>,
    path: PathBuf,
    place: ClipPlace,
    rate: u32,
    installed: Arc<std::sync::atomic::AtomicBool>,
}
impl Drop for Take {
    fn drop(&mut self) {
        self.capture.take();
        lock(&self.writer).take();
        if !self.installed.load(Ordering::Acquire) {
            let _ = fs::remove_file(&self.path);
        }
    }
}
struct Finishing<'a>(&'a std::sync::atomic::AtomicBool);
impl Drop for Finishing<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl Session {
    pub(super) fn recording_idle(&self) -> Result<MutexGuard<'_, Option<Take>>, String> {
        let take = lock(&self.inner.recording);
        if take.is_some() || self.inner.recording_finishing.load(Ordering::Acquire) {
            return Err("Stop or cancel recording first.".into());
        }
        Ok(take)
    }
    pub fn recording_inputs(&self) -> Vec<RecordingInput> {
        recording_inputs()
    }
    pub fn recording_state(&self) -> RecordingState {
        let take = lock(&self.inner.recording);
        match take.as_ref() {
            Some(take) => RecordingState {
                active: true,
                frames: take.capture.as_ref().map_or(0, |capture| capture.frames()),
                sample_rate: take.rate,
                start_tick: take.place.start,
                error: take
                    .capture
                    .as_ref()
                    .filter(|c| c.failed())
                    .map(|_| "Input failed. Stop to discard this take.".into()),
            },
            None => RecordingState {
                active: false,
                frames: 0,
                sample_rate: 0,
                start_tick: 0,
                error: None,
            },
        }
    }
    /// Capture starts at the first input callback after opening. Placement is
    /// explicitly chosen by the user; independent hardware clocks and driver
    /// input/output latency are not compensated in this initial workflow.
    pub fn recording_start(
        &self,
        source: RecordingSource,
        start: u32,
        track: Option<PlaylistTrackId>,
    ) -> Result<RecordingState, String> {
        self.recording_start_with(source, start, track, |source, rate, sink| {
            Ok(Box::new(Capture::start(source, rate, sink)?))
        })
    }
    pub(super) fn recording_start_with<F>(
        &self,
        source: RecordingSource,
        start: u32,
        track: Option<PlaylistTrackId>,
        open: F,
    ) -> Result<RecordingState, String>
    where
        F: FnOnce(RecordingSource, u32, Sink) -> Result<Box<dyn CaptureHandle>, String>,
    {
        let _configuring = lock(&self.inner.configuring);
        let _saving = lock(&self.inner.save);
        let mut owned = self.recording_idle()?;
        if self.inner.exporting.load(Ordering::Acquire) {
            return Err("Wait for export before recording.".into());
        }
        let status = self.engine_status();
        if !status.running || status.sample_rate == 0 {
            return Err("Open an audio output before recording.".into());
        }
        if start >= MAX_SONG_TICKS {
            return Err("Recording start is outside the song.".into());
        }
        {
            let state = self.state();
            if track.is_some_and(|id| {
                !state
                    .document
                    .project()
                    .playlist
                    .tracks
                    .iter()
                    .any(|t| t.id == id)
            }) {
                return Err("Playlist track no longer exists.".into());
            }
        }
        let folder = self.store().recordings_dir();
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let mut path = None;
        for _ in 0..1000 {
            let candidate = folder.join(format!(
                "Take-{}-{}.wav",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(_) => {
                    path = Some(candidate);
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        let path = path.ok_or("Could not reserve a unique recording file.")?;
        let writer = match WavWriter::create(&path, status.sample_rate, 2, WavSampleFormat::Float32)
        {
            Ok(writer) => Arc::new(Mutex::new(Some(writer))),
            Err(e) => {
                let _ = fs::remove_file(&path);
                return Err(e.to_string());
            }
        };
        let mut take = Take {
            capture: None,
            writer: writer.clone(),
            path,
            place: ClipPlace {
                start,
                track,
                mixer_track: None,
            },
            rate: status.sample_rate,
            installed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        take.capture = Some(open(
            source,
            status.sample_rate,
            Box::new(move |block| {
                lock(&writer)
                    .as_mut()
                    .ok_or("Recording writer closed.")?
                    .write(block)
                    .map_err(|e| e.to_string())
            }),
        )?);
        *owned = Some(take);
        drop(owned);
        Ok(self.recording_state())
    }
    pub fn recording_cancel(&self) {
        let take = lock(&self.inner.recording).take();
        drop(take);
    }
    pub fn recording_stop(&self) -> Result<DispatchResult, String> {
        let _finishing;
        let mut owned = lock(&self.inner.recording);
        let mut take = owned.take().ok_or("No recording is active.")?;
        // Exclude a second take/ordinary edits, but do not hold a mutex during
        // source/native preparation or helper/input destruction.
        self.inner
            .recording_finishing
            .store(true, Ordering::Release);
        _finishing = Finishing(&self.inner.recording_finishing);
        drop(owned);
        let frames = take
            .capture
            .take()
            .ok_or("Capture is unavailable.")?
            .finish()?;
        if frames == 0 {
            return Err("The take contains no audio.".into());
        }
        lock(&take.writer)
            .take()
            .ok_or("Recording writer closed.")?
            .finalize()
            .map_err(|e| e.to_string())?;
        let result = self.attach_audio_clip_from_file(
            &crate::paths::display(&take.path),
            take.place,
            take.installed.clone(),
        )?;
        // Completed source stays on disk for undo/redo and future project saves.
        take.path = PathBuf::new();
        Ok(result)
    }
}
