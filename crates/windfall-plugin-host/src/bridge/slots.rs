//! Bounded ownership of atomic payload words. No shared Rust object layout.

use super::protocol::*;
use crate::{HostEvent, Transport};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

/// A mapping owner exposes aligned atomic words and keeps them mapped until
/// every endpoint has been retired on a control thread.
pub trait WordStorage: Send + Sync {
    fn words(&self) -> &[AtomicU32];
}

/// Local backing for deterministic protocol/adapter tests, never crash isolation.
pub struct LocalWords(Box<[AtomicU32]>);
impl LocalWords {
    pub fn new() -> Arc<Self> {
        Arc::new(Self((0..REGION_WORDS).map(|_| AtomicU32::new(0)).collect()))
    }
}
impl WordStorage for LocalWords {
    fn words(&self) -> &[AtomicU32] {
        &self.0
    }
}

#[derive(Clone)]
pub struct Region {
    storage: Arc<dyn WordStorage>,
    pub config: Config,
}
impl Region {
    /// Control-side initialization before either endpoint is installed.
    pub fn initialize(
        storage: Arc<dyn WordStorage>,
        config: Config,
    ) -> Result<Self, ProtocolError> {
        if storage.words().len() != REGION_WORDS {
            return Err(ProtocolError::Layout);
        }
        let header = config.header()?;
        // Pre-touch every page before installation, including otherwise unused
        // slots. OS residency remains outside a headless deadline guarantee.
        for cell in storage.words() {
            cell.store(0, Ordering::Relaxed);
        }
        for (cell, word) in storage.words()[..HEADER_WORDS].iter().zip(header) {
            cell.store(word.to_le(), Ordering::Relaxed);
        }
        Ok(Self { storage, config })
    }
    /// Check the entire exact header before loading native code.
    pub fn attach(storage: Arc<dyn WordStorage>, expected: Config) -> Result<Self, ProtocolError> {
        if storage.words().len() != REGION_WORDS {
            return Err(ProtocolError::Layout);
        }
        let header = std::array::from_fn::<_, HEADER_WORDS, _>(|index| {
            u32::from_le(storage.words()[index].load(Ordering::Relaxed))
        });
        let config = Config::from_header(&header, storage.words().len() * 4)?;
        if config != expected {
            return Err(ProtocolError::Identity);
        }
        Ok(Self { storage, config })
    }
    /// Startup negotiation only; neither endpoint may process yet.
    pub fn negotiate_latency(&mut self, native_latency: usize) -> Result<(), ProtocolError> {
        let config = Config {
            native_latency,
            ..self.config
        }
        .validate()?;
        self.storage.words()[11].store((native_latency as u32).to_le(), Ordering::Release);
        self.config = config;
        Ok(())
    }
    fn offset(slot: usize, offset: usize) -> usize {
        HEADER_WORDS + slot * SLOT_WORDS + offset
    }
    fn get(&self, slot: usize, offset: usize) -> u32 {
        u32::from_le(self.storage.words()[Self::offset(slot, offset)].load(Ordering::Relaxed))
    }
    fn put(&self, slot: usize, offset: usize, value: u32) {
        self.storage.words()[Self::offset(slot, offset)].store(value.to_le(), Ordering::Relaxed);
    }
    fn state(&self, slot: usize) -> &AtomicU32 {
        &self.storage.words()[Self::offset(slot, STATE)]
    }
    fn claim(&self, slot: usize, before: u32, after: u32) -> bool {
        self.state(slot)
            .compare_exchange(
                before.to_le(),
                after.to_le(),
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_ok()
    }
    fn publish(&self, slot: usize, state: u32) {
        self.state(slot).store(state.to_le(), Ordering::Release);
    }
    fn state_value(&self, slot: usize) -> u32 {
        u32::from_le(self.state(slot).load(Ordering::Acquire))
    }
    fn read_identity(&self, slot: usize, offset: usize) -> Result<Identity, ProtocolError> {
        Identity::decode(std::array::from_fn(|index| self.get(slot, offset + index)))
    }
    fn write_identity(&self, slot: usize, offset: usize) {
        for (index, word) in self.config.identity.encode().iter().enumerate() {
            self.put(slot, offset + index, *word);
        }
    }
    fn sequence(&self, slot: usize, offset: usize) -> u64 {
        pair(self.get(slot, offset), self.get(slot, offset + 1))
    }
    fn write_sequence(&self, slot: usize, offset: usize, sequence: u64) {
        self.put(slot, offset, sequence as u32);
        self.put(slot, offset + 1, (sequence >> 32) as u32);
    }
    /// The host can recycle expired pending/completed slots, never a live
    /// helper-owned one. Exactly four single-attempt CAS operations.
    pub fn discard_before(&self, sequence: u64) {
        for slot in 0..SLOT_COUNT {
            let state = self.state_value(slot);
            if matches!(state, READY | DONE)
                && self.sequence(slot, SEQUENCE) < sequence
                && self.claim(slot, state, HOST_WRITE)
            {
                self.publish(slot, EMPTY);
            }
        }
    }
    pub fn submit(&self, sequence: u64, block: &InputBlock) -> bool {
        let Some(slot) = (0..SLOT_COUNT).find(|slot| self.claim(*slot, EMPTY, HOST_WRITE)) else {
            return false;
        };
        self.write_identity(slot, IDENTITY);
        self.write_sequence(slot, SEQUENCE, sequence);
        self.put(slot, FRAMES, self.config.block as u32);
        self.put(slot, EVENT_COUNT, block.event_count as u32);
        self.put(slot, PARAM_COUNT, block.parameter_count as u32);
        self.put(slot, FLAGS, u32::from(!block.controls_complete));
        self.put(slot, NOTE_CHANNEL, 0);
        self.write_sequence(slot, EPOCH, block.epoch);
        self.write_sequence(slot, CONTROL_START, block.control_start);
        self.write_sequence(slot, CONTROL_END, block.control_end);
        // Values were checked on admission; invalid transport is retained locally.
        let transport = encode_transport(block.transport).expect("admitted transport");
        for (index, word) in transport.iter().enumerate() {
            self.put(slot, TRANSPORT + index, *word);
        }
        for index in 0..self.config.block {
            self.put(
                slot,
                INPUT + index,
                finite_input(block.left[index]).to_bits(),
            );
            self.put(
                slot,
                INPUT + MAX_BLOCK + index,
                finite_input(block.right[index]).to_bits(),
            );
        }
        for (index, velocity) in block.notes.iter().enumerate() {
            self.put(slot, NOTES + index, velocity.to_bits());
        }
        for (index, parameter) in block.parameters[..block.parameter_count].iter().enumerate() {
            let words = parameter.encode().expect("admitted parameter");
            for (word, value) in words.iter().enumerate() {
                self.put(slot, PARAMETERS + index * 3 + word, *value);
            }
        }
        for (index, event) in block.events[..block.event_count].iter().enumerate() {
            let words = encode_event(*event, self.config.block).expect("admitted event");
            for (word, value) in words.iter().enumerate() {
                self.put(slot, EVENTS + index * EVENT_WORDS + word, *value);
            }
        }
        self.publish(slot, READY);
        true
    }
    /// Native owner thread claims the oldest ready sequence. No payload is
    /// accessed before the acquire; the native plugin never sees mapped pointers.
    pub fn take_input(
        &self,
        block: &mut InputBlock,
    ) -> Result<Option<(usize, u64)>, ProtocolError> {
        let slot = (0..SLOT_COUNT)
            .filter(|slot| self.state_value(*slot) == READY)
            .min_by_key(|slot| self.sequence(*slot, SEQUENCE));
        let Some(slot) = slot else {
            return Ok(None);
        };
        if !self.claim(slot, READY, HELPER_WRITE) {
            return Ok(None);
        }
        let sequence = self.sequence(slot, SEQUENCE);
        let result = self.read_input(slot, block);
        if let Err(error) = result {
            self.complete(slot, sequence, &OutputBlock::silent(), OUTPUT_FAILED);
            return Err(error);
        }
        Ok(Some((slot, sequence)))
    }
    fn read_input(&self, slot: usize, block: &mut InputBlock) -> Result<(), ProtocolError> {
        if self.read_identity(slot, IDENTITY)? != self.config.identity {
            return Err(ProtocolError::Identity);
        }
        let frames = self.get(slot, FRAMES) as usize;
        let events = self.get(slot, EVENT_COUNT) as usize;
        let params = self.get(slot, PARAM_COUNT) as usize;
        if frames != self.config.block
            || events > EVENT_CAPACITY
            || params > PARAM_CAPACITY
            || self.get(slot, FLAGS) > 1
            || self.get(slot, NOTE_CHANNEL) != 0
        {
            return Err(ProtocolError::Layout);
        }
        block.transport = decode_transport(std::array::from_fn(|index| {
            self.get(slot, TRANSPORT + index)
        }))?;
        for index in 0..frames {
            block.left[index] = f32::from_bits(self.get(slot, INPUT + index));
            block.right[index] = f32::from_bits(self.get(slot, INPUT + MAX_BLOCK + index));
            if !block.left[index].is_finite() || !block.right[index].is_finite() {
                return Err(ProtocolError::Audio);
            }
        }
        for (index, note) in block.notes.iter_mut().enumerate() {
            *note = f32::from_bits(self.get(slot, NOTES + index));
            if !note.is_finite() || !(0.0..=1.0).contains(note) {
                return Err(ProtocolError::Event);
            }
        }
        for index in 0..params {
            block.parameters[index] = Parameter::decode(std::array::from_fn(|word| {
                self.get(slot, PARAMETERS + index * 3 + word)
            }))?;
        }
        for index in 0..events {
            block.events[index] = decode_event(
                std::array::from_fn(|word| self.get(slot, EVENTS + index * EVENT_WORDS + word)),
                frames,
            )?;
            if index > 0 && block.events[index - 1].time() > block.events[index].time() {
                return Err(ProtocolError::Event);
            }
        }
        block.parameter_count = params;
        block.event_count = events;
        block.epoch = self.sequence(slot, EPOCH);
        block.control_start = self.sequence(slot, CONTROL_START);
        block.control_end = self.sequence(slot, CONTROL_END);
        block.controls_complete = self.get(slot, FLAGS) == 0;
        if block.epoch == 0 || block.control_start > block.control_end {
            return Err(ProtocolError::Sequence);
        }
        Ok(())
    }
    /// Only the helper owning this slot calls complete. Failed blocks contain
    /// no usable output; the host validates reply identity and sequence again.
    pub fn complete(&self, slot: usize, sequence: u64, output: &OutputBlock, status: u32) {
        self.write_identity(slot, REPLY_IDENTITY);
        self.write_sequence(slot, REPLY_SEQUENCE, sequence);
        self.put(slot, REPLY_FRAMES, self.config.block as u32);
        self.put(slot, REPLY_STATUS, status);
        self.write_sequence(slot, REPLY_EPOCH, output.epoch);
        self.write_sequence(slot, PROCESSED_GENERATION, output.processed_generation);
        self.put(slot, NATIVE_DROPS, output.native_drops);
        for index in 0..self.config.block {
            self.put(slot, OUTPUT + index, output.left[index].to_bits());
            self.put(
                slot,
                OUTPUT + MAX_BLOCK + index,
                output.right[index].to_bits(),
            );
        }
        self.publish(slot, DONE);
    }
    /// Callback: inspect four slots and copy only one acquired matching reply.
    /// No retries, allocation, clock, locks, wait or process/pipe/file operation.
    pub fn receive(
        &self,
        sequence: u64,
        epoch: u64,
        output: &mut OutputBlock,
    ) -> Result<bool, ProtocolError> {
        for slot in 0..SLOT_COUNT {
            if self.state_value(slot) != DONE || self.sequence(slot, SEQUENCE) != sequence {
                continue;
            }
            if !self.claim(slot, DONE, HOST_READ) {
                continue;
            }
            let result = self.read_output(slot, sequence, epoch, output);
            self.publish(slot, EMPTY);
            return result.map(|()| true);
        }
        Ok(false)
    }
    fn read_output(
        &self,
        slot: usize,
        sequence: u64,
        epoch: u64,
        output: &mut OutputBlock,
    ) -> Result<(), ProtocolError> {
        if self.read_identity(slot, REPLY_IDENTITY)? != self.config.identity {
            return Err(ProtocolError::Identity);
        }
        if self.sequence(slot, REPLY_SEQUENCE) != sequence {
            return Err(ProtocolError::Sequence);
        }
        if self.sequence(slot, REPLY_EPOCH) != epoch {
            return Err(ProtocolError::Sequence);
        }
        if self.get(slot, REPLY_FRAMES) as usize != self.config.block
            || self.get(slot, REPLY_STATUS) != OUTPUT_OK
        {
            return Err(ProtocolError::Audio);
        }
        for index in 0..self.config.block {
            output.left[index] = f32::from_bits(self.get(slot, OUTPUT + index));
            output.right[index] = f32::from_bits(self.get(slot, OUTPUT + MAX_BLOCK + index));
            if !output.left[index].is_finite() || !output.right[index].is_finite() {
                return Err(ProtocolError::Audio);
            }
        }
        // Even a malformed peer changing metadata during copy cannot make a
        // stale sequence or owner acceptable. All payload accesses remain atomic.
        if self.read_identity(slot, REPLY_IDENTITY)? != self.config.identity
            || self.sequence(slot, REPLY_SEQUENCE) != sequence
            || self.sequence(slot, REPLY_EPOCH) != epoch
        {
            return Err(ProtocolError::Identity);
        }
        output.epoch = epoch;
        output.processed_generation = self.sequence(slot, PROCESSED_GENERATION);
        output.native_drops = self.get(slot, NATIVE_DROPS);
        if output.processed_generation > self.sequence(slot, CONTROL_END) {
            return Err(ProtocolError::Sequence);
        }
        Ok(())
    }
    /// Supervisor observation only; performs no ownership transition.
    pub fn busy_sequence(&self) -> Option<u64> {
        (0..SLOT_COUNT)
            .filter(|slot| self.state_value(*slot) == HELPER_WRITE)
            .map(|slot| self.sequence(slot, SEQUENCE))
            .min()
    }
    pub fn pending_sequence(&self) -> Option<u64> {
        (0..SLOT_COUNT)
            .filter(|slot| matches!(self.state_value(*slot), READY | HELPER_WRITE))
            .map(|slot| self.sequence(slot, SEQUENCE))
            .min()
    }
    /// Offline scheduling observation. The callback never waits on this.
    pub fn output_finished(&self, sequence: u64) -> bool {
        (0..SLOT_COUNT)
            .any(|slot| self.state_value(slot) == DONE && self.sequence(slot, SEQUENCE) == sequence)
    }
    pub fn latch_helper_failure(&self) {
        self.storage.words()[HELPER_FAILURE].store(1, Ordering::Release);
    }
    pub fn helper_failed(&self) -> bool {
        self.storage.words()[HELPER_FAILURE].load(Ordering::Acquire) != 0
    }
}

pub fn finite_input(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1024.0, 1024.0)
    } else {
        0.0
    }
}

