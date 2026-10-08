//! Atomic Mac fixture publication and the frozen native owner probe's CF owner.
use std::ffi::{CStr, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use vst3::{ComPtr, Steinberg::IPluginFactory};
type CFRef = *const c_void;
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: CFRef);
    fn CFURLCreateFromFileSystemRepresentation(
        allocator: CFRef,
        bytes: *const u8,
        count: isize,
        directory: u8,
    ) -> CFRef;
    fn CFBundleCreate(allocator: CFRef, url: CFRef) -> CFRef;
    fn CFBundleLoadExecutableAndReturnError(bundle: CFRef, error: *mut CFRef) -> u8;
    fn CFStringCreateWithCString(
        allocator: CFRef,
        text: *const std::ffi::c_char,
        encoding: u32,
    ) -> CFRef;
    fn CFBundleGetFunctionPointerForName(bundle: CFRef, name: CFRef) -> *mut c_void;
}
struct CF(CFRef);
impl Drop for CF {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) };
        }
    }
}
pub fn publish(bundle: &Path, library: &Path, executable: &str) {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    assert!(!executable.contains(['/', '&', '<', '>', '"']));
    let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
    let temporary = bundle.with_extension(format!("bundle-{}-{serial}.tmp", std::process::id()));
    std::fs::create_dir_all(temporary.join("Contents/MacOS")).unwrap();
    std::fs::copy(library, temporary.join("Contents/MacOS").join(executable)).unwrap();
    std::fs::write(temporary.join("Contents/Info.plist"), format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
        <plist version=\"1.0\"><dict>\
        <key>CFBundleExecutable</key><string>{executable}</string>\
        <key>CFBundleIdentifier</key><string>org.windfall.fixture.{}.{serial}</string>\
        <key>CFBundlePackageType</key><string>BNDL</string>\
        <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>\
        <key>CFBundleVersion</key><string>1</string></dict></plist>\n", std::process::id())).unwrap();
    std::fs::rename(temporary, bundle).unwrap();
}

/// Only used by the real fixture's intentional wrong-owner subprocess. Its
/// native factory/component refs are released before exit and final CFRelease.
pub struct NativeFixture {
    factory: Option<ComPtr<IPluginFactory>>,
    bundle: CF,
    exit: Option<unsafe extern "C" fn() -> bool>,
    attempted: bool,
}
impl NativeFixture {
    pub fn load(path: &Path) -> Self {
        let path = path.canonicalize().unwrap();
        let bytes = path.as_os_str().as_bytes();
        let url = CF(unsafe {
            CFURLCreateFromFileSystemRepresentation(
                ptr::null(),
                bytes.as_ptr(),
                bytes.len() as isize,
                1,
            )
        });
        assert!(!url.0.is_null());
        let mut owner = Self {
            factory: None,
            bundle: CF(unsafe { CFBundleCreate(ptr::null(), url.0) }),
            exit: None,
            attempted: false,
        };
        assert!(!owner.bundle.0.is_null());
        let mut error = CF(ptr::null());
        assert_ne!(
            unsafe { CFBundleLoadExecutableAndReturnError(owner.bundle.0, &mut error.0) },
            0
        );
        let entry: unsafe extern "C" fn(CFRef) -> bool =
            unsafe { std::mem::transmute(owner.symbol(c"bundleEntry")) };
        owner.exit = Some(unsafe {
            std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> bool>(
                owner.symbol(c"bundleExit"),
            )
        });
        let get: unsafe extern "system" fn() -> *mut IPluginFactory =
            unsafe { std::mem::transmute(owner.symbol(c"GetPluginFactory")) };
        owner.attempted = true;
        assert!(unsafe { entry(owner.bundle.0) });
        owner.factory = unsafe { ComPtr::from_raw(get()) };
        assert!(owner.factory.is_some());
        owner
    }
    fn symbol(&self, name: &CStr) -> *mut c_void {
        let name =
            CF(unsafe { CFStringCreateWithCString(ptr::null(), name.as_ptr(), 0x0800_0100) });
        assert!(!name.0.is_null());
        let pointer = unsafe { CFBundleGetFunctionPointerForName(self.bundle.0, name.0) };
        assert!(!pointer.is_null());
        pointer
    }
    pub fn factory(&self) -> &ComPtr<IPluginFactory> {
        self.factory.as_ref().unwrap()
    }
}
impl Drop for NativeFixture {
    fn drop(&mut self) {
        drop(self.factory.take());
        if std::mem::replace(&mut self.attempted, false)
            && let Some(exit) = self.exit.take()
        {
            unsafe { exit() };
        }
        // bundle field is released last; no observer Library is retained.
    }
}
