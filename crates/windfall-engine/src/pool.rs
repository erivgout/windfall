//! Decoded audio the engine can play, looked up by sample id.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::clip_processing::{ClipAudioCache, render};
use windfall_core::AudioBuffer;
use windfall_project::{ClipStretch, SampleId};

/// The decoded samples of a project.
///
/// The engine never reads files: whoever owns the project decodes each
/// sample and puts it here. Cloning is cheap, because clones share the map
/// until one of them is changed, and the audio itself is always shared.
#[derive(Debug, Clone, Default)]
pub struct SamplePool {
    samples: Arc<HashMap<SampleId, AudioBuffer>>,
    clip_audio: Arc<Mutex<ClipAudioCache>>,
    pub(crate) plugin_factory: Option<Arc<dyn crate::plugins::PluginFactory>>,
}

impl SamplePool {
    /// Installs the control-side native plugin provider used by playback and render.
    pub fn set_plugin_factory(&mut self, factory: Arc<dyn crate::plugins::PluginFactory>) {
        self.plugin_factory = Some(factory);
    }
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces a sample and returns the one it replaced.
    pub fn insert(&mut self, id: SampleId, buffer: AudioBuffer) -> Option<AudioBuffer> {
        Arc::make_mut(&mut self.samples).insert(id, buffer)
    }

    pub fn remove(&mut self, id: SampleId) -> Option<AudioBuffer> {
        Arc::make_mut(&mut self.samples).remove(&id)
    }

    pub fn get(&self, id: SampleId) -> Option<&AudioBuffer> {
        self.samples.get(&id)
    }

    /// Returns cached spectral audio for a playlist clip, preparing it if needed.
    /// Allocates and performs DSP on a cache miss: call only on control/worker threads.
    /// Clones share this bounded cache; inserting/reloading a sample invalidates by source identity.
    pub fn clip_audio(
        &self,
        id: SampleId,
        settings: ClipStretch,
        pitch: f32,
    ) -> Option<AudioBuffer> {
        let source = self.get(id)?;
        if matches!(settings, ClipStretch::Tape) {
            return Some(source.clone());
        }
        if let Some(buffer) = self
            .clip_audio
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(source, settings, pitch)
        {
            return Some(buffer);
        }
        let rendered = render(source, settings, pitch);
        self.clip_audio
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(source.clone(), settings, pitch, rendered.clone());
        Some(rendered)
    }

    /// Whether compiling this project would perform slow spectral work.
    /// Missing sources stay silent and do not require preparation.
    pub fn needs_clip_preparation(&self, project: &windfall_project::Project) -> bool {
        let cache = self.clip_audio.lock().unwrap_or_else(|e| e.into_inner());
        project.playlist.clips.iter().any(|clip| {
            let windfall_project::ClipContent::Audio {
                sample,
                stretch,
                pitch,
                ..
            } = clip.content
            else {
                return false;
            };
            !matches!(stretch, ClipStretch::Tape)
                && self
                    .get(sample)
                    .is_some_and(|source| cache.get(source, stretch, pitch).is_none())
        })
    }

    /// Freezes the available rendered variants for compilation under a caller lock.
    /// Returns None if DSP would be needed. Concurrent worker cache eviction cannot
    /// turn a hit in this snapshot into a miss while it is being compiled.
    pub fn cached_clip_pool(&self, project: &windfall_project::Project) -> Option<Self> {
        let cache = self.clip_audio.lock().unwrap_or_else(|e| e.into_inner());
        for clip in &project.playlist.clips {
            if let windfall_project::ClipContent::Audio {
                sample,
                stretch,
                pitch,
                ..
            } = clip.content
                && !matches!(stretch, ClipStretch::Tape)
                && let Some(source) = self.get(sample)
                && cache.get(source, stretch, pitch).is_none()
            {
                return None;
            }
        }
        let mut snapshot = self.clone();
        snapshot.clip_audio = Arc::new(Mutex::new(cache.clone()));
        Some(snapshot)
    }

    pub fn contains(&self, id: SampleId) -> bool {
        self.samples.contains_key(&id)
    }

