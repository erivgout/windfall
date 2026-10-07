//! Main-thread controller edits and atomic restart requests.
use crate::events::PluginEvent;
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use vst3::Steinberg::Vst::*;
use vst3::{Class, ComWrapper, Steinberg::*};
thread_local! { static MARK: u8 = const { 0 }; static AUDIO: Cell<bool> = const { Cell::new(false) }; }
pub(super) fn token() -> usize {
    MARK.with(|mark| std::ptr::from_ref(mark) as usize)
}
pub(super) struct AudioCall(bool);
impl AudioCall {
    pub fn enter() -> Self {
        Self(AUDIO.replace(true))
    }
}
impl Drop for AudioCall {
    fn drop(&mut self) {
        AUDIO.set(self.0);
    }
}
pub(super) struct Handler {
    main: usize,
    pub flags: AtomicI32,
    pub dirty: AtomicBool,
    events: RefCell<Vec<PluginEvent>>,
    edits: RefCell<Option<rtrb::Producer<(u32, f64)>>>,
}
impl Handler {
    pub fn new() -> ComWrapper<Self> {
        ComWrapper::new(Self {
            main: token(),
            flags: AtomicI32::new(0),
            dirty: AtomicBool::new(false),
            events: RefCell::new(Vec::with_capacity(4096)),
            edits: RefCell::new(None),
        })
    }
    fn is_main(&self) -> bool {
        !AUDIO.get() && token() == self.main
    }
    fn event(&self, event: PluginEvent) -> tresult {
        if !self.is_main() {
            return kResultFalse;
        }
        let Ok(mut events) = self.events.try_borrow_mut() else {
            return kResultFalse;
        };
        if events.len() == events.capacity() {
            return kResultFalse;
        }
        events.push(event);
        kResultOk
    }
    pub fn set_queue(&self, queue: Option<rtrb::Producer<(u32, f64)>>) {
        *self.edits.borrow_mut() = queue;
    }
    pub fn drain(&self, out: &mut dyn FnMut(PluginEvent)) {
        for event in self.events.borrow_mut().drain(..) {
            out(event);
        }
    }
}
impl Class for Handler {
    type Interfaces = (IComponentHandler, IComponentHandler2);
}

#[allow(non_snake_case)]
impl IComponentHandler2Trait for Handler {
    unsafe fn setDirty(&self, state: TBool) -> tresult {
        if !self.is_main() {
            return kResultFalse;
        }
        if state != 0 {
            self.dirty.store(true, Ordering::Relaxed);
        }
        kResultOk
    }
    unsafe fn requestOpenEditor(&self, _name: FIDString) -> tresult {
        kNotImplemented
    }
    unsafe fn startGroupEdit(&self) -> tresult {
        kNotImplemented
    }
    unsafe fn finishGroupEdit(&self) -> tresult {
        kNotImplemented
    }
}

#[allow(non_snake_case)]
impl IComponentHandlerTrait for Handler {
    unsafe fn beginEdit(&self, id: u32) -> tresult {
        self.event(PluginEvent::GestureBegin { id })
    }
    unsafe fn performEdit(&self, id: u32, value: f64) -> tresult {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) || id == u32::MAX {
            return kInvalidArgument;
        }
        let result = self.event(PluginEvent::ParamValue { id, value });
        if result != kResultOk {
            return result;
        }
        if let Ok(mut edits) = self.edits.try_borrow_mut()
            && let Some(queue) = edits.as_mut()
            && queue.push((id, value)).is_err()
        {
            return kResultFalse;
        }
        kResultOk
    }
    unsafe fn endEdit(&self, id: u32) -> tresult {
        self.event(PluginEvent::GestureEnd { id })
    }
    unsafe fn restartComponent(&self, flags: i32) -> tresult {
        self.flags.fetch_or(flags, Ordering::Relaxed);
        kResultOk
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdk_edits_queue_normalized_values_and_reject_audio_thread_callbacks() {
        let handler = Handler::new();
        let (producer, mut consumer) = rtrb::RingBuffer::new(2);
        handler.set_queue(Some(producer));
        // SAFETY: owned SDK handler, valid synchronous test arguments.
        unsafe {
            assert_eq!(handler.beginEdit(7), kResultOk);
            assert_eq!(handler.performEdit(7, 0.25), kResultOk);
            assert_eq!(handler.endEdit(7), kResultOk);
            assert_eq!(handler.setDirty(1), kResultOk);
        }
        assert_eq!(consumer.pop(), Ok((7, 0.25)));
        assert!(handler.dirty.load(Ordering::Relaxed));
        let mut events = Vec::new();
        handler.drain(&mut |e| events.push(e));
        assert_eq!(
            events,
            [
                PluginEvent::GestureBegin { id: 7 },
                PluginEvent::ParamValue { id: 7, value: 0.25 },
                PluginEvent::GestureEnd { id: 7 }
            ]
        );
        let _audio = AudioCall::enter();
        unsafe {
            assert_eq!(handler.performEdit(7, 0.75), kResultFalse);
            assert_eq!(handler.setDirty(1), kResultFalse);
            assert_eq!(handler.restartComponent(8), kResultOk);
        }
        assert_eq!(handler.flags.load(Ordering::Relaxed), 8);
        assert!(consumer.pop().is_err());
    }
}
