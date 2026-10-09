//! Partition an owned loop capture and publish all chosen passes atomically.
use super::*;
use std::collections::BTreeSet;
use windfall_project::file::sample_path_for;
use windfall_project::{
    ClipContent, ClipInit, Command, MAX_MIXER_SIGNAL_TRACKS, SampleId, TrackId,
};

impl Session {
    pub(super) fn attach_loop_takes(
        &self,
        take: &mut Take,
        selection: Option<RecordingTakeSelection>,
    ) -> Result<DispatchResult, String> {
        let span = take.loop_frames.ok_or("Loop span is unavailable.")?;
        let region = take.loop_region.ok_or("Loop region is unavailable.")?;
        let audio = self.inner.cache.decode(&take.path)?;
        let frames = audio.frames() as u64;
        let total = (frames as f64 / span).ceil() as u32;
        if total > 256 {
            return Err(
                "Loop recording exceeded 256 takes; choose a longer region for this session."
                    .into(),
            );
        }
        let selected: BTreeSet<u32> = match selection.unwrap_or(RecordingTakeSelection::All) {
            RecordingTakeSelection::All => (0..total).collect(),
            RecordingTakeSelection::Latest => std::iter::once(total.saturating_sub(1)).collect(),
            RecordingTakeSelection::Only { indices } => indices.into_iter().collect(),
            RecordingTakeSelection::Except { indices } => {
                let excluded: BTreeSet<u32> = indices.into_iter().collect();
                (0..total)
                    .filter(|index| !excluded.contains(index))
                    .collect()
            }
        };
        if selected.is_empty() || selected.iter().any(|index| *index >= total) {
            return Err("Recording take selection is outside this capture.".into());
        }
        let folder = take
            .path
            .parent()
            .ok_or("Recording folder is unavailable.")?
            .to_owned();
        let mut imports = Vec::with_capacity(selected.len());
        for index in selected {
            let first = take_boundary(index, span);
            let last = take_boundary(index + 1, span).min(frames);
            if first >= last {
                continue;
            }
            let file = reserve_take(&folder, index)?;
            // Register ownership before opening/writing so any later failure
            // removes this exact newly reserved file too.
            take.split_paths.push(file.clone());
            let mut writer = WavWriter::create(&file, take.rate, 2, WavSampleFormat::Float32)
                .map_err(|e| e.to_string())?;
            let samples = &audio.samples()[first as usize * 2..last as usize * 2];
            for block in samples.chunks(16_384) {
                writer.write(block).map_err(|e| e.to_string())?;
            }
            writer.finalize().map_err(|e| e.to_string())?;
            let buffer = self.inner.cache.decode(&file)?;
            let length = if last == take_boundary(index + 1, span) {
                region.end - region.start
            } else {
                (self
                    .controller()
                    .song_tick_after_seconds(region.start, buffer.duration_secs())
                    - f64::from(region.start))
                .ceil()
                .max(1.0) as u32
            }
            .min(region.end - region.start);
            imports.push((index, file, buffer, length));
        }
        let ticket = {
            let _recording = self.recording_finish_guard()?;
            let state = self.state();
            let project = state.document.project();
            let mut next = project.next_id;
            let mut allocate = || {
                let id = next;
                next = next.saturating_add(1);
                id
            };
            let mut commands = Vec::new();
            let mixer = take.place.mixer_track.unwrap_or_else(|| {
                if project
                    .mixer
                    .tracks
                    .iter()
                    .filter(|track| !track.current)
                    .count()
                    < MAX_MIXER_SIGNAL_TRACKS
                {
                    let id = TrackId(allocate());
                    commands.push(Command::AddMixerTrack {
                        name: Some("Recorded takes".into()),
                    });
                    id
                } else {
                    TrackId::MASTER
                }
            });
            let mut sources = Vec::new();
            let mut retained = Vec::new();
            let latest = imports.last().map(|(index, ..)| *index);
            for (index, file, buffer, length) in imports {
                let sample = SampleId(allocate());
                let name = format!("Recording take {}", index + 1);
                commands.push(Command::AddSample {
                    name: name.clone(),
                    path: sample_path_for(&file, state.project_dir(), &self.inner.factory_dir),
                });
                let track = if let Some(track) = take.place.track.filter(|_| Some(index) == latest)
                {
                    track
                } else {
                    let track = PlaylistTrackId(allocate());
                    commands.push(Command::AddPlaylistTrack {
                        name: Some(name),
                        index: None,
                    });
                    track
                };
                commands.push(Command::AddClips {
                    clips: vec![ClipInit {
                        track,
                        start: region.start,
                        length: Some(length),
                        offset: None,
                        muted: Some(Some(index) != latest),
                        content: ClipContent::Audio {
                            sample,
                            mixer_track: mixer,
                            output: Default::default(),
                            normalize: false,
                            gain: 1.0,
                            pan: 0.0,
                            fade_in: 0,
                            fade_out: 0,
                            reverse: false,
                            pitch: 0.0,
                            stretch: Default::default(),
                        },
                    }],
                });
                let clip = windfall_project::ClipId(allocate());
                retained.push(windfall_project::AudioTakeRef {
                    pass: index as u16,
                    clip,
                });
                sources.push((sample, buffer));
            }
            commands.push(Command::CreateAudioTakeGroup {
                name: "Loop recording".into(),
                lanes: vec![windfall_project::AudioTakeLane {
                    name: "Recorded input".into(),
                    takes: retained,
                }],
            });
            self.sample_edit_ticket(
                &state,
                Command::Batch {
                    label: Some("Keep recording takes".into()),
                    commands,
                },
                None,
                sources,
            )?
            .on_install(take.installed.clone())
        };
        let mut prepared = ticket.prepare()?;
        let mut retirement = None;
        #[cfg(test)]
        self.pause("recording:prepared");
        let result = {
            let _recording = self.recording_finish_guard()?;
            prepared.commit(&mut self.state(), &mut retirement)
        };
        if let Some(retirement) = &mut retirement {
            self.retire_project(retirement);
        }
        result
    }
}
pub(super) fn reserve_take(folder: &std::path::Path, index: u32) -> Result<PathBuf, String> {
    for _ in 0..1000 {
        let file = folder.join(format!(
            "Take-{}-{}-pass-{}.wav",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed),
            index + 1
        ));
        match OpenOptions::new().write(true).create_new(true).open(&file) {
            Ok(_) => return Ok(file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("Could not reserve a recording take file.".into())
}
