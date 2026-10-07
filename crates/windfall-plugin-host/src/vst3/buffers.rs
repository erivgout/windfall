//! Fixed-capacity SDK event and parameter COM objects, allocated at activation.
use crate::processor::EVENT_CAPACITY;
use std::cell::{Cell, UnsafeCell};
use std::rc::Rc;
use vst3::Steinberg::Vst::*;
use vst3::{Class, ComPtr, ComWrapper, Steinberg::*};

#[derive(Clone, Copy)]
struct Point {
    offset: i32,
    value: f64,
    next: Option<usize>,
}

struct Arena {
    points: UnsafeCell<Vec<Point>>,
    used: Cell<usize>,
    dropped: Cell<u32>,
}
impl Arena {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            points: UnsafeCell::new(vec![
                Point {
                    offset: 0,
                    value: 0.0,
                    next: None
                };
                EVENT_CAPACITY
            ]),
            used: Cell::new(0),
            dropped: Cell::new(0),
        })
    }
    fn drop_point(&self) {
        self.dropped.set(self.dropped.get().saturating_add(1));
    }
}
struct Queue {
    id: Cell<u32>,
    first: Cell<Option<usize>>,
    last: Cell<Option<usize>>,
    count: Cell<usize>,
    arena: Rc<Arena>,
}
impl Class for Queue {
    type Interfaces = (IParamValueQueue,);
}
#[allow(non_snake_case)]
impl IParamValueQueueTrait for Queue {
    unsafe fn getParameterId(&self) -> u32 {
        self.id.get()
    }
    unsafe fn getPointCount(&self) -> i32 {
        self.count.get() as i32
    }
    unsafe fn getPoint(&self, index: i32, offset: *mut i32, value: *mut f64) -> tresult {
        if index < 0 || index as usize >= self.count.get() || offset.is_null() || value.is_null() {
            return kInvalidArgument;
        }
        let Some(mut at) = self.first.get() else {
            return kInvalidArgument;
        };
        // SAFETY: arena is used exclusively during this synchronous audio call;
        // its storage never grows, and links only point to allocated slots.
        let points = unsafe { &*self.arena.points.get() };
        for _ in 0..index {
            let Some(next) = points[at].next else {
                return kInvalidArgument;
            };
            at = next;
        }
        // SAFETY: SDK caller supplies output pointers, checked non-null.
        unsafe {
            offset.write(points[at].offset);
            value.write(points[at].value);
        }
        kResultOk
    }
    unsafe fn addPoint(&self, offset: i32, value: f64, index: *mut i32) -> tresult {
        if offset < 0 || !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return kInvalidArgument;
        }
        let at = self.arena.used.get();
        if at == EVENT_CAPACITY {
            self.arena.drop_point();
            return kResultFalse;
        }
        // SAFETY: audio-call-exclusive arena, fixed storage and bounded index.
        let points = unsafe { &mut *self.arena.points.get() };
        if let Some(last) = self.last.get() {
            if points[last].offset > offset {
                return kInvalidArgument;
            }
            points[last].next = Some(at);
        } else {
            self.first.set(Some(at));
        }
        points[at] = Point {
            offset,
            value,
            next: None,
        };
        self.last.set(Some(at));
        self.arena.used.set(at + 1);
        // SAFETY: optional SDK output pointer.
        if !index.is_null() {
            unsafe {
                index.write(self.count.get() as i32);
            }
        }
        self.count.set(self.count.get() + 1);
        kResultOk
    }
}
struct Slot {
    queue: ComWrapper<Queue>,
    interface: ComPtr<IParamValueQueue>,
}
pub(super) struct Changes {
    queues: Vec<Slot>,
    active: Cell<usize>,
    arena: Rc<Arena>,
}
impl Changes {
    pub fn new() -> ComWrapper<Self> {
        let arena = Arena::new();
        let queues = (0..EVENT_CAPACITY)
            .map(|_| {
                let queue = ComWrapper::new(Queue {
                    id: Cell::new(0),
                    first: Cell::new(None),
                    last: Cell::new(None),
                    count: Cell::new(0),
                    arena: Rc::clone(&arena),
                });
                let interface = queue.to_com_ptr().expect("queue interface");
                Slot { queue, interface }
            })
            .collect();
        ComWrapper::new(Self {
            queues,
            active: Cell::new(0),
            arena,
        })
    }
    pub fn clear(&self) {
        for slot in self.queues.iter().take(self.active.get()) {
            slot.queue.first.set(None);
            slot.queue.last.set(None);
            slot.queue.count.set(0);
        }
        self.active.set(0);
        self.arena.used.set(0);
        self.arena.dropped.set(0);
    }
    pub fn push(&self, id: u32, offset: u32, value: f64) {
        let mut index = 0;
        // SAFETY: synchronous call on exclusively owned COM objects and valid args.
        unsafe {
            let queue = self.addParameterData(&id, &mut index);
            if let Some(queue) = vst3::ComRef::from_raw(queue) {
                queue.addPoint(offset as i32, value, &mut index);
            }
        }
    }
    pub fn visit(&self, out: &mut dyn FnMut(u32, u32, f64)) {
        // SAFETY: caller owns this arena; SDK calls have returned.
        let points = unsafe { &*self.arena.points.get() };
        for slot in self.queues.iter().take(self.active.get()) {
            let mut at = slot.queue.first.get();
            for _ in 0..slot.queue.count.get() {
                let Some(index) = at else {
                    break;
                };
                let point = points[index];
                out(slot.queue.id.get(), point.offset as u32, point.value);
                at = point.next;
            }
        }
    }
    pub fn dropped(&self) -> u32 {
        self.arena.dropped.get()
    }
}
impl Class for Changes {
    type Interfaces = (IParameterChanges,);
}
#[allow(non_snake_case)]
impl IParameterChangesTrait for Changes {
    unsafe fn getParameterCount(&self) -> i32 {
        self.active.get() as i32
    }
    unsafe fn getParameterData(&self, index: i32) -> *mut IParamValueQueue {
        if index < 0 || index as usize >= self.active.get() {
            return std::ptr::null_mut();
        }
        self.queues[index as usize].interface.as_ptr()
    }
    unsafe fn addParameterData(&self, id: *const u32, index: *mut i32) -> *mut IParamValueQueue {
        if id.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: SDK caller supplies a readable parameter-id pointer.
        let id = unsafe { id.read() };
        if id == u32::MAX {
            return std::ptr::null_mut();
        }
        let active = self.active.get();
        let at = match self
            .queues
            .iter()
            .take(active)
            .position(|slot| slot.queue.id.get() == id)
        {
            Some(at) => at,
            None if active < self.queues.len() => {
                self.queues[active].queue.id.set(id);
                self.active.set(active + 1);
                active
            }
            None => {
                self.arena.drop_point();
                return std::ptr::null_mut();
            }
        };
        // SAFETY: optional SDK output pointer.
        if !index.is_null() {
            unsafe {
                index.write(at as i32);
            }
        }
        self.queues[at].interface.as_ptr()
    }
}

