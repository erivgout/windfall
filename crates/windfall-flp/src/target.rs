//! How an FL Studio file names a control: a fader, a knob of a plugin, the
//! tempo.
//!
//! Three kinds of event name controls with the same 32-bit number: the
//! starting values of controls (events 216 and 225), changes recorded into
//! a pattern (event 223), and links from automation clips (event 227). The
//! high 16 bits say whose control it is and the low 16 bits which one.
//!
//! | High bits 15 to 13 | Owner | Rest of the high half |
//! |---|---|---|
//! | 0 | a channel | the channel number |
//! | 1 | a mixer insert | the insert in bits 12 to 6, an effect slot in bits 5 to 0 |
//! | 2 | the project | nothing |
//!
//! In the low half, bit 15 means a parameter of the plugin on the channel
//! or in the slot, numbered by the other 15 bits. Otherwise the low byte
//! picks one of the owner's own controls.
//!
//! Sources: DawVert `objects/file_proj/_flp/auto.py` (the whole scheme),
//! PyFLP `pyflp/mixer.py` (the insert's controls and where insert and slot
//! sit) and LMMS `FlpImport.cpp` (the channel's controls, and 0x1FC0 for an
//! insert's fader). FLParser reads the insert's controls the same way.

/// A control of an FL Studio project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlTarget {
    /// One of a channel's own controls.
    Channel { channel: u16, param: ChannelParam },
    /// A parameter of the plugin on a channel, by its number.
    ChannelPlugin { channel: u16, param: u16 },
    /// One of a mixer insert's own controls.
    Insert { insert: u16, param: InsertParam },
    /// The level at which one insert feeds another.
    Route { insert: u16, target: u16 },
    /// One of an effect slot's own controls.
    Slot {
        insert: u16,
        slot: u16,
        param: SlotParam,
    },
    /// A parameter of the plugin in an effect slot, by its number.
    SlotPlugin { insert: u16, slot: u16, param: u16 },
    /// A control of the whole project.
    Main(MainParam),
    /// A number that fits none of the above.
    Unknown(u32),
}

/// A channel's own controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelParam {
    Volume,
    Pan,
    /// The first of the two knobs a channel's filter usually follows.
    ModX,
    ModY,
    Pitch,
    /// A control that is not one of the five above: the page of controls
    /// it is on and its number there, as DawVert lists them.
    Other {
        page: u8,
        index: u8,
    },
}

/// A mixer insert's own controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InsertParam {
    Volume,
    Pan,
    StereoSeparation,
    /// Gain of a band of the insert's equaliser: 0 low, 1 mid, 2 high.
    EqGain(u8),
    EqFrequency(u8),
    EqWidth(u8),
    Other(u8),
}

/// An effect slot's own controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotParam {
    Enabled,
    Mix,
    Other(u8),
}

/// Controls of the whole project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MainParam {
    Volume,
    Swing,
    Pitch,
    Tempo,
    Other(u16),
}

const PLUGIN_BIT: u32 = 0x8000;
/// The second byte of a mixer control that is not a plugin parameter, as
/// every file this crate was run over writes it.
const MIXER_PAGE: u32 = 0x1F00;
/// The first insert control that is a route. The target insert is added.
const FIRST_ROUTE: u32 = 64;
const INSERT_VOLUME: u32 = 192;