pub struct InputBlock {
    pub epoch: u64,
    pub control_start: u64,
    pub control_end: u64,
    pub controls_complete: bool,
    pub left: [f32; MAX_BLOCK],
    pub right: [f32; MAX_BLOCK],
    pub transport: Transport,
    pub notes: [f32; 128],
    pub parameters: Box<[Parameter]>,
    pub parameter_count: usize,
    pub events: Box<[HostEvent]>,
    pub event_count: usize,
}
impl InputBlock {
    pub fn new() -> Self {
        Self {
            epoch: 1,
            control_start: 0,
            control_end: 0,
            controls_complete: true,
            left: [0.0; MAX_BLOCK],
            right: [0.0; MAX_BLOCK],
            transport: Transport::default(),
            notes: [0.0; 128],
            parameters: vec![Parameter { id: 0, value: 0.0 }; PARAM_CAPACITY].into_boxed_slice(),
            parameter_count: 0,
            events: vec![HostEvent::AllNotesOff { time: 0 }; EVENT_CAPACITY].into_boxed_slice(),
            event_count: 0,
        }
    }
}
impl Default for InputBlock {
    fn default() -> Self {
        Self::new()
    }
}
pub struct OutputBlock {
    pub epoch: u64,
    pub processed_generation: u64,
    pub native_drops: u32,
    pub left: [f32; MAX_BLOCK],
    pub right: [f32; MAX_BLOCK],
}
impl OutputBlock {
    pub const fn silent() -> Self {
        Self {
            epoch: 1,
            processed_generation: 0,
            native_drops: 0,
            left: [0.0; MAX_BLOCK],
            right: [0.0; MAX_BLOCK],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn region() -> Region {
        Region::initialize(
            LocalWords::new(),
            Config {
                identity: Identity {
                    session: 1,
                    token: 2,
                    revision: 3,
                    binding: 4,
                },
                sample_rate: 48_000,
                block: 64,
                native_latency: 0,
                kind: Kind::Effect,
            },
        )
        .unwrap()
    }
    #[test]
    fn claimed_late_slot_is_not_reused_and_matching_output_roundtrips() {
        let region = region();
        let input = InputBlock::new();
        assert!(region.submit(0, &input));
        let mut worker = InputBlock::new();
        let (slot, sequence) = region.take_input(&mut worker).unwrap().unwrap();
        region.discard_before(100);
        assert_eq!(region.busy_sequence(), Some(0));
        for sequence in 1..4 {
            assert!(region.submit(sequence, &input));
        }
        assert!(!region.submit(4, &input));
        let mut output = OutputBlock::silent();
        output.left[..64].fill(0.5);
        region.complete(slot, sequence, &output, OUTPUT_OK);
        let mut received = OutputBlock::silent();
        assert_eq!(region.receive(1, 1, &mut received), Ok(false));
        assert_eq!(region.receive(0, 1, &mut received), Ok(true));
        assert_eq!(&received.left[..64], &[0.5; 64]);
        assert_eq!(region.receive(0, 1, &mut received), Ok(false));
    }
    #[test]
    fn invalid_tokens_sequences_samples_and_counts_are_refused() {
        for corrupt in [
            REPLY_IDENTITY,
            REPLY_SEQUENCE,
            REPLY_FRAMES,
            REPLY_STATUS,
            OUTPUT,
        ] {
            let region = region();
            assert!(region.submit(7, &InputBlock::new()));
            let (slot, sequence) = region.take_input(&mut InputBlock::new()).unwrap().unwrap();
            region.complete(slot, sequence, &OutputBlock::silent(), OUTPUT_OK);
            region.put(
                slot,
                corrupt,
                if corrupt == OUTPUT {
                    f32::NAN.to_bits()
                } else {
                    u32::MAX
                },
            );
            assert!(
                region.receive(7, 1, &mut OutputBlock::silent()).is_err(),
                "offset {corrupt}"
            );
            assert_eq!(region.state_value(slot), EMPTY);
        }
        let region = region();
        assert!(region.submit(0, &InputBlock::new()));
        region.put(0, EVENT_COUNT, u32::MAX);
        assert!(region.take_input(&mut InputBlock::new()).is_err());
        assert_eq!(region.state_value(0), DONE);
    }
}
