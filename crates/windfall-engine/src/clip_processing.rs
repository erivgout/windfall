//! Worker/control-side rendered clip audio. Never called by the audio callback.
use std::collections::VecDeque;
use windfall_core::AudioBuffer;
use windfall_project::{ClipStretch, ClipStretchQuality};
use windfall_stretch::{Quality, stretch_with_formants};

/// A bounded cache retains source identity as well as prepared audio, preventing
/// pointer reuse from making a reloaded file match an older rendering.
#[derive(Debug, Clone, Default)]
pub(crate) struct ClipAudioCache {
    entries: VecDeque<Entry>,
    bytes: usize,
}

#[derive(Debug, Clone)]
struct Entry {
    source: AudioBuffer,
    settings: ClipStretch,
    pitch: f32,
    rendered: AudioBuffer,
}

impl ClipAudioCache {
    pub fn get(
        &self,
        source: &AudioBuffer,
        settings: ClipStretch,
        pitch: f32,
    ) -> Option<AudioBuffer> {
        self.entries
            .iter()
            .find(|entry| {
                entry.settings == settings
                    && entry.pitch == pitch
                    && entry.source.samples().as_ptr() == source.samples().as_ptr()
                    && entry.source.samples().len() == source.samples().len()
            })
            .map(|entry| entry.rendered.clone())
    }

    pub fn insert(
        &mut self,
        source: AudioBuffer,
        settings: ClipStretch,
        pitch: f32,
        rendered: AudioBuffer,
    ) {
        let bytes = rendered.samples().len().saturating_mul(4);
        // Keep at most 32 variants / 256 MiB, except a single larger clip.
        while !self.entries.is_empty()
            && (self.entries.len() >= 32 || self.bytes.saturating_add(bytes) > 256 * 1024 * 1024)
        {
            if let Some(old) = self.entries.pop_front() {
                self.bytes -= old.rendered.samples().len() * 4;
            }
        }
        self.bytes += bytes;
        self.entries.push_back(Entry {
            source,
            settings,
            pitch,
            rendered,
        });
    }
}

pub(crate) fn render(source: &AudioBuffer, settings: ClipStretch, pitch: f32) -> AudioBuffer {
    let ClipStretch::Spectral {
        ratio,
        quality,
        formants,
    } = settings
    else {
        return source.clone();
    };
    let quality = match quality {
        ClipStretchQuality::Fast => Quality::Fast,
        ClipStretchQuality::Standard => Quality::Standard,
        ClipStretchQuality::High => Quality::High,
    };
    stretch_with_formants(source, ratio, f64::from(pitch), quality, formants)
}
