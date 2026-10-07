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
