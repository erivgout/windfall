//! A native notification must not cross a replacement document boundary.
use super::{Rig, SAMPLE_RATE};
use windfall_engine::plugins::PluginFactory;
use windfall_project::{Command, PluginTarget};

#[test]
fn received_native_update_is_rejected_after_same_binding_project_replacement() {
    let mut rig = Rig::new();
    let (manager, path) = super::plugins::manager(&rig);
    let plugin = manager
        .binding(
            &path,
            "org.windfall.test.sine",
            PluginTarget::Instrument {
                channel: windfall_project::ChannelId(0),
            },
        )
        .unwrap();
    let channel = windfall_project::ChannelId(
        rig.session
            .dispatch(Command::AddPluginInstrument { plugin }, None)
            .unwrap()
            .created[0],
    );
    rig.run(256);
    let target = PluginTarget::Instrument { channel };
    let binding = rig.project().plugin(target).unwrap().clone();
    let stamp = (
        manager.runtime.revision(),
        manager.runtime.selected_token(target).unwrap(),
        crate::plugins::binding_identity(&binding),
    );
    let file = rig.file("same-binding.windfall");
    // Write the original binding exactly, without replacing it with captured state.
    windfall_project::file::save(&rig.project(), std::path::Path::new(&file)).unwrap();
    let held = rig.session.hold("plugin-update:apply");
    let runtime = manager.runtime.clone();
    let applying = rig.session.background(move |session| {
        session.dispatch_plugin_update(
            &runtime,
            stamp,
            Command::SetPluginParam {
                target,
                id: 1,
                value: 0.9,
            },
            None,
        )
    });
    held.wait();
    rig.session.project_open(&file).unwrap();
    rig.run(SAMPLE_RATE as usize / 100);
    assert_eq!(
        rig.project().plugin(target),
        Some(&binding),
        "the replacement deliberately has the same numeric target and exact binding"
    );
    held.release();
    assert!(
        !applying.join().unwrap().unwrap(),
        "the stale notification must be discarded"
    );
    assert_eq!(rig.project().plugin(target), Some(&binding));
}
