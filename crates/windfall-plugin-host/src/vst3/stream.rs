//! Bounded synchronous SDK state streams, used away from the audio thread.
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ptr;
use vst3::{Class, ComWrapper, Steinberg::*};

pub(super) struct Stream {
    data: RefCell<Data>,
    limit: usize,
    writable: bool,
    failed: Cell<bool>,
}

struct Data {
    bytes: Vec<u8>,
    at: usize,
}
impl Stream {
    pub fn new(bytes: Vec<u8>, limit: usize, writable: bool) -> ComWrapper<Self> {
        ComWrapper::new(Self {
            data: RefCell::new(Data { bytes, at: 0 }),
            limit,
            writable,
            failed: Cell::new(false),
        })
    }
    pub fn bytes(&self) -> Vec<u8> {
        self.data.borrow().bytes.clone()
    }
    pub fn rewind(&self) {
        self.data.borrow_mut().at = 0;
    }
    pub fn failed(&self) -> bool {
        self.failed.get()
    }
}
impl Class for Stream {
    type Interfaces = (IBStream,);
}
#[allow(non_snake_case)]
impl IBStreamTrait for Stream {
    unsafe fn read(&self, buffer: *mut c_void, numBytes: i32, numBytesRead: *mut i32) -> tresult {
        if numBytes < 0 || (numBytes > 0 && buffer.is_null()) {
            return kInvalidArgument;
        }
        let Ok(mut data) = self.data.try_borrow_mut() else {
            return kInternalError;
        };
        let count = (numBytes as usize).min(data.bytes.len().saturating_sub(data.at));
        // SAFETY: caller supplies numBytes of writable storage; source range is bounded.
        unsafe {
            if count > 0 {
                ptr::copy_nonoverlapping(data.bytes.as_ptr().add(data.at), buffer.cast(), count);
            }
            if !numBytesRead.is_null() {
                numBytesRead.write(count as i32);
            }
        }
        data.at += count;
        if count == 0 && numBytes > 0 {
            kResultFalse
        } else {
            kResultOk
        }
    }
    unsafe fn write(
        &self,
        buffer: *mut c_void,
        numBytes: i32,
        numBytesWritten: *mut i32,
    ) -> tresult {
        if numBytes < 0 || (numBytes > 0 && buffer.is_null()) || !self.writable {
            self.failed.set(true);
            return kInvalidArgument;
        }
        // SAFETY: optional SDK output pointer is writable when supplied.
        if !numBytesWritten.is_null() {
            unsafe {
                numBytesWritten.write(0);
            }
        }
        let Ok(mut data) = self.data.try_borrow_mut() else {
            return kInternalError;
        };
        let Some(end) = data
            .at
            .checked_add(numBytes as usize)
            .filter(|&end| end <= self.limit)
        else {
            self.failed.set(true);
            return kOutOfMemory;
        };
        if end > data.bytes.len() {
            data.bytes.resize(end, 0);
        }
        // SAFETY: caller supplies numBytes of readable storage; destination is sized.
        unsafe {
            if numBytes > 0 {
                ptr::copy_nonoverlapping(
                    buffer.cast(),
                    data.bytes.as_mut_ptr().add(data.at),
                    numBytes as usize,
                );
            }
            if !numBytesWritten.is_null() {
                numBytesWritten.write(numBytes);
            }
        }
        data.at = end;
        kResultOk
    }
    unsafe fn seek(&self, pos: i64, mode: i32, result: *mut i64) -> tresult {
        let Ok(mut data) = self.data.try_borrow_mut() else {
            return kInternalError;
        };
        let base = match mode {
            0 => 0,
            1 => data.at as i64,
            2 => data.bytes.len() as i64,
            _ => return kInvalidArgument,
        };
        let Some(at) = base
            .checked_add(pos)
            .filter(|&at| at >= 0 && at as u64 <= self.limit as u64)
        else {
            return kInvalidArgument;
        };
        data.at = at as usize;
        // SAFETY: optional SDK output storage.
        if !result.is_null() {
            unsafe {
                result.write(at);
            }
        }
        kResultOk
    }
    unsafe fn tell(&self, pos: *mut i64) -> tresult {
        if pos.is_null() {
            return kInvalidArgument;
        }
        let Ok(data) = self.data.try_borrow() else {
            return kInternalError;
        };
        // SAFETY: SDK caller supplies writable output storage.
        unsafe {
            pos.write(data.at as i64);
        }
        kResultOk
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn write_limit_is_sticky_even_when_plugin_ignores_the_error() {
        let stream = Stream::new(Vec::new(), 4, true);
        let mut bytes = [1_u8; 8];
        let mut count = -1;
        // SAFETY: valid synchronous stream storage and SDK output pointer.
        unsafe {
            assert_eq!(
                stream.write(bytes.as_mut_ptr().cast(), 8, &mut count),
                kOutOfMemory
            );
        }
        assert_eq!(count, 0);
        assert!(stream.failed());
        assert!(stream.bytes().is_empty());
    }
    #[test]
    fn seek_read_and_eof_never_escape_the_input() {
        let stream = Stream::new(vec![1, 2, 3, 4], 4, false);
        let mut at = -1;
        let mut bytes = [0_u8; 8];
        let mut count = -1;
        // SAFETY: all pointers refer to the output arrays/integers above.
        unsafe {
            assert_eq!(stream.seek(-2, 2, &mut at), kResultOk);
            assert_eq!(at, 2);
            assert_eq!(
                stream.read(bytes.as_mut_ptr().cast(), 8, &mut count),
                kResultOk
            );
            assert_eq!(count, 2);
            assert_eq!(&bytes[..2], &[3, 4]);
            assert_eq!(
                stream.read(bytes.as_mut_ptr().cast(), 8, &mut count),
                kResultFalse
            );
            assert_eq!(count, 0);
            assert_eq!(stream.seek(i64::MAX, 1, &mut at), kInvalidArgument);
        }
    }
}
