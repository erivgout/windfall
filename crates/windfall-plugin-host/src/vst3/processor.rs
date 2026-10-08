//! Preallocated VST3 audio-thread data. Controller calls never happen here.
use super::{
    buffers::{Changes, Events},
    handlers::AudioCall,
    instance::{Objects, Value},
};
use crate::{
    descriptor::{AudioPort, PluginLayout},
    error::PluginError,
    events::{HostEvent, PluginEvent, Transport},
    processor::{AudioIo, BlockResult, ProcessFailed, ProcessStatus, ProcessorBackend},
};
use std::{any::Any, sync::Arc};
use vst3::{
    ComPtr, ComWrapper,
    Steinberg::{Vst::*, *},
};

struct Ports {
    buffers: Vec<Vec<Vec<f32>>>,
    _pointers: Vec<Vec<*mut f32>>,
    buses: Vec<AudioBusBuffers>,
    main: Option<usize>,
}

fn process_context(
    transport: &Transport,
    sample_rate: f64,
    steady_time: u64,
) -> Result<ProcessContext, ProcessFailed> {
    let (bar_start, _) = transport.bar_position().ok_or(ProcessFailed)?;
    // SAFETY: ProcessContext is POD, only flagged fields are consumed.
    let mut context: ProcessContext = unsafe { std::mem::zeroed() };
    context.state = 512 | 1024 | 2048 | 8192 | 131072 | if transport.playing { 2 } else { 0 };
    context.sampleRate = sample_rate;
    context.projectTimeSamples = (transport.position_seconds * sample_rate) as i64;
    context.continousTimeSamples = steady_time.min(i64::MAX as u64) as i64;
    context.projectTimeMusic = transport.position_beats;
    context.tempo = transport.tempo_bpm;
    context.timeSigNumerator = transport.numerator.into();
    context.timeSigDenominator = transport.denominator.into();
    context.barPositionMusic = bar_start;
    Ok(context)
}
impl Ports {
    fn new(ports: &[AudioPort], max_block: usize) -> Self {
        let mut buffers: Vec<Vec<Vec<f32>>> = ports
            .iter()
            .map(|p| (0..p.channels).map(|_| vec![0.0; max_block]).collect())
            .collect();
        let mut pointers: Vec<Vec<*mut f32>> = buffers
            .iter_mut()
            .map(|p| p.iter_mut().map(|b| b.as_mut_ptr()).collect())
            .collect();
        let buses = pointers
            .iter_mut()
            .map(|p| AudioBusBuffers {
                numChannels: p.len() as i32,
                silenceFlags: 0,
                __field0: AudioBusBuffers__type0 {
                    channelBuffers32: p.as_mut_ptr(),
                },
            })
            .collect();
        Self {
            buffers,
            _pointers: pointers,
            buses,
            main: ports.iter().position(|p| p.main && p.channels > 0),
        }
    }
    fn clear(&mut self, frames: usize) {
        for (index, bus) in self.buffers.iter_mut().enumerate() {
            for (channel_index, channel) in bus.iter_mut().enumerate() {
                channel[..frames].fill(0.0);
                self._pointers[index][channel_index] = channel.as_mut_ptr();
            }
            self.buses[index].numChannels = bus.len() as i32;
            self.buses[index].__field0.channelBuffers32 = self._pointers[index].as_mut_ptr();
            self.buses[index].silenceFlags = 0;
        }
    }
    fn input(&mut self, left: &[f32], right: &[f32]) {
        if let Some(main) = self.main {
            let bus = &mut self.buffers[main];
            if bus.len() == 1 {
                for ((out, l), r) in bus[0].iter_mut().zip(left).zip(right) {
                    *out = (*l + *r) * 0.5;
                }
            } else {
                bus[0][..left.len()].copy_from_slice(left);
                bus[1][..right.len()].copy_from_slice(right);
            }
        }
    }
    fn output(&self, left: &mut [f32], right: &mut [f32]) {
        if let Some(main) = self.main {
            let bus = &self.buffers[main];
            left.copy_from_slice(&bus[0][..left.len()]);
            right.copy_from_slice(&bus[usize::from(bus.len() > 1)][..right.len()]);
        } else {
            left.fill(0.0);
            right.fill(0.0);
        }
    }
}
pub(super) struct VstProcessor {
    objects: Arc<Objects>,
    values: Arc<[Value]>,
    inputs: Ports,
    outputs: Ports,
    parameters: ComWrapper<Changes>,
    output_parameters: ComWrapper<Changes>,
    parameter_ptr: ComPtr<IParameterChanges>,
    output_parameter_ptr: ComPtr<IParameterChanges>,
    events: ComWrapper<Events>,
    output_events: ComWrapper<Events>,
    event_ptr: ComPtr<IEventList>,
    output_event_ptr: ComPtr<IEventList>,
    edits: rtrb::Consumer<(u32, f64)>,
    sample_rate: f64,
    started: bool,
    reset_notes: bool,
    held: [[bool; 128]; 16],
    event_input: bool,
}
// SAFETY: all mutable COM buffers are exclusively owned by this processor,
// moved to one audio thread, and used synchronously in process only. Rc in
// Changes shares arenas only between queues of this same exclusive owner.
unsafe impl Send for VstProcessor {}
impl VstProcessor {
    /// Owner-only lifecycle; a refusal retains both native and Rust ownership.
    pub(super) fn quiesce(&mut self) -> Result<(), PluginError> {
        if self.started {
            if unsafe { self.objects.processor.setProcessing(0) } != kResultOk {
                return Err(PluginError::Deactivate(
                    "setProcessing(false) refused".into(),
                ));
            }
            self.started = false;
        }
        Ok(())
    }
    /// The native owner calls this only after exclusive audio ownership returns.
    /// Preserve editor points still queued for process as deferred state overrides.
    pub(super) fn retain_pending_edits(&mut self) {
        for _ in 0..crate::instance::QUEUE_CAPACITY {
            let Ok((id, value)) = self.edits.pop() else {
                break;
            };
            if let Some(v) = self.value(id)
                && v.writable
            {
                v.set(value);
                v.pending.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
    pub fn new(
        objects: Arc<Objects>,
        layout: &PluginLayout,
        values: Arc<[Value]>,
        sample_rate: f64,
        max_block: u32,
        edits: rtrb::Consumer<(u32, f64)>,
    ) -> Result<Self, PluginError> {
        // SAFETY: inactive component on its main thread, bounded bus indices.
        unsafe {
            if objects.processor.canProcessSampleSize(0) != kResultOk {
                return Err(PluginError::Activate(
                    "32-bit audio is not supported".into(),
                ));
            }
            let mut ins = Vec::with_capacity(layout.audio_inputs.len());
            let mut outs = Vec::with_capacity(layout.audio_outputs.len());
            for (direction, count, arrangements) in [
                (0, layout.audio_inputs.len(), &mut ins),
                (1, layout.audio_outputs.len(), &mut outs),
            ] {
                for index in 0..count {
                    let mut arrangement = 0;
                    if objects.processor.getBusArrangement(
                        direction,
                        index as i32,
                        &mut arrangement,
                    ) != kResultOk
                    {
                        return Err(PluginError::Activate("bus arrangement refused".into()));
                    }
                    arrangements.push(arrangement);
                }
            }
            if objects.processor.setBusArrangements(
                ins.as_mut_ptr(),
                ins.len() as i32,
                outs.as_mut_ptr(),
                outs.len() as i32,
            ) != kResultOk
            {
                return Err(PluginError::Activate("bus arrangements refused".into()));
            }
            for (direction, ports) in [(0, &layout.audio_inputs), (1, &layout.audio_outputs)] {
                if objects.component.getBusCount(0, direction) != ports.len() as i32 {
                    return Err(PluginError::Layout(
                        "bus count changed during arrangement".into(),
                    ));
                }
                for (index, port) in ports.iter().enumerate() {
                    let mut bus: BusInfo = std::mem::zeroed();
                    if objects
                        .component
                        .getBusInfo(0, direction, index as i32, &mut bus)
                        != kResultOk
                        || bus.channelCount != port.channels as i32
                    {
                        return Err(PluginError::Layout(
                            "channel count changed during arrangement".into(),
                        ));
                    }
                }
            }
            for (media, direction, count) in [
                (0, 0, ins.len()),
                (0, 1, outs.len()),
                (1, 0, layout.note_inputs as usize),
                (1, 1, layout.note_outputs as usize),
            ] {
                for index in 0..count {
                    if objects
                        .component
                        .activateBus(media, direction, index as i32, 1)
                        != kResultOk
                    {
                        return Err(PluginError::Activate("bus activation refused".into()));
                    }
                }
            }
            let mut setup = ProcessSetup {
                processMode: 0,
                symbolicSampleSize: 0,
                maxSamplesPerBlock: max_block as i32,
                sampleRate: sample_rate,
            };
            if objects.processor.setupProcessing(&mut setup) != kResultOk
                || objects.component.setActive(1) != kResultOk
            {
                return Err(PluginError::Activate("processing setup refused".into()));
            }
        }
        let parameters = Changes::new();
        let parameter_ptr = parameters
            .to_com_ptr::<IParameterChanges>()
            .expect("changes");
        let output_parameters = Changes::new();
        let output_parameter_ptr = output_parameters
            .to_com_ptr::<IParameterChanges>()
            .expect("changes");
        let events = Events::new();
        let event_ptr = events.to_com_ptr::<IEventList>().expect("events");
        let output_events = Events::new();
        let output_event_ptr = output_events.to_com_ptr::<IEventList>().expect("events");
        Ok(Self {
            objects,
            values,
            inputs: Ports::new(&layout.audio_inputs, max_block as usize),
            outputs: Ports::new(&layout.audio_outputs, max_block as usize),
            parameters,
            output_parameters,
            parameter_ptr,
            output_parameter_ptr,
            events,
            output_events,
            event_ptr,
            output_event_ptr,
            edits,
            sample_rate,
            started: false,
            reset_notes: false,
            held: [[false; 128]; 16],
            event_input: layout.note_inputs > 0,
        })
    }
    fn value(&self, id: u32) -> Option<&Value> {
        self.values
            .binary_search_by_key(&id, |v| v.id)
            .ok()
            .map(|i| &self.values[i])
    }
    fn note(&self, time: u32, key: u8, channel: u8, velocity: f32, on: bool) {
        if !self.event_input {
            return;
        }
        // SAFETY: Event is an SDK POD. Only the active pointer-free union
        // variant below is exposed to the plugin for this process call.
        let mut event: Event = unsafe { std::mem::zeroed() };
        event.sampleOffset = time as i32;
        event.flags = 1;
        let id = i32::from(channel) * 128 + i32::from(key);
        if on {
            event.r#type = 0;
            event.__field0.noteOn = NoteOnEvent {
                channel: channel.into(),
                pitch: key.into(),
                tuning: 0.0,
                velocity,
                length: 0,
                noteId: id,
            };
        } else {
            event.r#type = 1;
            event.__field0.noteOff = NoteOffEvent {
                channel: channel.into(),
                pitch: key.into(),
                velocity,
                noteId: id,
                tuning: 0.0,
            };
        }
        self.events.push(event);
    }
}
impl ProcessorBackend for VstProcessor {
    fn process(
        &mut self,
        audio: AudioIo<'_>,
        events: &[HostEvent],
        transport: &Transport,
        steady_time: u64,
        out: &mut dyn FnMut(PluginEvent),
    ) -> Result<BlockResult, ProcessFailed> {
        let mut context = process_context(transport, self.sample_rate, steady_time)?;
        let _guard = AudioCall::enter();
        // SAFETY: processor has one exclusive audio owner, lifecycle calls
        // occur here rather than on the concurrent controller/main thread.
        if !self.started {
            if unsafe { self.objects.processor.setProcessing(1) } != kResultOk {
                return Err(ProcessFailed);
            }
            self.started = true;
        }
        let frames = audio.frames();
        self.inputs.clear(frames);
        self.outputs.clear(frames);
        self.parameters.clear();
        self.output_parameters.clear();
        self.events.clear();
        self.output_events.clear();
        let mut pending_count = 0;
        for value in self.values.iter() {
            if pending_count == 256 {
                break;
            }
            if value
                .pending
                .swap(false, std::sync::atomic::Ordering::Relaxed)
            {
                self.parameters.push(value.id, 0, value.normalized());
                pending_count += 1;
            }
        }
        if self.reset_notes {
            for channel in 0..16 {
                for key in 0..128 {
                    if self.held[channel][key] {
                        self.note(0, key as u8, channel as u8, 0.0, false);
                    }
                }
            }
            self.held = [[false; 128]; 16];
            self.reset_notes = false;
        }
        // Bounded queue drain, followed by sorted scheduled points.
        for _ in 0..crate::instance::QUEUE_CAPACITY {
            let Ok((id, value)) = self.edits.pop() else {
                break;
            };
            if let Some(v) = self.value(id) {
                if !v.writable {
                    continue;
                }
                v.set(value);
                self.parameters.push(id, 0, value);
            }
        }
        for event in events {
            match *event {
                HostEvent::Param { time, id, value } => {
                    if let Some(v) = self.value(id) {
                        if !v.writable {
                            continue;
                        }
                        let normalized = (value / v.scale).clamp(0.0, 1.0);
                        v.set(normalized);
                        self.parameters.push(id, time, normalized);
                    }
                }
                HostEvent::NoteOn {
                    time,
                    key,
                    channel,
                    velocity,
                } => {
                    self.note(time, key, channel, velocity, true);
                    self.held[channel as usize][key as usize] = true;
                }
                HostEvent::NoteOff {
                    time,
                    key,
                    channel,
                    velocity,
                } => {
                    self.note(time, key, channel, velocity, false);
                    self.held[channel as usize][key as usize] = false;
                }
                HostEvent::AllNotesOff { time } => {
                    for channel in 0..16 {
                        for key in 0..128 {
                            if self.held[channel][key] {
                                self.note(time, key as u8, channel as u8, 0.0, false);
                                self.held[channel][key] = false;
                            }
                        }
                    }
                }
            }
        }
        let (left, right) = match audio {
            AudioIo::Separate {
                in_left,
                in_right,
                out_left,
                out_right,
            } => {
                self.inputs.input(in_left, in_right);
                (out_left, out_right)
            }
            AudioIo::InPlace { left, right } => {
                self.inputs.input(left, right);
                (left, right)
            }
        };
        let mut data = ProcessData {
            processMode: 0,
            symbolicSampleSize: 0,
            numSamples: frames as i32,
            numInputs: self.inputs.buses.len() as i32,
            numOutputs: self.outputs.buses.len() as i32,
            inputs: self.inputs.buses.as_mut_ptr(),
            outputs: self.outputs.buses.as_mut_ptr(),
            inputParameterChanges: self.parameter_ptr.as_ptr(),
            outputParameterChanges: self.output_parameter_ptr.as_ptr(),
            inputEvents: self.event_ptr.as_ptr(),
            outputEvents: self.output_event_ptr.as_ptr(),
            processContext: &mut context,
        };
        // SAFETY: all buffers and interfaces live for this synchronous call;
        // plugin must not retain buffer/context pointers beyond process.
        if unsafe { self.objects.processor.process(&mut data) } != kResultOk {
            return Err(ProcessFailed);
        }
        self.outputs.output(left, right);
        self.output_parameters.visit(&mut |id, _time, normalized| {
            if let Some(v) = self.value(id) {
                v.set(normalized);
                out(PluginEvent::ParamValue {
                    id,
                    value: normalized * v.scale,
                });
            }
        });
        Ok(BlockResult {
            status: ProcessStatus::Continue,
            tail: None,
            dropped_events: self
                .parameters
                .dropped()
                .saturating_add(self.output_parameters.dropped())
                .saturating_add(self.events.dropped())
                .saturating_add(self.output_events.dropped()),
        })
    }
    fn reset(&mut self) {
        self.stop();
        self.reset_notes = true;
    }
    fn stop(&mut self) {
        let _guard = AudioCall::enter();
        let _ = self.quiesce();
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod meter_tests {
    use super::*;
    use crate::MeterAnchor;

    #[test]
    fn native_bar_fields_follow_shortened_song_bars() {
        let origin = 4001.0 / 960.0;
        for (beats, bar_start) in [
            (origin, origin),
            (origin + 3.499, origin),
            (origin + 3.5, origin + 3.5),
            (origin + 7.125, origin + 7.0),
        ] {
            let context = process_context(
                &Transport {
                    playing: true,
                    tempo_bpm: 137.0,
                    position_beats: beats,
                    position_seconds: 9.25,
                    numerator: 7,
                    denominator: 8,
                    meter_anchor: Some(MeterAnchor {
                        bar_origin_beats: origin,
                        bar_origin_index: 2,
                    }),
                },
                48_000.0,
                123,
            )
            .unwrap_or_else(|_| panic!("valid transport"));
            assert_eq!(context.barPositionMusic, bar_start);
            assert_eq!(context.projectTimeMusic, beats);
            assert_eq!(context.projectTimeSamples, 444_000);
            assert_eq!(context.continousTimeSamples, 123);
            assert_eq!(context.sampleRate, 48_000.0);
            assert_eq!(context.tempo, 137.0);
            assert_eq!(context.timeSigNumerator, 7);
            assert_eq!(context.timeSigDenominator, 8);
            assert_eq!(context.state, 512 | 1024 | 2048 | 8192 | 131072 | 2);
        }
    }

    #[test]
    fn native_scalar_pattern_transport_retains_beat_zero_origin() {
        let context = process_context(
            &Transport {
                position_beats: 8.125,
                ..Transport::default()
            },
            48_000.0,
            u64::MAX,
        )
        .unwrap_or_else(|_| panic!("valid scalar transport"));
        assert_eq!(context.barPositionMusic, 8.0);
        assert_eq!(context.continousTimeSamples, i64::MAX);
        assert_eq!(context.state & 2, 0);
    }

    #[test]
    fn invalid_anchor_refuses_native_transport_before_processing() {
        for anchor in [
            MeterAnchor {
                bar_origin_beats: f64::INFINITY,
                bar_origin_index: 0,
            },
            MeterAnchor {
                bar_origin_beats: -1.0,
                bar_origin_index: 0,
            },
            MeterAnchor {
                bar_origin_beats: 1.0,
                bar_origin_index: 0,
            },
            MeterAnchor {
                bar_origin_beats: 0.0,
                bar_origin_index: u32::MAX,
            },
        ] {
            assert!(
                process_context(
                    &Transport {
                        meter_anchor: Some(anchor),
                        ..Transport::default()
                    },
                    48_000.0,
                    0
                )
                .is_err()
            );
        }
    }
}
