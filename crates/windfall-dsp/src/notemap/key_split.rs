use super::{MappedNote, NoteTransform};
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Low accepts lowMin <= key < boundary; high accepts boundary <= key <= highMax.
/// Outside those ranges is dropped. Each range maps to one destination key/channel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct KeySplitParams {
    pub low_min: u8,
    pub boundary: u8,
    pub high_max: u8,
    pub low_key: u8,
    pub high_key: u8,
    pub low_channel: u8,
    pub high_channel: u8,
}
impl Default for KeySplitParams {
    fn default() -> Self {
        Self {
            low_min: 0,
            boundary: 60,
            high_max: 127,
            low_key: 48,
            high_key: 72,
            low_channel: 0,
            high_channel: 1,
        }
    }
}
param_set!(KeySplitParams, "Key split", {
    int [low_min] "lowMin" "Low minimum" { None, 0, 127, 0 }
    int [boundary] "boundary" "Boundary key" { None, 0, 127, 60 }
    int [high_max] "highMax" "High maximum" { None, 0, 127, 127 }
    int [low_key] "lowKey" "Low destination key" { None, 0, 127, 48 }
    int [high_key] "highKey" "High destination key" { None, 0, 127, 72 }
    int [low_channel] "lowChannel" "Low destination channel" { None, 0, 15, 0 }
    int [high_channel] "highChannel" "High destination channel" { None, 0, 15, 1 }
});

#[derive(Debug, Clone, Copy, Default)]
pub struct KeySplit {
    params: KeySplitParams,
}
impl NoteTransform for KeySplit {
    type Params = KeySplitParams;
    fn set_params(&mut self, params: &Self::Params) {
        self.params = params.sanitized();
    }
    fn map(&self, note: MappedNote) -> Option<MappedNote> {
        let mut note = note.sanitized();
        let p = self.params;
        if note.key < p.boundary && note.key >= p.low_min {
            note.key = p.low_key;
            note.channel = p.low_channel;
        } else if note.key >= p.boundary && note.key <= p.high_max {
            note.key = p.high_key;
            note.channel = p.high_channel;
        } else {
            return None;
        }
        Some(note)
    }
}
