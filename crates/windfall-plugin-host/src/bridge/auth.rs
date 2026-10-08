//! Off-realtime private launch-channel authentication, before Load disclosure.

use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    process::Child,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use windows_sys::Win32::Security::Cryptography::{
    BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom,
};

const MAGIC: &[u8; 4] = b"WFAH";
const VERSION: u32 = 1;
const HELLO_BYTES: usize = 40;
const CANDIDATES: usize = 8;
const CANDIDATE_TIMEOUT: Duration = Duration::from_millis(50);

pub struct Key([u8; 32]);
impl Key {
    pub fn generate() -> io::Result<Self> {
        let mut bytes = [0; 32];
        // SAFETY: sized writable buffer; system-preferred RNG needs no provider.
        let result = unsafe {
            BCryptGenRandom(
                std::ptr::null_mut(),
                bytes.as_mut_ptr(),
                bytes.len() as u32,
                BCRYPT_USE_SYSTEM_PREFERRED_RNG,
            )
        };
        if result != 0 {
            return Err(io::Error::other("bridge launch RNG failed"));
        }
        Ok(Self(bytes))
    }
    pub fn write_to_child(&self, child: &mut Child, session: u64) -> io::Result<()> {
        // Newly created private stdin pipe: 32-byte key + eight-byte session. No key is
        // in argv/environment/logs. Closing this handle ends the transfer.
        let mut pipe = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("helper needs private stdin"))?;
        if session == 0 {
            return Err(io::Error::other("invalid launch session"));
        }
        pipe.write_all(&self.0)?;
        pipe.write_all(&session.to_le_bytes())
    }
    fn matches(&self, hello: &[u8; HELLO_BYTES]) -> bool {
        &hello[..4] == MAGIC
            && hello[4..8] == VERSION.to_le_bytes()
            && hello[8..]
                .iter()
                .zip(&self.0)
                .fold(0u8, |difference, (a, b)| difference | (a ^ b))
                == 0
    }
}
impl Drop for Key {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

/// Helper owner only, before reading Load or loading native code.
pub fn helper_hello(socket: &mut TcpStream) -> io::Result<u64> {
    let mut key = Key([0; 32]);
    let mut input = std::io::stdin().lock();
    input.read_exact(&mut key.0)?;
    let mut session = [0u8; 8];
    input.read_exact(&mut session)?;
    let session = u64::from_le_bytes(session);
    if session == 0 {
        return Err(io::Error::other("invalid launch session"));
    }
    let mut hello = [0; HELLO_BYTES];
    hello[..4].copy_from_slice(MAGIC);
    hello[4..8].copy_from_slice(&VERSION.to_le_bytes());
    hello[8..].copy_from_slice(&key.0);
    socket.set_write_timeout(Some(Duration::from_secs(5)))?;
    socket.write_all(&hello)?;
    hello.fill(0);
    Ok(session)
}

struct Candidate {
    socket: TcpStream,
    bytes: [u8; HELLO_BYTES],
    received: usize,
    started: Instant,
}
/// Sends no bytes. The caller may disclose Load only on the returned socket.
/// Multiple bounded nonblocking candidates prevent a stalled first socket from
/// monopolizing startup. This is not protection from a privileged local actor.
pub fn accept(
    listener: &TcpListener,
    child: &mut Child,
    key: &Key,
    deadline: Instant,
    cancelled: Option<&AtomicBool>,
) -> io::Result<TcpStream> {
    let mut candidates: Vec<Candidate> = Vec::with_capacity(CANDIDATES);
    loop {
        if cancelled.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            return Err(io::Error::other("helper authentication cancelled"));
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "helper authentication timed out",
            ));
        }
        if child.try_wait()?.is_some() {
            return Err(io::Error::other("helper exited before authentication"));
        }
        // At most eight new accepts per iteration, including rejected excess.
        for _ in 0..CANDIDATES {
            match listener.accept() {
                Ok((socket, address)) => {
                    if address.ip().is_loopback() {
                        if candidates.len() == CANDIDATES {
                            // A full stalled set cannot exclude the newly
                            // launched child. Evict the oldest without writing.
                            let oldest = candidates
                                .iter()
                                .enumerate()
                                .min_by_key(|(_, candidate)| candidate.started)
                                .map(|(index, _)| index)
                                .expect("full candidate set");
                            candidates.swap_remove(oldest);
                        }
                        socket.set_nonblocking(true)?;
                        candidates.push(Candidate {
                            socket,
                            bytes: [0; HELLO_BYTES],
                            received: 0,
                            started: Instant::now(),
                        });
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        let mut at = 0;
        while at < candidates.len() {
            let candidate = &mut candidates[at];
            let mut reject = candidate.started.elapsed() >= CANDIDATE_TIMEOUT;
            if !reject {
                match candidate
                    .socket
                    .read(&mut candidate.bytes[candidate.received..])
                {
                    Ok(0) => reject = true,
                    Ok(length) => candidate.received += length,
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) => {}
                    Err(_) => reject = true,
                }
            }
            if candidate.received >= 8
                && (&candidate.bytes[..4] != MAGIC
                    || candidate.bytes[4..8] != VERSION.to_le_bytes())
            {
                reject = true;
            }
            if candidate.received == HELLO_BYTES && !reject {
                if key.matches(&candidate.bytes) {
                    return Ok(candidates.swap_remove(at).socket);
                }
                reject = true;
            }
            if reject {
                candidates.swap_remove(at);
            } else {
                at += 1;
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hello_has_exact_version_and_private_key_without_downgrade() {
        let key = Key([42; 32]);
        let mut hello = [0u8; HELLO_BYTES];
        hello[..4].copy_from_slice(MAGIC);
        hello[4..8].copy_from_slice(&VERSION.to_le_bytes());
        hello[8..].fill(42);
        assert!(key.matches(&hello));
        for index in 0..HELLO_BYTES {
            hello[index] ^= 1;
            assert!(!key.matches(&hello));
            hello[index] ^= 1;
        }
        hello[4..8].copy_from_slice(&0u32.to_le_bytes());
        assert!(!key.matches(&hello));
    }
}
