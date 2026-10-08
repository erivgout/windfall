//! Fixed-cost history and delay transitions, private to the delay family.
use super::preparation::{PreparationBudget, PreparationError, PreparationRequirements};
use crate::blocks::smooth::LinearRamp;

pub(crate) fn audio(x: f32) -> f32 {
    if x.is_finite() {
        x.clamp(-1000.0, 1000.0)
    } else {
        0.0
    }
}
pub(crate) fn bounded(x: f32) -> f32 {
    if x.is_finite() {
        x.clamp(-64.0, 64.0)
    } else {
        0.0
    }
}
pub(crate) fn rate(x: f32) -> f32 {
    crate::blocks::math::clean(x, 1.0, 384_000.0, 48_000.0)
}
pub(crate) fn balance(x: [f32; 2], pan: f32) -> [f32; 2] {
    [x[0] * (1.0 - pan.max(0.0)), x[1] * (1.0 + pan.min(0.0))]
}
#[derive(Default)]
pub(crate) struct History {
    data: Vec<f32>,
    write: usize,
    valid: usize,
}
impl History {
    fn try_new(maximum: usize, index: usize) -> Result<Self, PreparationError> {
        let mut data = Vec::new();
        let samples = maximum
            .checked_add(1)
            .ok_or(PreparationError::CapacityOverflow {
                maximum_delay_samples: maximum,
                history_count: 1,
            })?;
        let bytes = samples
            .checked_mul(size_of::<f32>())
            .filter(|&n| n <= isize::MAX as usize)
            .ok_or(PreparationError::CapacityOverflow {
                maximum_delay_samples: maximum,
                history_count: 1,
            })?;
        reserve_history(&mut data, samples).map_err(|_| PreparationError::ReservationFailed {
            history_index: index,
            requested_bytes: bytes,
        })?;
        // Capacity was reserved fallibly. Initialization cannot allocate.
        data.resize(samples, 0.0);
        #[cfg(test)]
        allocation_test::created(data.capacity() * size_of::<f32>());
        Ok(Self {
            data,
            write: 0,
            valid: 0,
        })
    }
    pub fn reset(&mut self) {
        self.write = 0;
        self.valid = 0;
    }
    pub fn valid(&self) -> usize {
        self.valid
    }
    pub fn bytes(&self) -> usize {
        self.data.capacity() * std::mem::size_of::<f32>()
    }
    pub fn read(&self, delay: usize, current: f32) -> f32 {
        if delay == 0 {
            return current;
        }
        if delay > self.valid || self.data.is_empty() {
            return 0.0;
        }
        let index = (self.write + self.data.len() - delay) % self.data.len();
        self.data[index]
    }
    pub fn push(&mut self, value: f32) {
        if self.data.is_empty() {
            return;
        }
        self.data[self.write] = value;
        self.write += 1;
        if self.write == self.data.len() {
            self.write = 0;
        }
        self.valid = (self.valid + 1).min(self.data.len() - 1);
    }
}

fn reserve_history(
    data: &mut Vec<f32>,
    samples: usize,
) -> Result<(), std::collections::TryReserveError> {
    #[cfg(test)]
    let _fault = allocation_test::reservation_fault(samples);
    data.try_reserve_exact(samples)
}

pub(crate) fn stage_histories<const N: usize>(
    rate: f32,
    maximum: usize,
    inline: usize,
    old_history_bytes: usize,
    budget: PreparationBudget,
) -> Result<[History; N], PreparationError> {
    PreparationRequirements::checked::<N>(rate, maximum, inline, old_history_bytes)?
        .admit(budget)?;
    let mut staged = std::array::from_fn(|_| History::default());
    for (index, history) in staged.iter_mut().enumerate() {
        *history = History::try_new(maximum, index)?;
    }
    // Charge actual capacities, not just the requested lengths, before publish.
    let heap = staged
        .iter()
        .try_fold(0_usize, |n, h| n.checked_add(h.bytes()))
        .ok_or(PreparationError::CapacityOverflow {
            maximum_delay_samples: maximum,
            history_count: N,
        })?;
    PreparationRequirements::with_heap::<N>(rate, maximum, inline, old_history_bytes, heap)?
        .admit(budget)?;
    Ok(staged)
}

#[cfg(test)]
impl Drop for History {
    fn drop(&mut self) {
        allocation_test::retired(self.bytes());
    }
}