    /// The ids of every sample in the pool, in no particular order.
    pub fn ids(&self) -> impl Iterator<Item = SampleId> + '_ {
        self.samples.keys().copied()
    }

    /// Every sample in the pool with its id, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (SampleId, &AudioBuffer)> + '_ {
        self.samples.iter().map(|(id, buffer)| (*id, buffer))
    }

    /// Removes every sample `keep` returns false for.
    pub fn retain(&mut self, mut keep: impl FnMut(SampleId) -> bool) {
        if self.samples.keys().all(|id| keep(*id)) {
            // Nothing goes, so clones keep sharing the map.
            return;
        }
        Arc::make_mut(&mut self.samples).retain(|id, _| keep(*id));
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_do_not_see_later_changes() {
        let mut pool = SamplePool::new();
        pool.insert(
            SampleId(1),
            AudioBuffer::from_interleaved(48_000, 1, vec![0.5]),
        );
        let snapshot = pool.clone();
        pool.remove(SampleId(1));
        pool.insert(
            SampleId(2),
            AudioBuffer::from_interleaved(48_000, 1, vec![0.1]),
        );

        assert!(snapshot.contains(SampleId(1)));
        assert!(!snapshot.contains(SampleId(2)));
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(SampleId(2)).map(AudioBuffer::frames), Some(1));
    }

    #[test]
    fn a_pool_lists_and_prunes_its_samples() {
        let mut pool = SamplePool::new();
        for id in [3, 1, 2] {
            pool.insert(
                SampleId(id),
                AudioBuffer::from_interleaved(48_000, 1, vec![id as f32]),
            );
        }
        let mut ids: Vec<u32> = pool.ids().map(|id| id.0).collect();
        ids.sort_unstable();
        assert_eq!(ids, [1, 2, 3]);
        assert!(
            pool.iter()
                .all(|(id, buffer)| buffer.samples() == [id.0 as f32])
        );

        let snapshot = pool.clone();
        pool.retain(|id| id != SampleId(2));
        assert_eq!(pool.len(), 2);
        assert!(!pool.contains(SampleId(2)));
        assert_eq!(snapshot.len(), 3);
    }
}

#[cfg(test)]
mod clip_processing_tests {
    use super::*;
    use windfall_project::ClipStretchQuality;
    #[test]
    fn prepared_audio_has_exact_length_independent_pitch_shared_cache_and_reload_identity() {
        let id = SampleId(71);
        let mut pool = SamplePool::new();
        let source = AudioBuffer::from_interleaved(
            48_000,
            1,
            (0..48_000)
                .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.2)
                .collect(),
        );
        pool.insert(id, source.clone());
        let setting = ClipStretch::Spectral {
            ratio: 1.5,
            quality: ClipStretchQuality::Standard,
            formants: false,
        };
        let started = std::time::Instant::now();
        let prepared = pool.clip_audio(id, setting, 12.0).unwrap();
        let preparation = started.elapsed();
        assert_eq!(prepared.frames(), 72_000);
        let center = &prepared.samples()[24_000..48_000];
        let crossings = center
            .windows(2)
            .filter(|p| p[0] <= 0.0 && p[1] > 0.0)
            .count();
        let frequency = crossings as f64 * 48_000.0 / center.len() as f64;
        eprintln!(
            "spectral cache validation: {} frames, {frequency:.2} Hz, worker preparation {preparation:?}",
            prepared.frames()
        );
        assert!((frequency - 880.0).abs() < 4.0, "frequency {frequency}");
        assert_eq!(pool.get(id).unwrap().samples(), source.samples());
        let cached = pool.clone().clip_audio(id, setting, 12.0).unwrap();
        assert_eq!(prepared.samples().as_ptr(), cached.samples().as_ptr());
        pool.insert(
            id,
            AudioBuffer::from_interleaved(48_000, 1, vec![0.0; 48_000]),
        );
        let reloaded = pool.clip_audio(id, setting, 12.0).unwrap();
        assert_ne!(prepared.samples().as_ptr(), reloaded.samples().as_ptr());
        assert!(reloaded.samples().iter().all(|s| *s == 0.0));
    }
}
