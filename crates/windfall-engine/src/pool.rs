//! Decoded audio the engine can play, looked up by sample id.

use std::collections::HashMap;
use std::sync::Arc;

use windfall_core::AudioBuffer;
use windfall_project::SampleId;

/// The decoded samples of a project.
///
/// The engine never reads files: whoever owns the project decodes each
/// sample and puts it here. Cloning is cheap, because clones share the map
/// until one of them is changed, and the audio itself is always shared.
#[derive(Debug, Clone, Default)]
pub struct SamplePool {
    samples: Arc<HashMap<SampleId, AudioBuffer>>,
}

impl SamplePool {
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

    pub fn contains(&self, id: SampleId) -> bool {
        self.samples.contains_key(&id)
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
}
