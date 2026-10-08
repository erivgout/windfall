//! Bounded, test-only DOS namespace probes. Never imported by production.
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn QueryDosDeviceW(name: *const u16, target: *mut u16, size: u32) -> u32;
    fn DefineDosDeviceW(flags: u32, name: *const u16, target: *const u16) -> i32;
}
#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}
#[repr(C)]
struct ObjectAttributes {
    length: u32,
    root: *mut std::ffi::c_void,
    name: *mut UnicodeString,
    attributes: u32,
    security_descriptor: *mut std::ffi::c_void,
    security_qos: *mut std::ffi::c_void,
}
#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtOpenSymbolicLinkObject(
        handle: *mut *mut std::ffi::c_void,
        access: u32,
        attributes: *mut ObjectAttributes,
    ) -> i32;
    fn NtQuerySymbolicLinkObject(
        handle: *mut std::ffi::c_void,
        target: *mut UnicodeString,
        returned: *mut u32,
    ) -> i32;
    fn NtCreateSymbolicLinkObject(
        handle: *mut *mut std::ffi::c_void,
        access: u32,
        attributes: *mut ObjectAttributes,
        target: *mut UnicodeString,
    ) -> i32;
}

fn wide(text: &str) -> Vec<u16> {
    assert!(text.len() < 2048 && !text.contains('\0'));
    text.encode_utf16().chain(Some(0)).collect()
}
pub(super) fn query(name: &str) -> io::Result<Option<Vec<String>>> {
    let name = wide(name);
    let mut target = [0u16; 4096];
    // SAFETY: bounded terminated name and writable output array live throughout.
    let count = unsafe { QueryDosDeviceW(name.as_ptr(), target.as_mut_ptr(), 4096) };
    if count == 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(2) {
            Ok(None)
        } else {
            Err(error)
        };
    }
    Ok(Some(
        target[..count as usize]
            .split(|unit| *unit == 0)
            .filter(|part| !part.is_empty())
            .map(String::from_utf16_lossy)
            .collect(),
    ))
}
fn define(name: &str, target: &str, remove: bool) -> io::Result<()> {
    let name = wide(name);
    let target = wide(target);
    // RAW_TARGET | NO_BROADCAST, and atomic EXACT_MATCH_ON_REMOVE for retirement.
    let flags = 1 | 8 | if remove { 2 | 4 } else { 0 };
    // SAFETY: both terminated inputs remain live; flags never remove by prefix.
    if unsafe { DefineDosDeviceW(flags, name.as_ptr(), target.as_ptr()) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub(super) struct Drive {
    name: String,
    targets: Vec<String>,
}
impl Drive {
    pub(super) fn new(target: String) -> Self {
        for letter in (b'Q'..=b'Z').rev() {
            let name = format!("{}:", char::from(letter));
            let global = format!(r"Global\{name}");
            if query(&name).unwrap().is_some() || query(&global).unwrap().is_some() {
                continue;
            }
            eprintln!(
                "R2_DOS owned local fixture {name}: effective and {global} queries both absent before creation; NT path \\??\\{name}"
            );
            define(&name, &target, false).unwrap();
            let owned = Self {
                name,
                targets: vec![target.clone()],
            };
            assert_eq!(query(&owned.name).unwrap().unwrap(), [target]);
            return owned;
        }
        panic!("No actually unused drive letter available for the owned probe");
    }
    pub(super) fn name(&self) -> &str {
        &self.name
    }
    fn new_global(target: String) -> io::Result<Self> {
        for letter in (b'Q'..=b'Z').rev() {
            let name = format!("{}:", char::from(letter));
            if query(&name)?.is_some() {
                continue;
            }
            let name = format!(r"Global\{name}");
            if query(&name)?.is_some() {
                continue;
            }
            let owned = Self {
                name,
                targets: vec![target.clone()],
            };
            eprintln!(
                "R2_DOS owned global fixture {}: effective and explicit global queries both absent before attempted creation; NT path \\??\\{}",
                owned.name, owned.name
            );
            define(&owned.name, &target, false)?;
            assert_eq!(query(&owned.name)?.unwrap(), [target]);
            return Ok(owned);
        }
        Err(io::Error::other("No unused global drive probe slot"))
    }
    pub(super) fn push(&mut self, target: String) -> io::Result<()> {
        assert!(self.targets.len() < 4);
        // Register the unique owned definition before the mutation so an error
        // after a partial native push still has exact-target retirement evidence.
        self.targets.push(target.clone());
        define(&self.name, &target, false)
    }
    pub(super) fn remove(&self, target: &str) -> io::Result<()> {
        if query(&self.name)?.is_some_and(|targets| targets.iter().any(|item| item == target)) {
            define(&self.name, target, true)?;
        }
        Ok(())
    }
    pub(super) fn open_link(&self) -> io::Result<OwnedHandle> {
        open_link(&format!(r"\??\{}", self.name))
    }
}
impl Drop for Drive {
    fn drop(&mut self) {
        for target in self.targets.iter().rev() {
            if let Err(error) = self.remove(target) {
                eprintln!("Owned DOS probe {} cleanup refused: {error}", self.name);
            }
        }
    }
}

pub(super) fn open_link(name: &str) -> io::Result<OwnedHandle> {
    let mut name = wide(name);
    let bytes = ((name.len() - 1) * 2) as u16;
    let mut name = UnicodeString {
        length: bytes,
        maximum_length: bytes,
        buffer: name.as_mut_ptr(),
    };
    let mut attributes = ObjectAttributes {
        length: size_of::<ObjectAttributes>() as u32,
        root: std::ptr::null_mut(),
        name: &mut name,
        attributes: 0x40, // OBJ_CASE_INSENSITIVE
        security_descriptor: std::ptr::null_mut(),
        security_qos: std::ptr::null_mut(),
    };
    let mut handle = std::ptr::null_mut();
    // SAFETY: the nested input objects and output pointer live throughout; the
    // returned handle is converted only on success and is owned exactly once.
    let status = unsafe { NtOpenSymbolicLinkObject(&mut handle, 1, &mut attributes) };
    if status < 0 {
        return Err(io::Error::other(format!(
            "NtOpenSymbolicLinkObject {status:#x}"
        )));
    }
    assert!(!handle.is_null());
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}
pub(super) fn link_target(handle: &OwnedHandle) -> io::Result<String> {
    let mut buffer = [0u16; 4096];
    let mut target = UnicodeString {
        length: 0,
        maximum_length: 8192,
        buffer: buffer.as_mut_ptr(),
    };
    let mut returned = 0;
    // SAFETY: the borrowed live object handle and bounded output remain valid.
    let status =
        unsafe { NtQuerySymbolicLinkObject(handle.as_raw_handle(), &mut target, &mut returned) };
    if status < 0 {
        return Err(io::Error::other(format!(
            "NtQuerySymbolicLinkObject {status:#x}"
        )));
    }
    assert!(target.length <= 8192 && target.length.is_multiple_of(2));
    Ok(String::from_utf16_lossy(
        &buffer[..target.length as usize / 2],
    ))
}

// Only a bare drive slot in this normal-token local DOS context is accepted.
// No global object path, OBJ_PERMANENT, OBJ_OPENIF or replacement flag exists.
fn create_local_link(name: &str, target: &str) -> io::Result<OwnedHandle> {
    assert!(
        name.len() == 2
            && (b'Q'..=b'Z').contains(&name.as_bytes()[0])
            && name.as_bytes()[1] == b':'
    );
    let mut path = wide(&format!(r"\??\{name}"));
    let mut name = UnicodeString {
        length: ((path.len() - 1) * 2) as u16,
        maximum_length: ((path.len() - 1) * 2) as u16,
        buffer: path.as_mut_ptr(),
    };
    let mut target = wide(target);
    let mut target = UnicodeString {
        length: ((target.len() - 1) * 2) as u16,
        maximum_length: ((target.len() - 1) * 2) as u16,
        buffer: target.as_mut_ptr(),
    };
    let mut attributes = ObjectAttributes {
        length: size_of::<ObjectAttributes>() as u32,
        root: std::ptr::null_mut(),
        name: &mut name,
        attributes: 0x40, // OBJ_CASE_INSENSITIVE only; nonpermanent, no OPENIF
        security_descriptor: std::ptr::null_mut(),
        security_qos: std::ptr::null_mut(),
    };
    let mut handle = std::ptr::null_mut();
    // SAFETY: all bounded nested inputs and the output handle pointer live for
    // the call. A successful newly created query handle is owned exactly once.
    // Existing objects cannot be opened/replaced by these creation attributes.
    let status =
        unsafe { NtCreateSymbolicLinkObject(&mut handle, 1, &mut attributes, &mut target) };
    if status < 0 {
        return Err(io::Error::other(format!(
            "NtCreateSymbolicLinkObject {status:#x}"
        )));
    }
    assert!(!handle.is_null());
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

#[test]
fn r2_nonpermanent_local_nt_link_effective_bytes_and_retirement_probe() {
    let folder = tempfile::tempdir().unwrap();
    let first = folder.path().join("first");
    let second = folder.path().join("second");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    std::fs::write(first.join("source"), b"first-owned-pcm").unwrap();
    std::fs::write(second.join("source"), b"other-owned-pcm").unwrap();
    let original = format!(r"\??\{}", first.display());
    let replacement = format!(r"\??\{}", second.display());
    let Some(name) = (b'Q'..=b'Z')
        .rev()
        .map(|letter| format!("{}:", char::from(letter)))
        .find(|name| {
            query(name).unwrap().is_none() && query(&format!(r"Global\{name}")).unwrap().is_none()
        })
    else {
        eprintln!("R2_NT_CREATE UNAVAILABLE: no unused local/global slot; no authority proof");
        return;
    };
    eprintln!(
        "R2_NT_CREATE local path \\??\\{name}; effective and Global\\{name} both absent before creation; normal-medium context, attributes=0x40/access=1"
    );
    let handle = match create_local_link(&name, &original) {
        Ok(handle) => handle,
        Err(error) => {
            eprintln!(
                "R2_NT_CREATE UNAVAILABLE: owned nonpermanent local creation refused: {error}; no authority proof"
            );
            return;
        }
    };
    assert_eq!(link_target(&handle).unwrap(), original);
    assert!(query(&format!(r"Global\{name}")).unwrap().is_none());
    assert_eq!(query(&name).unwrap().unwrap()[0], original);
    let path = format!(r"{name}\source");
    assert_eq!(std::fs::read(&path).unwrap(), b"first-owned-pcm");
    let attempted = create_local_link(&name, &replacement);
    let object = link_target(&handle).unwrap();
    let effective = query(&name).unwrap();
    eprintln!(
        "R2_NT_CREATE held-object create_same_name={attempted:?} original_object={object:?} effective={effective:?} bytes={:?}",
        std::fs::read(&path).unwrap()
    );
    assert!(
        attempted.is_err(),
        "creation opened or replaced the held local object"
    );
    assert_eq!(object, original);
    assert_eq!(effective.unwrap()[0], original);
    assert_eq!(std::fs::read(&path).unwrap(), b"first-owned-pcm");
    // Nonpermanent exact-object retirement closes this owned handle, never a
    // pathname or another definition. Verify its target before releasing it.
    assert_eq!(link_target(&handle).unwrap(), original);
    drop(handle);
    assert!(query(&name).unwrap().is_none());
    assert!(query(&format!(r"Global\{name}")).unwrap().is_none());
    let next = create_local_link(&name, &replacement).unwrap();
    assert_eq!(link_target(&next).unwrap(), replacement);
    assert_eq!(query(&name).unwrap().unwrap()[0], replacement);
    assert_eq!(std::fs::read(&path).unwrap(), b"other-owned-pcm");
    assert_eq!(link_target(&next).unwrap(), replacement);
    drop(next);
    assert!(query(&name).unwrap().is_none());
    assert!(query(&format!(r"Global\{name}")).unwrap().is_none());
    assert_eq!(
        std::fs::read(first.join("source")).unwrap(),
        b"first-owned-pcm"
    );
    assert_eq!(
        std::fs::read(second.join("source")).unwrap(),
        b"other-owned-pcm"
    );
    eprintln!(
        "R2_NT_CREATE executed local collision/refusal and exact-object retirement; global/local shadow axis remains UNRESOLVED, no global creation or competitor mutation attempted"
    );
}

#[test]
fn r2_retained_dos_link_push_pop_and_remove_recreate_probe() {
    let folder = tempfile::tempdir().unwrap();
    let first = folder.path().join("first");
    let second = folder.path().join("second");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    std::fs::write(first.join("source"), b"original").unwrap();
    std::fs::write(second.join("source"), b"replacement").unwrap();
    let original = format!(r"\??\{}", first.display());
    let replacement = format!(r"\??\{}", second.display());
    let mut drive = Drive::new(original.clone());
    let handle = drive.open_link().unwrap();
    assert_eq!(link_target(&handle).unwrap(), original);
    let path = format!(r"{}\source", drive.name());
    let pushed = drive.push(replacement.clone());
    let effective = query(drive.name()).unwrap().unwrap();
    eprintln!(
        "R2_DOS retained-object push={pushed:?} original_object={:?} effective={effective:?}",
        link_target(&handle).unwrap()
    );
    assert_eq!(
        effective[0], original,
        "retained DOS object did not pin effective push"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    let removed = drive.remove(&original);
    let effective = query(drive.name()).unwrap();
    eprintln!(
        "R2_DOS retained-object pop={removed:?} original_object={:?} effective={effective:?}",
        link_target(&handle).unwrap()
    );
    assert_eq!(
        effective.as_ref().unwrap()[0],
        original,
        "retained DOS object did not pin effective pop"
    );
    let recreated = drive.push(replacement.clone());
    eprintln!(
        "R2_DOS retained-object recreate={recreated:?} effective={:?}",
        query(drive.name()).unwrap()
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    drop(handle);
    drive.push(replacement.clone()).unwrap();
    assert_eq!(query(drive.name()).unwrap().unwrap()[0], replacement);
    assert_eq!(std::fs::read(path).unwrap(), b"replacement");
    assert_eq!(std::fs::read(first.join("source")).unwrap(), b"original");
    assert_eq!(
        std::fs::read(second.join("source")).unwrap(),
        b"replacement"
    );
    eprintln!(
        "R2_DOS effective remapping succeeds after retained object retirement; 17*(64 parents+leaf+DOS)=1122 proposed handles, production remains unchanged"
    );
}

#[test]
fn r2_normal_direct_physical_volume_mapping_probe() {
    use std::path::{Component, Prefix};
    let folder = tempfile::tempdir().unwrap();
    let drive = match folder.path().components().next().unwrap() {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => char::from(letter),
            _ => panic!("Fixture is not an ordinary local drive path"),
        },
        _ => panic!("Missing fixture drive prefix"),
    };
    let name = format!("{drive}:");
    let target = query(&name).unwrap().unwrap()[0].clone();
    let original_handle = open_link(&format!(r"\??\{name}")).unwrap();
    assert_eq!(link_target(&original_handle).unwrap(), target);
    let volume = target.strip_prefix(r"\Device\HarddiskVolume").unwrap();
    assert!(!volume.is_empty() && volume.bytes().all(|byte| byte.is_ascii_digit()));
    let path = folder.path().join("source");
    std::fs::write(&path, b"normal-physical-volume").unwrap();
    let mut alias = Drive::new(target.clone());
    let handle = alias.open_link().unwrap();
    let relative = path.strip_prefix(format!(r"{name}\")).unwrap();
    let mapped = format!(r"{}\{}", alias.name(), relative.display());
    assert_eq!(std::fs::read(&mapped).unwrap(), b"normal-physical-volume");
    let retarget = alias.push(format!(r"\??\{}", folder.path().display()));
    eprintln!(
        "R2_DOS physical volume={target:?} retarget={retarget:?} original_object={:?} effective={:?}",
        link_target(&handle).unwrap(),
        query(alias.name()).unwrap()
    );
    assert_eq!(link_target(&handle).unwrap(), target);
    assert_eq!(query(alias.name()).unwrap().unwrap()[0], target);
    assert_eq!(std::fs::read(mapped).unwrap(), b"normal-physical-volume");
    // The actual physical drive mapping was queried/opened only, never modified.
    assert_eq!(query(&name).unwrap().unwrap()[0], target);
}

#[test]
fn r2_global_object_and_local_shadow_effective_mapping_probe() {
    let folder = tempfile::tempdir().unwrap();
    let first = folder.path().join("global");
    let second = folder.path().join("local");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    std::fs::write(first.join("source"), b"global-original").unwrap();
    std::fs::write(second.join("source"), b"local-shadow").unwrap();
    let original = format!(r"\??\{}", first.display());
    let replacement = format!(r"\??\{}", second.display());
    let global = match Drive::new_global(original.clone()) {
        Ok(global) => global,
        Err(error) => {
            eprintln!(
                "R2_DOS UNRESOLVED global/local shadow fixture creation refused: {error}; no global authority acceptance"
            );
            return;
        }
    };
    let handle = global.open_link().unwrap();
    assert_eq!(link_target(&handle).unwrap(), original);
    let bare = global.name().strip_prefix(r"Global\").unwrap().to_string();
    assert_eq!(query(&bare).unwrap().unwrap()[0], original);
    let local = Drive {
        name: bare.clone(),
        targets: vec![replacement.clone()],
    };
    let shadowed = define(&bare, &replacement, false);
    let effective = query(&bare).unwrap().unwrap();
    let object = link_target(&handle).unwrap();
    eprintln!(
        "R2_DOS global/local shadow={shadowed:?} original_object={object:?} effective={effective:?}"
    );
    assert_eq!(object, original);
    assert_eq!(
        effective[0], original,
        "original object remains live while effective local mapping shadows it"
    );
    assert_eq!(
        std::fs::read(format!(r"{bare}\source")).unwrap(),
        b"global-original"
    );
    drop(local);
    drop(handle);
}
