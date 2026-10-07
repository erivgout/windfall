//! Exercise supplied plugins without opening an audio device.
//! Run a separate process per untrusted file to contain a crash/hang.
use windfall_plugin_host::{PluginHost, PluginKind, ProcessStatus, Transport};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: audio_probe <plugin file>")?;
    let module = PluginHost::windfall().load(std::path::Path::new(&path))?;
    let mut reports = Vec::new();
    for descriptor in module.descriptors() {
        let mut instance = module.create(&descriptor.id)?;
        let parameter_count = instance.params().len();
        let readable = instance
            .params()
            .iter()
            .map(|p| p.id)
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|&id| instance.param_value(id).is_some())
            .count();
        let state = instance.save_state()?;
        instance.load_state(&state)?;
        let mut p = instance.activate(48_000.0, 512)?;
        p.set_realtime(false);
        p.set_transport(Transport {
            playing: true,
            tempo_bpm: 127.0,
            ..Transport::default()
        });
        let instrument = descriptor.kind == PluginKind::Instrument;
        if instrument {
            p.note_on(32, 60, 0.8);
        }
        let mut peak = 0.0_f32;
        let mut energy = 0.0_f64;
        let mut nonzero = 0_u64;
        let mut left = [0.0; 512];
        let mut right = [0.0; 512];
        for block in 0..128 {
            for frame in 0..512 {
                let value = if instrument {
                    0.0
                } else {
                    ((block * 512 + frame) as f32 * std::f32::consts::TAU * 440.0 / 48_000.0).sin()
                        * 0.1
                };
                left[frame] = value;
                right[frame] = value * 0.75;
            }
            if instrument && block == 96 {
                p.note_off(64, 60);
            }
            if p.process(&mut left, &mut right) == ProcessStatus::Failed {
                return Err(format!("{} process failed", descriptor.name).into());
            }
            for sample in left.iter().chain(&right) {
                if !sample.is_finite() {
                    return Err("nonfinite output".into());
                }
                peak = peak.max(sample.abs());
                energy += f64::from(*sample).powi(2);
                nonzero += u64::from(sample.abs() > 1e-8);
            }
            instance.idle(&mut |_| {});
        }
        let health = p.health();
        if health.failed || health.scrubbed_samples != 0 || health.dropped_events != 0 {
            return Err(format!("{} unhealthy processing: {health:?}", descriptor.name).into());
        }
        let latency = p.latency_samples();
        let tail = p.tail_samples();
        p.stop();
        instance.deactivate(p).map_err(|error| error.error)?;
        let after = instance.save_state()?;
        instance.load_state(&after)?;
        if peak <= 1e-6 || nonzero == 0 {
            return Err(format!("{} produced no measurable audio", descriptor.name).into());
        }
        reports.push(serde_json::json!({"name":descriptor.name,"id":descriptor.id,"instrument":instrument,"parameters":parameter_count,"readableParameters":readable,"frames":65536,"peak":peak,"rms":(energy/131072.0).sqrt(),"nonzeroSamples":nonzero,"latency":latency,"tail":tail,"stateBytes":state.as_bytes().len(),"stateAfterBytes":after.as_bytes().len(),"health":format!("{health:?}")}));
    }
    println!("{}", serde_json::to_string_pretty(&reports)?);
    Ok(())
}
