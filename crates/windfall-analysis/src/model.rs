use crate::{AnalysisError, AudioShape, CHUNK_BYTES, Result, Work, add, lock};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelProvenance {
    pub origin: String,
    pub source_revision: String,
    pub author: String,
    pub license_spdx: String,
    pub license_reference: String,
    pub adapter_id: String,
    pub adapter_version: String,
    /// This first contract requires a deterministic CPU adapter.
    pub device: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputRole {
    pub role: String,
    pub channels: u16,
}

/// A fixed, fully specified local model. Shapes are exact; outputs preserve
/// selected frame count, sample rate and original selection frame origin.
/// Weight licensing is recorded independently of an adapter's code license.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelManifest {
    pub id: String,
    pub version: String,
    pub revision: u64,
    pub sha256: [u8; 32],
    pub bytes: u64,
    pub max_bytes: u64,
    pub provenance: ModelProvenance,
    pub sample_rate: u32,
    pub input_channels: u16,
    pub max_input_frames: u64,
    pub outputs: Vec<OutputRole>,
}
impl ModelManifest {
    pub fn validate(&self) -> Result<()> {
        let p = &self.provenance;
        for field in [
            &self.id,
            &self.version,
            &p.origin,
            &p.source_revision,
            &p.author,
            &p.license_spdx,
            &p.license_reference,
            &p.adapter_id,
            &p.adapter_version,
        ] {
            if field.is_empty() || field.len() > 1024 || field.chars().any(char::is_control) {
                return Err(AnalysisError::Invalid("model provenance fields"));
            }
        }
        if self.bytes == 0
            || self.bytes > self.max_bytes
            || self.max_bytes > usize::MAX as u64
            || self.sample_rate < 8_000
            || self.sample_rate > 192_000
            || !(1..=2).contains(&self.input_channels)
            || self.max_input_frames == 0
            || self.outputs.is_empty()
            || self.outputs.len() > 16
            || p.device != "cpu"
        {
            return Err(AnalysisError::Invalid("model byte/shape/device bounds"));
        }
        for (i, output) in self.outputs.iter().enumerate() {
            if output.role.is_empty()
                || output.role.len() > 64
                || !output
                    .role
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || !(1..=2).contains(&output.channels)
                || self.outputs[..i].iter().any(|r| r.role == output.role)
            {
                return Err(AnalysisError::Invalid("model output roles"));
            }
        }
        Ok(())
    }
    pub fn validate_input(&self, shape: AudioShape, frames: u64) -> Result<()> {
        self.validate()?;
        if shape.sample_rate != self.sample_rate
            || shape.channels != self.input_channels
            || frames == 0
            || frames > self.max_input_frames
        {
            return Err(AnalysisError::Invalid("model/input shape mismatch"));
        }
        Ok(())
    }
    pub(crate) fn cache_name(&self) -> String {
        let mut name = String::with_capacity(70);
        for byte in self.sha256 {
            use std::fmt::Write;
            let _ = write!(name, "{byte:02x}");
        }
        name.push_str(".model");
        name
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ModelCacheLimits {
    pub max_model_bytes: u64,
    pub max_cache_bytes: u64,
    pub max_entries: usize,
    pub max_imports: usize,
}
impl Default for ModelCacheLimits {
    fn default() -> Self {
        Self {
            max_model_bytes: 256 * 1024 * 1024,
            max_cache_bytes: 1024 * 1024 * 1024,
            max_entries: 32,
            max_imports: 1,
        }
    }
}
struct CacheState {
    importing: usize,
    reserved: u64,
}
struct CacheInner {
    root: PathBuf,
    limits: ModelCacheLimits,
    state: Mutex<CacheState>,
}

/// One control authority per dedicated cache directory; Clone shares its
/// reservations. External competitors are never replaced/deleted. A cache
/// hit is verified from content, not accepted from name/metadata alone.
#[derive(Clone)]
pub struct ModelCache(Arc<CacheInner>);

pub struct VerifiedModel {
    manifest: ModelManifest,
    bytes: Box<[u8]>,
}
impl VerifiedModel {
    pub fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl ModelCache {
    pub fn new(root: impl AsRef<Path>, limits: ModelCacheLimits) -> Result<Self> {
        if limits.max_model_bytes == 0
            || limits.max_model_bytes > limits.max_cache_bytes
            || limits.max_entries == 0
            || limits.max_imports == 0
            || limits.max_model_bytes > usize::MAX as u64
        {
            return Err(AnalysisError::Invalid("model cache bounds"));
        }
        fs::create_dir_all(root.as_ref())?;
        let root = fs::canonicalize(root)?;
        inventory(&root, limits)?;
        Ok(Self(Arc::new(CacheInner {
            root,
            limits,
            state: Mutex::new(CacheState {
                importing: 0,
                reserved: 0,
            }),
        })))
    }
    pub fn root(&self) -> &Path {
        &self.0.root
    }
    fn validate(&self, manifest: &ModelManifest) -> Result<()> {
        manifest.validate()?;
        if manifest.max_bytes > self.0.limits.max_model_bytes {
            return Err(AnalysisError::Budget("model bytes"));
        }
        Ok(())
    }
    /// Explicit local import. The supplied reader runs on the calling worker;
    /// streaming reads/writes/hash checks are chunked and cancellable. No URL
    /// fetching, uploads, weights discovery or automatic replacement exists.
    pub fn import_reader(
        &self,
        manifest: &ModelManifest,
        mut reader: impl Read,
        work: &mut Work,
    ) -> Result<PathBuf> {
        self.validate(manifest)?;
        work.check()?;
        let target = self.0.root.join(manifest.cache_name());
        if fs::symlink_metadata(&target).is_ok() {
            verify_file(&target, manifest, work, false)?;
            return Ok(target);
        }
        let reservation = {
            let mut state = lock(&self.0.state);
            let (used, entries) = inventory(&self.0.root, self.0.limits)?;
            if state.importing >= self.0.limits.max_imports
                || entries + state.importing >= self.0.limits.max_entries
            {
                return Err(AnalysisError::Budget("model import/entry count"));
            }
            if add(add(used, state.reserved)?, manifest.bytes)? > self.0.limits.max_cache_bytes {
                return Err(AnalysisError::Budget("model cache disk"));
            }
            state.importing += 1;
            state.reserved += manifest.bytes;
            ImportReservation {
                cache: self.clone(),
                bytes: manifest.bytes,
            }
        };
        let mut staged = NamedTempFile::new_in(&self.0.root)?;
        let mut chunk = [0; CHUNK_BYTES];
        let mut bytes = 0;
        let mut sha = Sha256::new();
        loop {
            work.check()?;
            let n = reader.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            bytes = add(bytes, n as u64)?;
            if bytes > manifest.bytes {
                return Err(AnalysisError::ModelSize);
            }
            work.checkpoint(n as u64)?;
            staged.write_all(&chunk[..n])?;
            sha.update(&chunk[..n]);
        }
        if bytes != manifest.bytes {
            return Err(AnalysisError::ModelSize);
        }
        if <[u8; 32]>::from(sha.finalize()) != manifest.sha256 {
            return Err(AnalysisError::Checksum);
        }
        staged.as_file().sync_all()?;
        work.check()?;
        match staged.persist_noclobber(&target) {
            Ok(file) => {
                drop(file);
            }
            Err(error)
                if error.error.kind() == std::io::ErrorKind::AlreadyExists
                    || fs::symlink_metadata(&target).is_ok() =>
            {
                // `error.file` owns only our staging. A competing final file is
                // left byte-for-byte intact, then checked rather than trusted.
                drop(error.file);
                verify_file(&target, manifest, work, false)?;
            }
            Err(error) => return Err(AnalysisError::Io(error.error)),
        }
        drop(reservation);
        Ok(target)
    }
    pub fn import_file(
        &self,
        manifest: &ModelManifest,
        path: impl AsRef<Path>,
        work: &mut Work,
    ) -> Result<PathBuf> {
        self.import_reader(manifest, File::open(path)?, work)
    }
    /// Rehash and copy to bounded immutable memory. A worker never infers from
    /// a mutable pathname or trusts a prior import's checksum check.
    pub fn load(&self, manifest: &ModelManifest, work: &mut Work) -> Result<VerifiedModel> {
        self.validate(manifest)?;
        let bytes = verify_file(
            &self.0.root.join(manifest.cache_name()),
            manifest,
            work,
            true,
        )?;
        Ok(VerifiedModel {
            manifest: manifest.clone(),
            bytes: bytes.into_boxed_slice(),
        })
    }
}

struct ImportReservation {
    cache: ModelCache,
    bytes: u64,
}
impl Drop for ImportReservation {
    fn drop(&mut self) {
        let mut s = lock(&self.cache.0.state);
        s.importing -= 1;
        s.reserved -= self.bytes;
    }
}

fn verify_file(
    path: &Path,
    manifest: &ModelManifest,
    work: &mut Work,
    retain: bool,
) -> Result<Vec<u8>> {
    work.check()?;
    let metadata = fs::symlink_metadata(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AnalysisError::ModelAbsent
        } else {
            e.into()
        }
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(AnalysisError::Invalid(
            "model cache entry is not a regular file",
        ));
    }
    let mut file = File::open(path)?;
    if file.metadata()?.len() != manifest.bytes {
        return Err(AnalysisError::ModelSize);
    }
    let mut data = Vec::new();
    if retain {
        data.try_reserve_exact(manifest.bytes as usize)
            .map_err(|_| AnalysisError::Budget("model allocation"))?;
        if data.capacity() as u64 > manifest.bytes {
            return Err(AnalysisError::Budget("model capacity"));
        }
    }
    let mut chunk = [0; CHUNK_BYTES];
    let mut sha = Sha256::new();
    let mut bytes = 0;
    loop {
        work.check()?;
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        bytes = add(bytes, n as u64)?;
        if bytes > manifest.bytes {
            return Err(AnalysisError::ModelSize);
        }
        work.checkpoint(n as u64)?;
        sha.update(&chunk[..n]);
        if retain {
            data.extend_from_slice(&chunk[..n]);
        }
    }
    work.check()?;
    if bytes != manifest.bytes {
        return Err(AnalysisError::ModelSize);
    }
    if <[u8; 32]>::from(sha.finalize()) != manifest.sha256 {
        return Err(AnalysisError::Checksum);
    }
    Ok(data)
}

pub(crate) fn inventory(root: &Path, limits: ModelCacheLimits) -> Result<(u64, usize)> {
    let mut bytes = 0;
    let mut count = 0;
    for entry in fs::read_dir(root)? {
        count += 1;
        if count > limits.max_entries {
            return Err(AnalysisError::Budget("directory entry count"));
        }
        let metadata = fs::symlink_metadata(entry?.path())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(AnalysisError::Invalid("cache/output directory entry"));
        }
        bytes = add(bytes, metadata.len())?;
        if bytes > limits.max_cache_bytes {
            return Err(AnalysisError::Budget("directory disk bytes"));
        }
    }
    Ok((bytes, count))
}