/// Test-only, thread-local reservation fault and lifetime accounting. It is
/// absent from production; none of these hooks run in audio callbacks.
#[cfg(test)]
pub(crate) mod allocation_test {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    thread_local! {
        static FAIL: Cell<Option<usize>> = const { Cell::new(None) };
        static ATTEMPTS: Cell<usize> = const { Cell::new(0) };
        static LIVE: Cell<usize> = const { Cell::new(0) };
        static CREATED: Cell<usize> = const { Cell::new(0) };
        static RETIRED: Cell<usize> = const { Cell::new(0) };
        static FAIL_ALLOCATION: Cell<Option<usize>> = const { Cell::new(None) };
    }
    // Only a selected, valid, fallible History reservation is allowed to arm
    // this allocator. Return null for that exact allocation once, then disarm
    // BEFORE std constructs its real allocation TryReserveError. Never fail
    // an infallible vec!/test-harness allocation and never exhaust OS memory.
    struct FaultAllocator;
    fn refuse(layout: Layout) -> bool {
        FAIL_ALLOCATION
            .try_with(|slot| {
                if slot.get() == Some(layout.size()) && layout.align() == align_of::<f32>() {
                    slot.set(None);
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }
    unsafe impl GlobalAlloc for FaultAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if refuse(layout) {
                std::ptr::null_mut()
            } else {
                unsafe { System.alloc(layout) }
            }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            if refuse(layout) {
                std::ptr::null_mut()
            } else {
                unsafe { System.alloc_zeroed(layout) }
            }
        }
        unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            let new_layout = unsafe { Layout::from_size_align_unchecked(size, layout.align()) };
            if refuse(new_layout) {
                std::ptr::null_mut()
            } else {
                unsafe { System.realloc(p, layout, size) }
            }
        }
        unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
            unsafe { System.dealloc(p, layout) }
        }
    }
    #[global_allocator]
    static ALLOCATOR: FaultAllocator = FaultAllocator;

    pub struct Fault;
    impl Fault {
        pub fn at(index: usize) -> Self {
            FAIL.with(|n| n.set(Some(index)));
            ATTEMPTS.with(|n| n.set(0));
            Self
        }
    }
    impl Drop for Fault {
        fn drop(&mut self) {
            FAIL.with(|n| n.set(None));
        }
    }
    pub fn snapshot() -> (usize, usize, usize) {
        (
            LIVE.with(Cell::get),
            CREATED.with(Cell::get),
            RETIRED.with(Cell::get),
        )
    }
    pub(super) struct ReservationFault;
    impl Drop for ReservationFault {
        fn drop(&mut self) {
            FAIL_ALLOCATION.with(|n| n.set(None));
        }
    }
    pub(super) fn reservation_fault(samples: usize) -> ReservationFault {
        let index = ATTEMPTS.with(|n| {
            let index = n.get();
            n.set(index + 1);
            index
        });
        if FAIL.with(|n| n.get() == Some(index)) {
            FAIL_ALLOCATION.with(|n| n.set(Some(samples * size_of::<f32>())));
        }
        ReservationFault
    }
    pub(super) fn created(bytes: usize) {
        LIVE.with(|n| n.set(n.get() + bytes));
        CREATED.with(|n| n.set(n.get() + 1));
    }
    pub(super) fn retired(bytes: usize) {
        if bytes != 0 {
            LIVE.with(|n| n.set(n.get() - bytes));
            RETIRED.with(|n| n.set(n.get() + 1));
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Tap {
    from: usize,
    to: usize,
    wanted: usize,
    remaining: u32,
    frames: u32,
}
impl Default for Tap {
    fn default() -> Self {
        Self {
            from: 0,
            to: 0,
            wanted: 0,
            remaining: 0,
            frames: 1,
        }
    }
}
impl Tap {
    pub fn set(&mut self, delay: usize, frames: u32, fresh: bool) {
        self.wanted = delay;
        self.frames = frames.max(1);
        if fresh {
            self.from = delay;
            self.to = delay;
            self.remaining = 0;
        }
    }
    pub fn read(&mut self, history: &History, current: f32) -> f32 {
        // Live changes retain the audible source until the new tap exists.
        if self.remaining == 0 && self.from != self.wanted && history.valid() >= self.wanted {
            self.to = self.wanted;
            self.remaining = self.frames;
        }
        let old = history.read(self.from, current);
        if self.remaining == 0 {
            return old;
        }
        let weight = 1.0 - self.remaining as f32 / self.frames as f32;
        let value = old + (history.read(self.to, current) - old) * weight;
        self.remaining -= 1;
        if self.remaining == 0 {
            self.from = self.to;
        }
        value
    }
    pub fn longest(&self) -> usize {
        self.from.max(self.to).max(self.wanted)
    }
    pub fn transition(&self, valid: usize) -> usize {
        if self.from == self.wanted && self.remaining == 0 {
            0
        } else {
            self.wanted.saturating_sub(valid) + self.remaining as usize + 2 * self.frames as usize
        }
    }
}
pub(crate) struct Ramps<const N: usize> {
    values: [LinearRamp; N],
    pub fresh: bool,
    frames: u32,
}
impl<const N: usize> Default for Ramps<N> {
    fn default() -> Self {
        Self {
            values: [LinearRamp::new(0.0); N],
            fresh: true,
            frames: 1,
        }
    }
}
impl<const N: usize> Ramps<N> {
    pub fn prepare(&mut self, rate: f32) {
        self.frames = (rate * 0.01).round().max(1.0) as u32;
        self.fresh = true;
    }
    pub fn reset(&mut self) {
        self.fresh = true;
    }
    pub fn restart_group(&mut self, range: std::ops::Range<usize>) {
        for ramp in &mut self.values[range] {
            ramp.snap(ramp.value());
        }
    }
    pub fn set(&mut self, values: [f32; N]) {
        for (r, v) in self.values.iter_mut().zip(values) {
            r.set_target(v, if self.fresh { 0 } else { self.frames });
        }
    }
    pub fn tick(&mut self) -> [f32; N] {
        self.fresh = false;
        std::array::from_fn(|i| self.values[i].tick())
    }
    pub fn max(&self, i: usize) -> f32 {
        self.values[i].value().max(self.values[i].target())
    }
    pub fn abs_max(&self, i: usize) -> f32 {
        self.values[i]
            .value()
            .abs()
            .max(self.values[i].target().abs())
    }
}