pub(super) struct Events {
    data: UnsafeCell<Vec<Event>>,
    len: Cell<usize>,
    dropped: Cell<u32>,
}
impl Events {
    pub fn new() -> ComWrapper<Self> {
        // SAFETY: Event is an SDK POD struct; a zeroed unused slot is valid storage.
        let zero = unsafe { std::mem::zeroed() };
        ComWrapper::new(Self {
            data: UnsafeCell::new(vec![zero; EVENT_CAPACITY + 2048]),
            len: Cell::new(0),
            dropped: Cell::new(0),
        })
    }
    pub fn clear(&self) {
        self.len.set(0);
        self.dropped.set(0);
    }
    pub fn push(&self, mut event: Event) {
        // SAFETY: event is a live SDK POD struct, used only synchronously.
        unsafe {
            self.addEvent(&mut event);
        }
    }
    pub fn dropped(&self) -> u32 {
        self.dropped.get()
    }
}
impl Class for Events {
    type Interfaces = (IEventList,);
}
#[allow(non_snake_case)]
impl IEventListTrait for Events {
    unsafe fn getEventCount(&self) -> i32 {
        self.len.get() as i32
    }
    unsafe fn getEvent(&self, index: i32, event: *mut Event) -> tresult {
        if index < 0 || index as usize >= self.len.get() || event.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: exclusively used fixed list, bounded index, writable SDK output.
        unsafe {
            event.write((&*self.data.get())[index as usize]);
        }
        kResultOk
    }
    unsafe fn addEvent(&self, event: *mut Event) -> tresult {
        if event.is_null() {
            return kInvalidArgument;
        }
        let at = self.len.get();
        if at == EVENT_CAPACITY + 2048 {
            self.dropped.set(self.dropped.get().saturating_add(1));
            return kResultFalse;
        }
        // SAFETY: synchronous SDK caller supplies Event storage; fixed list is exclusive.
        let event = unsafe { event.read() };
        // Only pointer-free notes are retained. SysEx/text pointers do not
        // survive the plugin call and are not part of the public host API.
        if event.r#type > 1 || event.sampleOffset < 0 {
            return kNotImplemented;
        }
        // SAFETY: bounded index into fixed exclusive storage.
        unsafe {
            (&mut *self.data.get())[at] = event;
        }
        self.len.set(at + 1);
        kResultOk
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parameter_queues_preserve_offsets_and_enforce_capacity() {
        let changes = Changes::new();
        changes.push(7, 3, 0.25);
        changes.push(9, 5, 0.5);
        changes.push(7, 11, 0.75);
        let mut points = Vec::new();
        changes.visit(&mut |id, time, value| points.push((id, time, value)));
        assert_eq!(points, [(7, 3, 0.25), (7, 11, 0.75), (9, 5, 0.5)]);
        changes.clear();
        for index in 0..EVENT_CAPACITY + 1 {
            changes.push(7, index as u32, 0.5);
        }
        assert_eq!(changes.dropped(), 1);
        // SAFETY: COM methods on owned bounded test buffers.
        unsafe {
            assert!(changes.getParameterData(-1).is_null());
            assert!(changes.getParameterData(1).is_null());
        }
        changes.clear();
        assert_eq!(changes.dropped(), 0);
        assert_eq!(unsafe { changes.getParameterCount() }, 0);
    }
    #[test]
    fn malformed_sdk_points_and_events_are_rejected() {
        let changes = Changes::new();
        let mut index = 0;
        // SAFETY: valid synchronous SDK arguments and owned COM buffers.
        unsafe {
            assert!(
                changes
                    .addParameterData(std::ptr::null(), &mut index)
                    .is_null()
            );
            let queue = vst3::ComRef::from_raw(changes.addParameterData(&7, &mut index)).unwrap();
            assert_eq!(queue.addPoint(-1, 0.5, &mut index), kInvalidArgument);
            assert_eq!(queue.addPoint(0, f64::NAN, &mut index), kInvalidArgument);
            assert_eq!(queue.addPoint(0, 1.5, &mut index), kInvalidArgument);
            assert_eq!(queue.addPoint(4, 0.5, &mut index), kResultOk);
            assert_eq!(queue.addPoint(2, 0.25, &mut index), kInvalidArgument);
            assert_eq!(
                queue.getPoint(0, std::ptr::null_mut(), std::ptr::null_mut()),
                kInvalidArgument
            );
            let events = Events::new();
            let mut event: Event = std::mem::zeroed();
            event.r#type = 2;
            assert_eq!(events.addEvent(&mut event), kNotImplemented);
            assert_eq!(events.getEvent(0, &mut event), kInvalidArgument);
        }
    }
}
