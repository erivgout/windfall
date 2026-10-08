//! Native Mac SDK entry/exit only; classes and CreatorThread are unchanged.
use std::ffi::c_void;
use std::sync::Mutex;
type CFRef = *const c_void;
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRetain(value: CFRef) -> CFRef;
    fn CFRelease(value: CFRef);
    fn CFGetTypeID(value: CFRef) -> usize;
    fn CFBundleGetTypeID() -> usize;
}
struct Entries {
    refs: [usize; 32],
    count: usize,
}
static ENTRIES: Mutex<Entries> = Mutex::new(Entries {
    refs: [0; 32],
    count: 0,
});
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bundleEntry(bundle: CFRef) -> bool {
    // SAFETY: the native host supplies a live CF object; reject null/nonbundle.
    if bundle.is_null() || unsafe { CFGetTypeID(bundle) != CFBundleGetTypeID() } {
        return false;
    }
    let Ok(mut entries) = ENTRIES.lock() else {
        std::process::abort()
    };
    if entries.count == entries.refs.len() {
        std::process::abort();
    }
    // SAFETY: retain one real bundle ref for every attempted SDK entry ticket.
    unsafe { CFRetain(bundle) };
    let index = entries.count;
    entries.refs[index] = bundle as usize;
    entries.count += 1;
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn bundleExit() -> bool {
    let bundle = {
        let Ok(mut entries) = ENTRIES.lock() else {
            std::process::abort()
        };
        if entries.count == 0 {
            return false;
        }
        entries.count -= 1;
        let index = entries.count;
        std::mem::replace(&mut entries.refs[index], 0) as CFRef
    };
    // SAFETY: pop exactly one retained entry ref; release outside the mutex.
    unsafe { CFRelease(bundle) };
    true
}
