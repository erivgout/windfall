//! Current desktop host's resident memory, queried only on a blocking worker.
//! Webviews and isolated plugin helpers have separate address spaces and are
//! deliberately not included. An unsupported or failed query is unavailable.

const JS_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;

pub(crate) fn resident_bytes() -> Option<u64> {
    platform_resident_bytes().filter(|bytes| *bytes <= JS_SAFE_INTEGER)
}

#[cfg(windows)]
fn platform_resident_bytes() -> Option<u64> {
    use windows_sys::Win32::System::{
        ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::GetCurrentProcess,
    };

    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    // SAFETY: the OS writes a sized, initialized counters struct. The current
    // process pseudo-handle needs neither acquisition nor closing.
    let success =
        unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    (success != 0).then_some(counters.WorkingSetSize as u64)
}

#[cfg(target_os = "linux")]
fn platform_resident_bytes() -> Option<u64> {
    use std::io::Read;

    // statm is seven decimal page counts; bound the read even if procfs fails.
    let mut bytes = Vec::with_capacity(257);
    std::fs::File::open("/proc/self/statm")
        .ok()?
        .take(257)
        .read_to_end(&mut bytes)
        .ok()?;
    // SAFETY: sysconf has no pointer arguments or caller-owned resources.
    let page_size = u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).ok()?;
    statm_resident_bytes(&bytes, page_size)
}

#[cfg(any(target_os = "linux", test))]
fn statm_resident_bytes(bytes: &[u8], page_size: u64) -> Option<u64> {
    if bytes.len() > 256 || page_size == 0 {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let mut fields = text.split_ascii_whitespace();
    let mut resident = 0;
    for index in 0..7 {
        let field = fields.next()?;
        if field.is_empty() || !field.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let pages = field.parse::<u64>().ok()?;
        if index == 1 {
            resident = pages;
        }
    }
    if fields.next().is_some() {
        return None;
    }
    resident.checked_mul(page_size)
}

#[cfg(target_os = "macos")]
fn platform_resident_bytes() -> Option<u64> {
    let mut info = std::mem::MaybeUninit::<libc::mach_task_basic_info_data_t>::zeroed();
    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    // SAFETY: libc's packed Mach ABI struct has the exact natural-word count.
    // task_info writes into this initialized, sufficiently sized storage. The
    // task-self send right is process-owned and must not be deallocated here.
    // Use the existing pinned libc binding rather than another dependency.
    #[allow(deprecated)]
    let result = unsafe {
        libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO,
            info.as_mut_ptr().cast(),
            &mut count,
        )
    };
    if result != libc::KERN_SUCCESS || count != libc::MACH_TASK_BASIC_INFO_COUNT {
        return None;
    }
    // SAFETY: storage started zeroed and task_info succeeded with the full count.
    Some(unsafe { info.assume_init() }.resident_size)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn platform_resident_bytes() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statm_uses_resident_pages_and_rejects_invalid_or_unbounded_counts() {
        assert_eq!(
            statm_resident_bytes(b"100 7 3 2 0 9 0\n", 4096),
            Some(28672)
        );
        assert_eq!(
            statm_resident_bytes(b"100 7 3 2 0 9 0", 16384),
            Some(114688)
        );
        for bytes in [
            b"100 -7 3 2 0 9 0".as_slice(),
            b"100 +7 3 2 0 9 0",
            b"100 7 3 2 0 9",
            b"100 7 3 2 0 9 0 1",
            b"100 18446744073709551615 3 2 0 9 0",
        ] {
            assert_eq!(statm_resident_bytes(bytes, 4096), None);
        }
        assert_eq!(statm_resident_bytes(&[b' '; 257], 4096), None);
        assert_eq!(statm_resident_bytes(b"100 7 3 2 0 9 0", 0), None);
    }

    #[test]
    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    fn current_process_query_returns_resident_bytes_in_the_wire_domain() {
        let bytes = resident_bytes().expect("the current process is queryable");
        assert!(bytes > 0 && bytes <= JS_SAFE_INTEGER);
        eprintln!("current desktop host resident bytes: {bytes}");
    }
}
