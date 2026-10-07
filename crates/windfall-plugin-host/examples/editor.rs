//! Open an editor without the desktop app; no audio device is opened.
use std::path::Path;
use std::time::{Duration, Instant};
use windfall_plugin_host::{EditorOptions, PluginHost};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.len() < 2 {
        return Err("usage: editor <plugin file> <plugin id> [seconds]".into());
    }
    let seconds = arguments.get(2).map_or(Ok(30_u64), |s| s.parse())?;
    let host = PluginHost::windfall();
    let module = host.load(Path::new(&arguments[0]))?;
    let mut instance = module.create(&arguments[1])?;
    let info = instance.open_editor(&EditorOptions {
        title: format!("Windfall: {}", instance.descriptor().name),
        take_focus: false,
        ..EditorOptions::default()
    })?;
    println!("editor opened: {info:?}");
    #[cfg(windows)]
    if let Some(handle) = info.window {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetParent, IsWindow, IsWindowVisible};
        let handle = handle as *mut std::ffi::c_void;
        // SAFETY: the instance returned this live HWND on this thread.
        unsafe {
            assert_ne!(IsWindow(handle), 0);
            assert_ne!(IsWindowVisible(handle), 0);
            assert!(GetParent(handle).is_null());
        }
        println!("verified visible native top-level window");
    }
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline && instance.editor().is_some() {
        windfall_plugin_host::gui::pump_events(Some(Duration::from_millis(8)));
        instance.idle(&mut |notification| println!("{notification:?}"));
    }
    instance.close_editor();
    #[cfg(windows)]
    if let Some(handle) = info.window {
        // SAFETY: IsWindow also accepts destroyed handles.
        assert_eq!(
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(
                    handle as *mut std::ffi::c_void,
                )
            },
            0
        );
        println!("verified window destroyed");
    }
    Ok(())
}
