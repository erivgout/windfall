//! Bounded control framing, used only on worker/helper owner threads.

use super::{
    adapter::ParameterSpec,
    protocol::{Config, Identity, Kind, ProtocolError},
};
use crate::{MAX_STATE_BYTES, PluginState};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read, Write},
    net::TcpStream,
};

const MAGIC: &[u8; 4] = b"WFCB";
const VERSION: u32 = 1;
const PREFIX: usize = 24;
pub const MAX_METADATA_BYTES: usize = 1 << 20;
pub const MAX_PACKET_BYTES: usize = PREFIX + MAX_METADATA_BYTES + MAX_STATE_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    pub session: u64,
    pub token: u64,
    pub revision: u64,
    pub binding: u64,
}
impl From<Identity> for Owner {
    fn from(value: Identity) -> Self {
        Self {
            session: value.session,
            token: value.token,
            revision: value.revision,
            binding: value.binding,
        }
    }
}
impl From<Owner> for Identity {
    fn from(value: Owner) -> Self {
        Self {
            session: value.session,
            token: value.token,
            revision: value.revision,
            binding: value.binding,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub owner: Owner,
    pub sample_rate: u32,
    pub block: u32,
    pub native_latency: u32,
    pub instrument: bool,
}
impl From<Config> for Settings {
    fn from(config: Config) -> Self {
        Self {
            owner: config.identity.into(),
            sample_rate: config.sample_rate,
            block: config.block as u32,
            native_latency: config.native_latency as u32,
            instrument: config.kind == Kind::Instrument,
        }
    }
}
impl Settings {
    pub fn config(self) -> Result<Config, ProtocolError> {
        Config {
            identity: self.owner.into(),
            sample_rate: self.sample_rate,
            block: self.block as usize,
            native_latency: self.native_latency as usize,
            kind: if self.instrument {
                Kind::Instrument
            } else {
                Kind::Effect
            },
        }
        .validate()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlParameter {
    pub id: u32,
    pub min: f64,
    pub max: f64,
    pub value: f64,
    pub read_only: bool,
    pub stepped: bool,
}
impl From<ParameterSpec> for ControlParameter {
    fn from(value: ParameterSpec) -> Self {
        Self {
            id: value.id,
            min: value.min,
            max: value.max,
            value: value.value,
            read_only: value.read_only,
            stepped: value.stepped,
        }
    }
}
impl From<ControlParameter> for ParameterSpec {
    fn from(value: ControlParameter) -> Self {
        Self {
            id: value.id,
            min: value.min,
            max: value.max,
            value: value.value,
            read_only: value.read_only,
            stepped: value.stepped,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Message {
    Load {
        mapping: String,
        settings: Settings,
        path: String,
        id: String,
        format: String,
        approved_binary: crate::paths::PluginFileIdentity,
        parameters: Vec<ControlParameter>,
        offline: bool,
    },
    Ready {
        native_latency: u32,
        tail: u64,
        parameters: Vec<ControlParameter>,
    },
    Capture {
        epoch: u64,
        desired_generation: u64,
        pending: Vec<ControlParameter>,
    },
    Captured {
        epoch: u64,
        processed_generation: u64,
        reconciled_generation: u64,
        parameters: Vec<ControlParameter>,
    },
    Editor {
        open: bool,
    },
    Shutdown,
    Stopped,
    Error {
        message: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    owner: Owner,
    body: Message,
}
#[derive(Debug, Clone)]
pub struct Packet {
    pub request: u64,
    pub owner: Owner,
    pub body: Message,
    pub state: Vec<u8>,
}
impl Packet {
    pub fn new(request: u64, owner: Owner, body: Message) -> Self {
        Self {
            request,
            owner,
            body,
            state: Vec::new(),
        }
    }
    pub fn validate(&self) -> io::Result<()> {
        if self.request == 0
            || !Identity::from(self.owner).valid()
            || self.state.len() > MAX_STATE_BYTES
        {
            return Err(invalid("invalid bridge control identity/size"));
        }
        if !self.state.is_empty() {
            if !matches!(self.body, Message::Load { .. } | Message::Captured { .. }) {
                return Err(invalid("state on an unsupported control message"));
            }
            PluginState::from_bytes(self.state.clone())
                .content()
                .map_err(|_| invalid("malformed native state container"))?;
        }
        Ok(())
    }
    pub fn write(&self, socket: &mut TcpStream) -> io::Result<()> {
        self.validate()?;
        let metadata = serde_json::to_vec(&Metadata {
            owner: self.owner,
            body: self.body.clone(),
        })
        .map_err(invalid)?;
        if metadata.len() > MAX_METADATA_BYTES {
            return Err(invalid("bridge metadata too large"));
        }
        let mut prefix = [0u8; PREFIX];
        prefix[..4].copy_from_slice(MAGIC);
        prefix[4..8].copy_from_slice(&VERSION.to_le_bytes());
        prefix[8..16].copy_from_slice(&self.request.to_le_bytes());
        prefix[16..20].copy_from_slice(&(metadata.len() as u32).to_le_bytes());
        prefix[20..24].copy_from_slice(&(self.state.len() as u32).to_le_bytes());
        socket.write_all(&prefix)?;
        socket.write_all(&metadata)?;
        socket.write_all(&self.state)
    }
}
fn invalid(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

/// Incremental framing survives short/nonblocking/timeout reads without a
/// second reader, abandoned blocked threads or duplicated control requests.
#[derive(Default)]
pub struct Decoder {
    bytes: Vec<u8>,
}
impl Decoder {
    pub fn poll(&mut self, socket: &mut TcpStream) -> io::Result<Option<Packet>> {
        if let Some(packet) = self.parse()? {
            return Ok(Some(packet));
        }
        let mut chunk = [0u8; 8192];
        match socket.read(&mut chunk) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "audio helper control connection closed",
                ));
            }
            Ok(length) => {
                if self
                    .bytes
                    .len()
                    .checked_add(length)
                    .is_none_or(|size| size > MAX_PACKET_BYTES)
                {
                    return Err(invalid("bridge control buffer too large"));
                }
                self.bytes.extend_from_slice(&chunk[..length]);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        }
        self.parse()
    }
    fn parse(&mut self) -> io::Result<Option<Packet>> {
        if self.bytes.len() < PREFIX {
            return Ok(None);
        }
        let prefix = &self.bytes[..PREFIX];
        if &prefix[..4] != MAGIC
            || u32::from_le_bytes(prefix[4..8].try_into().expect("prefix")) != VERSION
        {
            return Err(invalid("unsupported bridge control version"));
        }
        let request = u64::from_le_bytes(prefix[8..16].try_into().expect("prefix"));
        let metadata_len = u32::from_le_bytes(prefix[16..20].try_into().expect("prefix")) as usize;
        let state = u32::from_le_bytes(prefix[20..24].try_into().expect("prefix")) as usize;
        if metadata_len == 0 || metadata_len > MAX_METADATA_BYTES || state > MAX_STATE_BYTES {
            return Err(invalid("invalid bridge control lengths"));
        }
        let total = PREFIX + metadata_len + state;
        if self.bytes.len() < total {
            return Ok(None);
        }
        let metadata: Metadata =
            serde_json::from_slice(&self.bytes[PREFIX..PREFIX + metadata_len]).map_err(invalid)?;
        let packet = Packet {
            request,
            owner: metadata.owner,
            body: metadata.body,
            state: self.bytes[PREFIX + metadata_len..total].to_vec(),
        };
        packet.validate()?;
        self.bytes.drain(..total);
        Ok(Some(packet))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_and_oversized_control_headers_fail_before_payload_allocation() {
        for length in [MAX_METADATA_BYTES + 1, u32::MAX as usize] {
            let mut decoder = Decoder::default();
            decoder.bytes.extend_from_slice(MAGIC);
            decoder.bytes.extend_from_slice(&VERSION.to_le_bytes());
            decoder.bytes.extend_from_slice(&1u64.to_le_bytes());
            decoder
                .bytes
                .extend_from_slice(&(length as u32).to_le_bytes());
            decoder.bytes.extend_from_slice(&0u32.to_le_bytes());
            assert!(decoder.parse().is_err());
            assert_eq!(decoder.bytes.len(), PREFIX);
        }
    }
}
