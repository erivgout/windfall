//! Creator identity for an unloadable fixture, without Unix Rust TLS cleanup.
//!
//! The verified native ABI is Linux x86_64/glibc: its target
//! `bits/pthreadtypes.h` defines pthread_t as unsigned long, and pthread.h
//! declares pthread_self() -> pthread_t. The header's pthread_equal inline
//! and the native libc implementation both compare the unsigned-long values.
//! Other targets retain the fixture's original Rust ThreadId behavior until
//! their native headers and unload behavior have been separately verified.

#[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]
type Identity = std::ffi::c_ulong;

#[cfg(not(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64")))]
type Identity = std::thread::ThreadId;

#[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]
unsafe extern "C" {
    fn pthread_self() -> Identity;
}

/// The creating thread must remain alive whenever its identity is compared.
/// pthread identities can be reused after a thread exits; fixture lifecycle
/// checks concern the still-live creator that owns the component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CreatorThread(Identity);

impl CreatorThread {
    pub(super) fn current() -> Self {
        #[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]
        {
            // SAFETY: verified target ABI; pthread_self has no arguments and
            // returns the identity of this live calling thread.
            Self(unsafe { pthread_self() })
        }
        #[cfg(not(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64")))]
        {
            Self(std::thread::current().id())
        }
    }
}