impl ControlTarget {
    /// Reads the 32-bit number of an event.
    pub fn decode(location: u32) -> Self {
        let control = location & 0xFFFF;
        let owner = location >> 16;
        let plugin = control & PLUGIN_BIT != 0;
        let param = (control & 0x7FFF) as u16;
        match owner >> 13 {
            0 => {
                let channel = (owner & 0x0FFF) as u16;
                if plugin {
                    ControlTarget::ChannelPlugin { channel, param }
                } else if control >> 13 != 0 {
                    ControlTarget::Unknown(location)
                } else {
                    let (page, index) = ((control >> 8) as u8, (control & 0xFF) as u8);
                    let param = match (page, index) {
                        (0, 0) => ChannelParam::Volume,
                        (0, 1) => ChannelParam::Pan,
                        (0, 2) => ChannelParam::ModX,
                        (0, 3) => ChannelParam::ModY,
                        (0, 4) => ChannelParam::Pitch,
                        _ => ChannelParam::Other { page, index },
                    };
                    ControlTarget::Channel { channel, param }
                }
            }
            1 => {
                let insert = ((owner >> 6) & 0x7F) as u16;
                let slot = (owner & 0x3F) as u16;
                if plugin {
                    return ControlTarget::SlotPlugin {
                        insert,
                        slot,
                        param,
                    };
                }
                let id = control & 0xFF;
                match id {
                    0 => ControlTarget::Slot {
                        insert,
                        slot,
                        param: SlotParam::Enabled,
                    },
                    1 => ControlTarget::Slot {
                        insert,
                        slot,
                        param: SlotParam::Mix,
                    },
                    2..FIRST_ROUTE => ControlTarget::Slot {
                        insert,
                        slot,
                        param: SlotParam::Other(id as u8),
                    },
                    FIRST_ROUTE..INSERT_VOLUME => ControlTarget::Route {
                        insert,
                        target: (id - FIRST_ROUTE) as u16,
                    },
                    _ => ControlTarget::Insert {
                        insert,
                        param: match id {
                            192 => InsertParam::Volume,
                            193 => InsertParam::Pan,
                            194 => InsertParam::StereoSeparation,
                            208..=210 => InsertParam::EqGain((id - 208) as u8),
                            216..=218 => InsertParam::EqFrequency((id - 216) as u8),
                            224..=226 => InsertParam::EqWidth((id - 224) as u8),
                            other => InsertParam::Other(other as u8),
                        },
                    },
                }
            }
            2 => ControlTarget::Main(match control & 0x0FFF {
                0 => MainParam::Volume,
                1 => MainParam::Swing,
                2 => MainParam::Pitch,
                5 => MainParam::Tempo,
                other => MainParam::Other(other as u16),
            }),
            _ => ControlTarget::Unknown(location),
        }
    }

