//! Recording finish preparation must stay off-lock and cancellable.
use super::Rig;
use crate::session::recording::{CaptureHandle, Sink};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use windfall_ipc::{RecordingLoopOptions, RecordingSource};
use windfall_project::TickRange;

struct Capture {
    drops: Option<Arc<AtomicUsize>>,
}
impl Drop for Capture {
    fn drop(&mut self) {
        if let Some(drops) = &self.drops {
            drops.fetch_add(1, Ordering::AcqRel);
        }
    }
}
impl CaptureHandle for Capture {
    fn frames(&self) -> u64 {
        480
    }
    fn failed(&self) -> bool {
        false
    }
    fn finish(self: Box<Self>) -> Result<u64, String> {
        Ok(480)
    }
}
fn source(looped: bool) -> RecordingSource {
    RecordingSource {
        host: "Fake".into(),
        device: "Fake".into(),
        left: 0,
        right: None,
        alignment: None,
        monitor: None,
        armed_tracks: None,
        mixer_tap: None,
        loop_recording: looped.then_some(RecordingLoopOptions {
            region: TickRange { start: 0, end: 960 },
        }),
    }
}
fn synthetic(_: RecordingSource, _: u32, mut sink: Sink) -> Result<Box<dyn CaptureHandle>, String> {
    sink(&[0.25; 960])?;
    Ok(Box::new(Capture { drops: None }))
}

#[test]
fn qa_cancel_during_single_and_loop_preparation_rejects_commit_and_releases_finishing() {
    for looped in [false, true] {
        let rig = Rig::new();
        let before = rig.project();
        rig.session
            .recording_start_with(source(looped), 0, None, synthetic)
            .unwrap();
        let hold = rig.session.hold(if looped {
            "recording:prepared"
        } else {
            "import:prepared"
        });
        let finish = rig.session.background(|session| session.recording_stop());
        hold.wait();
        // A slow native preparation cannot retain the recording mutex.
        let owned = rig
            .session
            .inner
            .recording
            .try_lock()
            .expect("preparation releases recording guard");
        assert!(owned.is_none());
        drop(owned);
        assert!(
            rig.session
                .inner
                .recording_finishing
                .load(Ordering::Acquire)
        );
        assert!(rig.session.project_new().is_err());
        assert!(
            rig.session
                .recording_start_with(source(false), 0, None, synthetic)
                .is_err()
        );
        rig.session.recording_cancel();
        assert!(
            rig.session
                .inner
                .recording_finish_cancelled
                .load(Ordering::Acquire)
        );
        // Cancel does not admit a replacement until the old resources retire.
        assert!(
            rig.session
                .recording_start_with(source(false), 0, None, synthetic)
                .is_err()
        );
        hold.release();
        assert!(finish.join().unwrap().is_err());
        assert_eq!(rig.project(), before);
        assert!(
            !rig.session
                .inner
                .recording_finishing
                .load(Ordering::Acquire)
        );
        assert!(
            !rig.session
                .inner
                .recording_finish_cancelled
                .load(Ordering::Acquire)
        );
        assert_eq!(
            std::fs::read_dir(rig.folder.path().join("recordings"))
                .unwrap()
                .count(),
            0
        );
        rig.session
            .recording_start_with(source(false), 0, None, synthetic)
            .unwrap();
        rig.session.recording_cancel();
    }
}

#[test]
fn qa_active_cancel_excludes_replacement_until_off_lock_resource_cleanup_finishes() {
    let rig = Rig::new();
    let before = rig.project();
    let drops = Arc::new(AtomicUsize::new(0));
    let capture_drops = drops.clone();
    rig.session
        .recording_start_with(source(false), 0, None, move |_, _, mut sink| {
            sink(&[0.25; 960])?;
            Ok(Box::new(Capture {
                drops: Some(capture_drops),
            }))
        })
        .unwrap();
    let recording_files = || {
        let mut files = std::fs::read_dir(rig.folder.path().join("recordings"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        files.sort();
        files
    };
    let owned_files = recording_files();
    // Reserving the final WAV and writing its transactional temporary file
    // intentionally owns two paths before finalization or cancellation.
    assert_eq!(owned_files.len(), 2);
    assert_eq!(
        owned_files
            .iter()
            .filter(|path| path.extension().is_some_and(|extension| extension == "wav"))
            .count(),
        1
    );
    assert_eq!(
        owned_files
            .iter()
            .filter(|path| path.extension().is_some_and(|extension| extension == "tmp"))
            .count(),
        1
    );
    let hold = rig.session.hold("recording:cancel-cleanup");
    let cancelled = rig.session.background(|session| session.recording_cancel());
    hold.wait();
    let owned = rig
        .session
        .inner
        .recording
        .try_lock()
        .expect("cancel cleanup releases recording guard");
    assert!(owned.is_none());
    drop(owned);
    assert!(
        rig.session
            .inner
            .recording_finishing
            .load(Ordering::Acquire)
    );
    assert!(rig.session.project_new().is_err());
    assert!(
        rig.session
            .recording_start_with(source(false), 0, None, synthetic)
            .is_err()
    );
    let settings = rig.session.engine_settings();
    assert!(rig.session.engine_configure(settings).error.is_some());
    assert_eq!(recording_files(), owned_files);
    assert_eq!(drops.load(Ordering::Acquire), 0);
    hold.release();
    cancelled.join().unwrap();
    assert_eq!(
        drops.load(Ordering::Acquire),
        1,
        "capture is destroyed once after the cleanup barrier"
    );
    assert_eq!(rig.project(), before);
    assert!(
        !rig.session
            .inner
            .recording_finishing
            .load(Ordering::Acquire)
    );
    assert!(
        !rig.session
            .inner
            .recording_finish_cancelled
            .load(Ordering::Acquire)
    );
    assert_eq!(
        std::fs::read_dir(rig.folder.path().join("recordings"))
            .unwrap()
            .count(),
        0
    );
    rig.session
        .recording_start_with(source(false), 0, None, synthetic)
        .unwrap();
    rig.session.recording_cancel();
    assert_eq!(
        drops.load(Ordering::Acquire),
        1,
        "the earlier capture cannot be destroyed twice"
    );
}

#[test]
fn qa_successful_single_and_loop_finish_preserves_published_audio_and_clears_exclusion() {
    for looped in [false, true] {
        let rig = Rig::new();
        rig.session
            .recording_start_with(source(looped), 0, None, synthetic)
            .unwrap();
        rig.session.recording_stop().unwrap();
        assert!(
            !rig.session
                .inner
                .recording_finishing
                .load(Ordering::Acquire)
        );
        let project = rig.project();
        let sample = project.samples.last().unwrap();
        assert_eq!(
            rig.session.sample_info_by_id(sample.id).unwrap().frames,
            480
        );
        assert_eq!(
            std::fs::read_dir(rig.folder.path().join("recordings"))
                .unwrap()
                .count(),
            1
        );
        rig.session.undo().unwrap();
        rig.session.redo().unwrap();
        assert_eq!(
            rig.session.sample_info_by_id(sample.id).unwrap().frames,
            480
        );
    }
}
