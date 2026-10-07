//! VST3 scanning exercises an actual independent SDK factory fixture.
#![cfg(any(windows, target_os = "linux"))]
mod common;
use std::time::Duration;
use windfall_plugin_host::scan::FailureKind;
use windfall_plugin_host::{PluginError, PluginFormat, PluginHost, PluginKind, scan_file};

#[test]
fn vst3_factory_metadata_and_initialized_buses_are_scanned_out_of_process() {
    let path = common::plugin_file("vst3", "fixture.vst3");
    let scan = scan_file(&common::scanner(Duration::from_secs(20)), &path).unwrap();
    assert!(scan.failure.is_none());
    assert_eq!(scan.plugins.len(), 2);
    let good = &scan.plugins[0];
    assert_eq!(good.descriptor.format, PluginFormat::Vst3);
    assert_eq!(good.descriptor.kind, PluginKind::Effect);
    assert_eq!(good.descriptor.vendor, "Windfall Tests");
    assert_eq!(good.descriptor.version, "1.0");
    assert_eq!(good.descriptor.features, ["Fx", "Tools"]);
    let layout = good.layout.as_ref().unwrap();
    assert_eq!(layout.audio_inputs[0].channels, 2);
    assert_eq!(layout.audio_outputs[0].name, "Stereo");
    assert!(layout.audio_outputs[0].main);
    assert_eq!(
        scan.plugins[1].failure.as_ref().unwrap().kind,
        FailureKind::Rejected
    );
    let host = PluginHost::windfall();
    let module = host.load(&path).unwrap();
    assert!(matches!(
        module.create(&good.descriptor.id),
        Err(PluginError::Unsupported(_))
    ));
}
