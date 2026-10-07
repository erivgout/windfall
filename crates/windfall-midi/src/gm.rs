//! General MIDI: the names of the 128 programs, and a drum kit made of
//! Windfall's factory sounds.

use windfall_project::SamplePath;

/// The General MIDI name of a program number, 0 to 127, which most files
/// use to say what a channel should sound like. A number past 127 gives the
/// name of the last program.
pub fn program_name(program: u8) -> &'static str {
    PROGRAMS[usize::from(program.min(127))]
}

const PROGRAMS: [&str; 128] = [
    "Acoustic Grand Piano",
    "Bright Acoustic Piano",
    "Electric Grand Piano",
    "Honky-tonk Piano",
    "Electric Piano 1",
    "Electric Piano 2",
    "Harpsichord",
    "Clavinet",
    "Celesta",
    "Glockenspiel",
    "Music Box",
    "Vibraphone",
    "Marimba",
    "Xylophone",
    "Tubular Bells",
    "Dulcimer",
    "Drawbar Organ",
    "Percussive Organ",
    "Rock Organ",
    "Church Organ",
    "Reed Organ",
    "Accordion",
    "Harmonica",
    "Tango Accordion",
    "Acoustic Guitar (nylon)",
    "Acoustic Guitar (steel)",
    "Electric Guitar (jazz)",
    "Electric Guitar (clean)",
    "Electric Guitar (muted)",
    "Overdriven Guitar",
    "Distortion Guitar",
    "Guitar Harmonics",
    "Acoustic Bass",
    "Electric Bass (finger)",
    "Electric Bass (pick)",
    "Fretless Bass",
    "Slap Bass 1",
    "Slap Bass 2",
    "Synth Bass 1",
    "Synth Bass 2",
    "Violin",
    "Viola",
    "Cello",
    "Contrabass",
    "Tremolo Strings",
    "Pizzicato Strings",
    "Orchestral Harp",
    "Timpani",
    "String Ensemble 1",
    "String Ensemble 2",
    "Synth Strings 1",
    "Synth Strings 2",
    "Choir Aahs",
    "Voice Oohs",
    "Synth Voice",
    "Orchestra Hit",
    "Trumpet",
    "Trombone",
    "Tuba",
    "Muted Trumpet",
    "French Horn",
    "Brass Section",
    "Synth Brass 1",
    "Synth Brass 2",
    "Soprano Sax",
    "Alto Sax",
    "Tenor Sax",
    "Baritone Sax",
    "Oboe",
    "English Horn",
    "Bassoon",
    "Clarinet",
    "Piccolo",
    "Flute",
    "Recorder",
    "Pan Flute",
    "Blown Bottle",
    "Shakuhachi",
    "Whistle",
    "Ocarina",
    "Lead 1 (square)",
    "Lead 2 (sawtooth)",
    "Lead 3 (calliope)",
    "Lead 4 (chiff)",
    "Lead 5 (charang)",
    "Lead 6 (voice)",
    "Lead 7 (fifths)",
    "Lead 8 (bass + lead)",
    "Pad 1 (new age)",
    "Pad 2 (warm)",
    "Pad 3 (polysynth)",
    "Pad 4 (choir)",
    "Pad 5 (bowed)",
    "Pad 6 (metallic)",
    "Pad 7 (halo)",
    "Pad 8 (sweep)",
    "FX 1 (rain)",
    "FX 2 (soundtrack)",
    "FX 3 (crystal)",
    "FX 4 (atmosphere)",
    "FX 5 (brightness)",
    "FX 6 (goblins)",
    "FX 7 (echoes)",
    "FX 8 (sci-fi)",
    "Sitar",
    "Banjo",
    "Shamisen",
    "Koto",
    "Kalimba",
    "Bagpipe",
    "Fiddle",
    "Shanai",
    "Tinkle Bell",
    "Agogo",
    "Steel Drums",
    "Woodblock",
    "Taiko Drum",
    "Melodic Tom",
    "Synth Drum",
    "Reverse Cymbal",
    "Guitar Fret Noise",
    "Breath Noise",
    "Seashore",
    "Bird Tweet",
    "Telephone Ring",
    "Helicopter",
    "Applause",
    "Gunshot",
];

