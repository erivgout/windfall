use super::stage::{Stage, input, output, rate, transition_samples};
use crate::blocks::biquad::BiquadCoeffs;
use crate::blocks::smooth::LinearRamp;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum FilterSlotMode {
    #[default]
    Bypass,
    Lowpass,
    Bandpass,
    Highpass,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum FilterRouting {
    #[default]
    Serial,
    Parallel,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct FilterSlotParams {
    pub mode: FilterSlotMode,
    /// Corner or center in Hz, 20 to 20,000; clamped below Nyquist.
    pub frequency_hz: f32,
    /// Resonance Q, 0.1 to 10; default 1/sqrt(2).
    pub q: f32,
}
impl Default for FilterSlotParams {
    fn default() -> Self {
        Self {
            mode: FilterSlotMode::Bypass,
            frequency_hz: 1_000.0,
            q: std::f32::consts::FRAC_1_SQRT_2,
        }
    }
}
param_set!(FilterSlotParams, "Filter Slot", {
    choice [mode] "mode" "Mode" { FilterSlotMode, Bypass, [Bypass "bypass" "Bypass", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Highpass "highpass" "Highpass"] }
    float [frequency_hz] "frequencyHz" "Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [q] "q" "Q" { Ratio, Logarithmic, 0.1, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
});

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct FilterBankParams {
    /// Eight independent stereo filter slots; all bypassed by default.
    pub slots: [FilterSlotParams; 8],
    /// Serial cascades all slots. Parallel averages the active branches.
    pub routing: FilterRouting,
}
impl Default for FilterBankParams {
    fn default() -> Self {
        Self {
            slots: [FilterSlotParams::default(); 8],
            routing: FilterRouting::Serial,
        }
    }
}
param_set!(FilterBankParams, "Filter Bank", {
    choice [slots[0].mode] "slots.0.mode" "Slot 1 Mode" { FilterSlotMode, Bypass, [Bypass "bypass" "Bypass", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Highpass "highpass" "Highpass"] }
    float [slots[0].frequency_hz] "slots.0.frequencyHz" "Slot 1 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [slots[0].q] "slots.0.q" "Slot 1 Q" { Ratio, Logarithmic, 0.1, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    choice [slots[1].mode] "slots.1.mode" "Slot 2 Mode" { FilterSlotMode, Bypass, [Bypass "bypass" "Bypass", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Highpass "highpass" "Highpass"] }
    float [slots[1].frequency_hz] "slots.1.frequencyHz" "Slot 2 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [slots[1].q] "slots.1.q" "Slot 2 Q" { Ratio, Logarithmic, 0.1, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    choice [slots[2].mode] "slots.2.mode" "Slot 3 Mode" { FilterSlotMode, Bypass, [Bypass "bypass" "Bypass", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Highpass "highpass" "Highpass"] }
    float [slots[2].frequency_hz] "slots.2.frequencyHz" "Slot 3 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [slots[2].q] "slots.2.q" "Slot 3 Q" { Ratio, Logarithmic, 0.1, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    choice [slots[3].mode] "slots.3.mode" "Slot 4 Mode" { FilterSlotMode, Bypass, [Bypass "bypass" "Bypass", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Highpass "highpass" "Highpass"] }
    float [slots[3].frequency_hz] "slots.3.frequencyHz" "Slot 4 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [slots[3].q] "slots.3.q" "Slot 4 Q" { Ratio, Logarithmic, 0.1, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    choice [slots[4].mode] "slots.4.mode" "Slot 5 Mode" { FilterSlotMode, Bypass, [Bypass "bypass" "Bypass", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Highpass "highpass" "Highpass"] }
    float [slots[4].frequency_hz] "slots.4.frequencyHz" "Slot 5 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [slots[4].q] "slots.4.q" "Slot 5 Q" { Ratio, Logarithmic, 0.1, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    choice [slots[5].mode] "slots.5.mode" "Slot 6 Mode" { FilterSlotMode, Bypass, [Bypass "bypass" "Bypass", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Highpass "highpass" "Highpass"] }
    float [slots[5].frequency_hz] "slots.5.frequencyHz" "Slot 6 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [slots[5].q] "slots.5.q" "Slot 6 Q" { Ratio, Logarithmic, 0.1, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    choice [slots[6].mode] "slots.6.mode" "Slot 7 Mode" { FilterSlotMode, Bypass, [Bypass "bypass" "Bypass", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Highpass "highpass" "Highpass"] }
    float [slots[6].frequency_hz] "slots.6.frequencyHz" "Slot 7 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [slots[6].q] "slots.6.q" "Slot 7 Q" { Ratio, Logarithmic, 0.1, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    choice [slots[7].mode] "slots.7.mode" "Slot 8 Mode" { FilterSlotMode, Bypass, [Bypass "bypass" "Bypass", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Highpass "highpass" "Highpass"] }
    float [slots[7].frequency_hz] "slots.7.frequencyHz" "Slot 8 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [slots[7].q] "slots.7.q" "Slot 8 Q" { Ratio, Logarithmic, 0.1, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    choice [routing] "routing" "Routing" { FilterRouting, Serial, [Serial "serial" "Serial", Parallel "parallel" "Parallel"] }
});

fn coefficients(slot: &FilterSlotParams, sample_rate: f32) -> BiquadCoeffs {
    match slot.mode {
        FilterSlotMode::Bypass => BiquadCoeffs::IDENTITY,
        FilterSlotMode::Lowpass => BiquadCoeffs::low_pass(slot.frequency_hz, slot.q, sample_rate),
        FilterSlotMode::Bandpass => BiquadCoeffs::band_pass(slot.frequency_hz, slot.q, sample_rate),
        FilterSlotMode::Highpass => BiquadCoeffs::high_pass(slot.frequency_hz, slot.q, sample_rate),
    }
}

/// Eight real second-order audio filters. Both routing graphs retain independent
/// histories and run continuously, so routing changes can crossfade over 5 ms.
/// Parallel bypass slots are excluded; an entirely bypassed bank passes dry audio.
pub struct FilterBank {
    sample_rate: f32,
    params: FilterBankParams,
    serial: [Stage; 8],
    parallel: [Stage; 8],
    active: [LinearRamp; 8],
    parallel_mix: LinearRamp,
    fresh: bool,
}
impl Default for FilterBank {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            params: FilterBankParams::default(),
            serial: [Stage::default(); 8],
            parallel: [Stage::default(); 8],
            active: [LinearRamp::new(0.0); 8],
            parallel_mix: LinearRamp::new(0.0),
            fresh: true,
        }
    }
}
impl FilterBank {
    fn apply(&mut self) {
        let samples = transition_samples(self.sample_rate, self.fresh);
        for i in 0..8 {
            let slot = self.params.slots[i];
            let coeffs = coefficients(&slot, self.sample_rate);
            self.serial[i].set(coeffs, samples);
            self.parallel[i].set(coeffs, samples);
            self.active[i].set_target(
                if slot.mode == FilterSlotMode::Bypass {
                    0.0
                } else {
                    1.0
                },
                samples,
            );
        }
        self.parallel_mix.set_target(
            if self.params.routing == FilterRouting::Parallel {
                1.0
            } else {
                0.0
            },
            samples,
        );
    }
}
impl Effect for FilterBank {
    type Params = FilterBankParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.reset();
    }
    fn reset(&mut self) {
        self.fresh = true;
        self.apply();
        for stage in self.serial.iter_mut().chain(&mut self.parallel) {
            stage.reset();
        }
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.params = params.sanitized();
        self.apply();
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            self.fresh = false;
            let dry = [input(*l), input(*r)];
            let mut serial = dry;
            let mut parallel = [0.0; 2];
            let mut weight = 0.0;
            for i in 0..8 {
                serial = self.serial[i].tick(serial);
                let branch = self.parallel[i].tick(dry);
                let active = f64::from(self.active[i].tick());
                weight += active;
                for channel in 0..2 {
                    parallel[channel] += active * branch[channel];
                }
            }
            // A sub-unity active weight blends dry in during the first slot's
            // activation or final slot's deactivation. At rest, this is the mean.
            let dry_weight = (1.0 - weight).max(0.0);
            let normalization = weight.max(1.0);
            let mix = f64::from(self.parallel_mix.tick());
            let mut frame = [0.0; 2];
            for channel in 0..2 {
                let p = (parallel[channel] + dry_weight * dry[channel]) / normalization;
                frame[channel] = if mix == 0.0 {
                    serial[channel]
                } else if mix == 1.0 {
                    p
                } else {
                    serial[channel] + mix * (p - serial[channel])
                };
            }
            *l = output(frame[0]);
            *r = output(frame[1]);
        }
    }
    fn tail_samples(&self) -> usize {
        self.serial
            .iter()
            .map(Stage::tail_samples)
            .sum::<usize>()
            .max(
                self.parallel
                    .iter()
                    .map(Stage::tail_samples)
                    .max()
                    .unwrap_or(0),
            )
    }
}
