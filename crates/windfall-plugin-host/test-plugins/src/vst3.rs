//! Independent VST3 factory and component fixtures for the scanner.
#![allow(non_snake_case)]
use std::ffi::c_void;
use std::ptr;
use vst3::Steinberg::Vst::*;
use vst3::{Class, ComPtr, ComWrapper, Interface, Steinberg::*};

fn chars<const N: usize>(text: &str) -> [char8; N] {
    let mut result = [0; N];
    for (slot, byte) in result.iter_mut().take(N - 1).zip(text.bytes()) {
        *slot = byte as char8;
    }
    result
}
fn cid(index: i32) -> TUID {
    let mut id = [0; 16];
    id[15] = (index + 1) as char8;
    id
}
struct Factory;
impl Class for Factory {
    type Interfaces = (IPluginFactory2,);
}
impl IPluginFactoryTrait for Factory {
    unsafe fn getFactoryInfo(&self, info: *mut PFactoryInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: caller supplies a writable SDK output struct.
        unsafe {
            info.write(PFactoryInfo {
                vendor: chars("Windfall Tests"),
                url: [0; 256],
                email: [0; 128],
                flags: 0,
            });
        }
        kResultOk
    }
    unsafe fn countClasses(&self) -> i32 {
        2
    }
    unsafe fn getClassInfo(&self, index: i32, info: *mut PClassInfo) -> tresult {
        if info.is_null() || !(0..2).contains(&index) {
            return kInvalidArgument;
        }
        // SAFETY: caller supplies a writable SDK output struct.
        unsafe {
            info.write(PClassInfo {
                cid: cid(index),
                cardinality: i32::MAX,
                category: chars("Audio Module Class"),
                name: chars(if index == 0 {
                    "VST3 Test Effect"
                } else {
                    "VST3 Absurd Ports"
                }),
            });
        }
        kResultOk
    }
    unsafe fn createInstance(
        &self,
        class: FIDString,
        iid: FIDString,
        obj: *mut *mut c_void,
    ) -> tresult {
        if class.is_null() || iid.is_null() || obj.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: SDK passes sixteen-byte ids and a writable output pointer.
        unsafe {
            obj.write(ptr::null_mut());
            let class = ptr::read_unaligned(class.cast::<TUID>());
            let iid = ptr::read_unaligned(iid.cast::<[u8; 16]>());
            if iid != IComponent::IID || (class != cid(0) && class != cid(1)) {
                return kNoInterface;
            }
            let component = ComWrapper::new(Component {
                absurd: class == cid(1),
            })
            .to_com_ptr::<IComponent>()
            .unwrap();
            obj.write(component.into_raw().cast());
        }
        kResultOk
    }
}
impl IPluginFactory2Trait for Factory {
    unsafe fn getClassInfo2(&self, index: i32, info: *mut PClassInfo2) -> tresult {
        if info.is_null() || !(0..2).contains(&index) {
            return kInvalidArgument;
        }
        // SAFETY: caller supplies a writable SDK output struct.
        unsafe {
            info.write(PClassInfo2 {
                cid: cid(index),
                cardinality: i32::MAX,
                category: chars("Audio Module Class"),
                name: chars(if index == 0 {
                    "VST3 Test Effect"
                } else {
                    "VST3 Absurd Ports"
                }),
                classFlags: 0,
                subCategories: chars("Fx|Tools"),
                vendor: chars("Windfall Tests"),
                version: chars("1.0"),
                sdkVersion: chars("3.8"),
            });
        }
        kResultOk
    }
}
struct Component {
    absurd: bool,
}
impl Class for Component {
    type Interfaces = (IComponent,);
}
impl IPluginBaseTrait for Component {
    unsafe fn initialize(&self, context: *mut FUnknown) -> tresult {
        // SAFETY: borrowed SDK host context, used only inside this call.
        let Some(context) = (unsafe { vst3::ComRef::from_raw(context) }) else {
            return kInvalidArgument;
        };
        let Some(host) = context.cast::<IHostApplication>() else {
            return kNoInterface;
        };
        let mut name = [0; 128];
        // SAFETY: live host interface and sized name output.
        if unsafe { host.getName(&mut name) } != kResultOk || name[0] != u16::from(b'W') {
            return kInvalidArgument;
        }
        kResultOk
    }
    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}
impl IComponentTrait for Component {
    unsafe fn getControllerClassId(&self, _id: *mut TUID) -> tresult {
        kNotImplemented
    }
    unsafe fn setIoMode(&self, _mode: IoMode) -> tresult {
        kResultOk
    }
    unsafe fn getBusCount(&self, media: MediaType, _dir: BusDirection) -> i32 {
        if self.absurd {
            1000
        } else if media == 0 {
            1
        } else {
            0
        }
    }
    unsafe fn getBusInfo(
        &self,
        media: MediaType,
        dir: BusDirection,
        index: i32,
        bus: *mut BusInfo,
    ) -> tresult {
        if media != 0 || index != 0 || bus.is_null() {
            return kInvalidArgument;
        }
        let mut name = [0; 128];
        for (slot, unit) in name.iter_mut().zip("Stereo".encode_utf16()) {
            *slot = unit;
        }
        // SAFETY: caller supplies a writable SDK bus structure.
        unsafe {
            bus.write(BusInfo {
                mediaType: media,
                direction: dir,
                channelCount: 2,
                name,
                busType: 0,
                flags: 1,
            });
        }
        kResultOk
    }
    unsafe fn getRoutingInfo(
        &self,
        _input: *mut RoutingInfo,
        _output: *mut RoutingInfo,
    ) -> tresult {
        kNotImplemented
    }
    unsafe fn activateBus(
        &self,
        _media: MediaType,
        _dir: BusDirection,
        _index: i32,
        _state: TBool,
    ) -> tresult {
        kResultOk
    }
    unsafe fn setActive(&self, _state: TBool) -> tresult {
        kResultOk
    }
    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }
    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }
}
/// SDK factory export, independently implemented from the host.
#[unsafe(no_mangle)]
pub extern "system" fn GetPluginFactory() -> *mut IPluginFactory {
    let factory: ComPtr<IPluginFactory> = ComWrapper::new(Factory).to_com_ptr().unwrap();
    factory.into_raw()
}
#[unsafe(no_mangle)]
pub extern "system" fn InitDll() -> bool {
    true
}
#[unsafe(no_mangle)]
pub extern "system" fn ExitDll() -> bool {
    true
}
#[unsafe(no_mangle)]
pub extern "system" fn ModuleEntry(_module: *mut c_void) -> bool {
    true
}
#[unsafe(no_mangle)]
pub extern "system" fn ModuleExit() -> bool {
    true
}