/// Which sample plays for which key of a MIDI drum channel.
///
/// On the drum channel a key is not a pitch but a drum: 36 is a kick, 38 a
/// snare, 42 a closed hi-hat. Given a kit, [`import`](crate::import) makes
/// a sampler channel for each of the kit's samples that the file plays, and
/// moves the notes of its keys there.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DrumKit {
    /// One pad for each key that has a sample. Keys without a pad stay on
    /// a synth channel. Of two pads for one key the first counts, and pads
    /// that share a sample share a channel.
    pub pads: Vec<DrumPad>,
}

/// One MIDI drum key mapped to a project sample.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrumPad {
    /// The MIDI key that plays the pad, 0 to 127.
    pub key: u8,
    /// What the pad's channel is called.
    pub name: String,
    pub sample: SamplePath,
}

impl DrumKit {
    /// The General MIDI drum map played with Windfall's factory sounds,
    /// as far as the factory kit has a sound for a drum. Latin percussion
    /// it has no sound for is left without a pad.
    pub fn factory() -> Self {
        let pads = FACTORY_PADS.iter().flat_map(|(keys, file)| {
            let name = file.rsplit('/').next().unwrap_or(file);
            keys.iter().map(move |&key| DrumPad {
                key,
                name: name.to_owned(),
                sample: SamplePath::Factory(format!("Drums/{file}.wav")),
            })
        });
        Self {
            pads: pads.collect(),
        }
    }

    /// The pad a key plays.
    pub fn pad(&self, key: u8) -> Option<&DrumPad> {
        self.pads.iter().find(|pad| pad.key == key)
    }
}

/// The General MIDI percussion keys each factory sound stands in for, with
/// the sound's path inside the factory `Drums` folder, less its extension.
const FACTORY_PADS: [(&[u8], &str); 20] = [
    (&[35], "Kicks/Kick Deep"),
    (&[36], "Kicks/Kick Punch"),
    (&[37], "Percussion/Rim Click"),
    (&[38], "Snares/Snare Tight"),
    (&[39], "Claps/Clap Tight"),
    (&[40], "Snares/Snare Fat"),
    (&[41, 43, 45], "Toms/Tom Low"),
    (&[47, 48], "Toms/Tom Mid"),
    (&[50], "Toms/Tom High"),
    (&[42], "Hats/Hat Closed 1"),
    (&[44], "Hats/Hat Pedal"),
    (&[46], "Hats/Hat Open 1"),
    (&[49, 52, 55, 57], "Cymbals/Crash"),
    (&[51, 53, 59], "Cymbals/Ride"),
    (&[54], "Percussion/Tambourine"),
    (&[56], "Percussion/Cowbell"),
    (&[60, 61, 62, 63, 64], "Percussion/Conga"),
    (&[69, 70, 82], "Percussion/Shaker"),
    (&[75], "Percussion/Clave"),
    (&[76, 77], "Percussion/Wood Block"),
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;

    use super::*;

    #[test]
    fn every_program_has_a_name() {
        assert_eq!(program_name(0), "Acoustic Grand Piano");
        assert_eq!(program_name(33), "Electric Bass (finger)");
        assert_eq!(program_name(127), "Gunshot");
        assert_eq!(program_name(200), "Gunshot");
        let names: HashSet<&str> = PROGRAMS.iter().copied().collect();
        assert_eq!(names.len(), 128);
    }

    #[test]
    fn the_factory_kit_points_at_sounds_that_ship() {
        let factory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/factory");
        let kit = DrumKit::factory();
        let mut keys = HashSet::new();
        for pad in &kit.pads {
            assert!(keys.insert(pad.key), "key {} has two pads", pad.key);
            assert_eq!(pad.sample.problem(), None);
            let SamplePath::Factory(path) = &pad.sample else {
                panic!("{:?} is not a factory sound", pad.sample);
            };
            assert!(factory.join(path).is_file(), "{path} is not in the factory");
            assert!(path.ends_with(&format!("{}.wav", pad.name)));
        }
        assert_eq!(kit.pad(36).map(|pad| pad.name.as_str()), Some("Kick Punch"));
        assert_eq!(
            kit.pad(38).map(|pad| pad.name.as_str()),
            Some("Snare Tight")
        );
        assert_eq!(
            kit.pad(42).map(|pad| pad.name.as_str()),
            Some("Hat Closed 1")
        );
        assert_eq!(kit.pad(81), None);
    }
}