    /// The 32-bit number an event holds for this control: the inverse of
    /// [`decode`](Self::decode) for every control it can tell apart. Numbers
    /// that do not fit their field lose their high bits.
    pub fn encode(&self) -> u32 {
        let slot_owner = |insert: u16, slot: u16| {
            (1 << 13) | ((u32::from(insert) & 0x7F) << 6) | (u32::from(slot) & 0x3F)
        };
        let (owner, control) = match *self {
            ControlTarget::Channel { channel, param } => {
                let control = match param {
                    ChannelParam::Volume => 0,
                    ChannelParam::Pan => 1,
                    ChannelParam::ModX => 2,
                    ChannelParam::ModY => 3,
                    ChannelParam::Pitch => 4,
                    ChannelParam::Other { page, index } => {
                        ((u32::from(page) & 0x1F) << 8) | u32::from(index)
                    }
                };
                (u32::from(channel) & 0x0FFF, control)
            }
            ControlTarget::ChannelPlugin { channel, param } => (
                u32::from(channel) & 0x0FFF,
                PLUGIN_BIT | (u32::from(param) & 0x7FFF),
            ),
            ControlTarget::Insert { insert, param } => {
                let id = match param {
                    InsertParam::Volume => 192,
                    InsertParam::Pan => 193,
                    InsertParam::StereoSeparation => 194,
                    InsertParam::EqGain(band) => 208 + u32::from(band.min(2)),
                    InsertParam::EqFrequency(band) => 216 + u32::from(band.min(2)),
                    InsertParam::EqWidth(band) => 224 + u32::from(band.min(2)),
                    InsertParam::Other(id) => u32::from(id),
                };
                (slot_owner(insert, 0), MIXER_PAGE | id)
            }
            ControlTarget::Route { insert, target } => (
                slot_owner(insert, 0),
                MIXER_PAGE | (FIRST_ROUTE + (u32::from(target) & 0x7F)),
            ),
            ControlTarget::Slot {
                insert,
                slot,
                param,
            } => {
                let id = match param {
                    SlotParam::Enabled => 0,
                    SlotParam::Mix => 1,
                    SlotParam::Other(id) => u32::from(id),
                };
                (slot_owner(insert, slot), MIXER_PAGE | id)
            }
            ControlTarget::SlotPlugin {
                insert,
                slot,
                param,
            } => (
                slot_owner(insert, slot),
                PLUGIN_BIT | (u32::from(param) & 0x7FFF),
            ),
            ControlTarget::Main(param) => {
                let control = match param {
                    MainParam::Volume => 0,
                    MainParam::Swing => 1,
                    MainParam::Pitch => 2,
                    MainParam::Tempo => 5,
                    MainParam::Other(other) => u32::from(other) & 0x0FFF,
                };
                (2 << 13, control)
            }
            ControlTarget::Unknown(location) => return location,
        };
        (owner << 16) | control
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_seen_in_files_decode_to_what_they_set() {
        // The first records of event 225 in a project saved by FL Studio
        // 20.8.4: main volume, then the first slot of the master.
        assert_eq!(
            ControlTarget::decode(0x4000_0000),
            ControlTarget::Main(MainParam::Volume)
        );
        assert_eq!(
            ControlTarget::decode(0x2000_1F00),
            ControlTarget::Slot {
                insert: 0,
                slot: 0,
                param: SlotParam::Enabled
            }
        );
        assert_eq!(
            ControlTarget::decode(0x2001_1F01),
            ControlTarget::Slot {
                insert: 0,
                slot: 1,
                param: SlotParam::Mix
            }
        );
        // LMMS's importer knows an insert's fader as 0x1FC0.
        assert_eq!(
            ControlTarget::decode(0x2140_1FC0),
            ControlTarget::Insert {
                insert: 5,
                param: InsertParam::Volume
            }
        );
        assert_eq!(
            ControlTarget::decode(0x0005_0001),
            ControlTarget::Channel {
                channel: 5,
                param: ChannelParam::Pan
            }
        );
        assert_eq!(
            ControlTarget::decode(0x0003_8007),
            ControlTarget::ChannelPlugin {
                channel: 3,
                param: 7
            }
        );
        assert_eq!(
            ControlTarget::decode(0x4000_0005),
            ControlTarget::Main(MainParam::Tempo)
        );
    }

    #[test]
    fn routes_sit_between_the_slot_controls_and_the_fader() {
        assert_eq!(
            ControlTarget::decode(0x2040_1F40),
            ControlTarget::Route {
                insert: 1,
                target: 0
            }
        );
        assert_eq!(
            ControlTarget::decode(0x2040_1FBF),
            ControlTarget::Route {
                insert: 1,
                target: 127
            }
        );
    }

    #[test]
    fn what_decodes_encodes_back() {
        let targets = [
            ControlTarget::Channel {
                channel: 17,
                param: ChannelParam::Volume,
            },
            ControlTarget::Channel {
                channel: 0,
                param: ChannelParam::Other { page: 2, index: 4 },
            },
            ControlTarget::ChannelPlugin {
                channel: 4095,
                param: 0x7FFF,
            },
            ControlTarget::Insert {
                insert: 126,
                param: InsertParam::EqFrequency(2),
            },
            ControlTarget::Insert {
                insert: 3,
                param: InsertParam::StereoSeparation,
            },
            ControlTarget::Route {
                insert: 4,
                target: 99,
            },
            ControlTarget::Slot {
                insert: 9,
                slot: 9,
                param: SlotParam::Mix,
            },
            ControlTarget::SlotPlugin {
                insert: 1,
                slot: 2,
                param: 300,
            },
            ControlTarget::Main(MainParam::Pitch),
            ControlTarget::Main(MainParam::Other(9)),
            ControlTarget::Unknown(0xE000_1234),
        ];
        for target in targets {
            assert_eq!(ControlTarget::decode(target.encode()), target, "{target:?}");
        }
    }
}
