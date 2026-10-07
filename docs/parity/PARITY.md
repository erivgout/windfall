# Windfall parity matrix

> **Generated file.** Do not edit it. Edit `docs/parity/parity.json` and run `node scripts/parity.mjs`.
>
> The "FL feature" column lists FL Studio feature and plugin names as plain references only ("FL equivalent").
> Notes may mention FL names for the same purpose. They are Image-Line's names and are never used as Windfall
> names. `TBD` in the Windfall column means the Windfall name has not been chosen yet.

As of 2026-10-07. Sources:

- <https://www.image-line.com/fl-studio/compare-editions>
- <https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/>
- <https://forum.image-line.com/viewtopic.php?p=2065297>

## Summary

**47 of 342 rows accounted for (13.7%).** A row is accounted for when it is done or won't do.

| Status | Rows |
| --- | --- |
| Todo | 273 |
| In progress | 22 |
| Done | 45 |
| Won't do | 2 |

| Area | Rows | Todo | In progress | Done | Won't do |
| --- | --- | --- | --- | --- | --- |
| Core features | 23 | 17 | 4 | 1 | 1 |
| Main windows | 113 | 70 | 12 | 30 | 1 |
| Instruments | 41 | 39 | 1 | 1 | 0 |
| Effects | 80 | 75 | 0 | 5 | 0 |
| Visual and video | 7 | 7 | 0 | 0 | 0 |
| Audio editors | 3 | 3 | 0 | 0 | 0 |
| File formats and plugin hosting | 36 | 32 | 0 | 4 | 0 |
| Workflow, MIDI and settings | 39 | 30 | 5 | 4 | 0 |

| Phase | Rows | Todo | In progress | Done | Won't do |
| --- | --- | --- | --- | --- | --- |
| 0. Spike | 1 | 0 | 1 | 0 | 0 |
| 1. Make a beat | 39 | 10 | 7 | 20 | 2 |
| 2. Write a song | 99 | 60 | 14 | 25 | 0 |
| 3. Record and edit audio | 34 | 34 | 0 | 0 | 0 |
| 4. Plugins and files | 13 | 13 | 0 | 0 | 0 |
| 5. The long tail | 129 | 129 | 0 | 0 | 0 |
| 6. Extras | 24 | 24 | 0 | 0 | 0 |
| 7. Release | 3 | 3 | 0 | 0 | 0 |

## Core features

| FL feature | Windfall | Phase | Status | Notes |
| --- | --- | --- | --- | --- |
| Lifetime Free Updates | Free updates | 7 | todo | Windfall is free under GPL-3.0; this row closes when auto-update ships. |
| Stem Separation | Stem separation | 5 | todo | unverified: lowest edition. The compare table ticks Producer, but the purchase box on the same page shows it locked below Signature. Candidate basis: Demucs (MIT). |
| Audio Recording | Audio recording | 3 | todo |  |
| Audio Clips | Audio clips | 2 | in-progress | Audio clips with gain, pan, fades, reverse and tape-style pitch. Time-stretch and recording are not in yet. |
| Loop Starter | Loop starter | 6 | todo | Needs Windfall's own CC0 loop content; FL's loops cannot be reused. |
| FL Studio Mobile Rack + FX | TBD | 5 | todo | A Windfall equivalent would be a rack of Windfall's own modules. FL 2026 added a SoundFont player and a note arpeggiator module. |
| Audio Logger | Audio logger | 3 | todo | unverified: edition availability (the compare page leaves every edition cell blank). New in FL Studio 2026. |
| Chord Generator | Chord progression generator | 6 | todo | The manual calls it the Chord Progression Tool. Not named in the plan's phase table, so placed in phase 6. |
| Gopher | Assistant | 6 | todo | The plan's phase-6 assistant. Control of the app and plugins is marked experimental in FL 2026. |
| Denoising | Denoising | 5 | todo | Candidate basis: RNNoise (BSD). In FL this lives in the audio editor's noise removal and vocal denoiser tools. |
| Sound Content | Factory sound library | 1 | wont-do | Reason: Image-Line owns its bundled samples, loops and presets, so they can never be shipped. Windfall ships its own content under CC0 or a similar license. |
| Piano Roll | Piano roll | 2 | in-progress | Umbrella row; sub-features are tracked as win-piano-* rows. Draw, paint, select and erase tools, clipboard, quantize, velocity and pan lane, ghost notes. The specialist tools are separate rows. |
| Mixer | Mixer | 2 | in-progress | Umbrella row; the basic mixer lands in phase 1 and routing, sends and effect slots in phase 2 (win-mixer-* rows). Tracks, routing, sends, effect slots, meters and delay compensation are in. Sidechain, track EQ and track presets are separate rows. |
| Full Song Arrangement | Playlist | 2 | done | Pattern, audio and automation clips on the playlist, with song mode. |
| Automation Clips | Automation clips | 2 | in-progress | Curves with bends and holds for volume, pan, sends, effect and instrument settings, effect mix and tempo. No LFO or step drawing modes yet. |
| Time signature changes | Time signature changes | 2 | todo |  |
| MIDI Support | MIDI input | 3 | todo |  |
| MIDI Out | MIDI output | 3 | todo |  |
| VST2, VST3, Audio Unit and CLAP support | Plugin hosting | 4 | todo | Umbrella row; each format is tracked as an fmt-host-* row. The plan covers CLAP, VST3 and AU, not VST2. |
| FL Studio Remote | Phone remote | 6 | todo | The plan's phase-6 phone remote. |
| Fruity Envelope Controller | TBD | 5 | todo |  |
| Fruity Keyboard Controller | TBD | 5 | todo |  |
| Fruity Voltage Controller | TBD | 5 | todo |  |

## Main windows

| FL feature | Windfall | Phase | Status | Notes |
| --- | --- | --- | --- | --- |
| Channel Rack | Channel rack | 1 | done |  |
| Step sequencer | Step sequencer | 1 | done |  |
| Graph editor | Step graph editor | 1 | todo |  |
| Channel mute, solo, pan and volume | Channel controls | 1 | done |  |
| Channel target mixer track selector | Channel-to-mixer routing | 1 | done | Windfall plans automatic routing: each new channel gets its own mixer track. New channels get their own mixer track; the routing badge on each row shows and changes it. |
| Patterns and pattern selector | Patterns | 1 | done |  |
| Swing (global and per channel) | Swing | 1 | in-progress | Global swing only; no per-channel swing mix yet. |
| Channel groups and filter | Channel groups | 1 | todo |  |
| Channel button menu (clone, replace, insert, delete, rename, color) | Channel menu | 1 | done |  |
| Fill each N steps and Advanced Fill tool | Step fill tools | 1 | in-progress | Fill every 2, 4 or 8 steps and shift left or right. No advanced fill tool yet. |
| Cut and Cut-by groups | Cut groups | 1 | done |  |
| Channel sampler: loop points and ping-pong loop | Sample looping | 1 | todo |  |
| Channel sampler: precomputed effects | Sample pre-processing | 1 | todo |  |
| Mini piano roll preview | Note preview in rack | 2 | todo |  |
| Send to Piano roll | Steps to notes | 2 | todo |  |
| Channel settings: envelopes, LFOs and filter | Channel envelopes and LFOs | 2 | in-progress | Volume envelope on the sampler only. |
| Channel settings: arpeggiator | Channel arpeggiator | 2 | todo |  |
| Channel settings: echo delay | Note echo | 2 | todo |  |
| Channel settings: polyphony and portamento | Channel polyphony and glide | 2 | todo |  |
| Channel settings: gate, shift and swing mix | Channel note timing | 2 | todo |  |
| Channel sampler: time-stretch and pitch modes | Sample time-stretch | 3 | todo | Candidate basis: Signalsmith Stretch (MIT). FL licenses a commercial stretch engine for this. |
| Layer channel (Fruity Layer) | Layer channel | 5 | todo | Listed in the manual's plugin index, not on the compare page. The manual points to the modular rack as the more flexible alternative. |
| Piano roll: Draw tool | Piano roll draw tool | 2 | done |  |
| Piano roll: Paint tool and drum sequencer mode | Piano roll paint tool | 2 | in-progress | Paint tool is in. No drum sequencer mode yet. |
| Piano roll: Delete tool | Piano roll delete tool | 2 | done |  |
| Piano roll: Mute tool | Piano roll mute tool | 2 | todo |  |
| Piano roll: Slice tool | Piano roll slice tool | 2 | todo |  |
| Piano roll: Select tool | Piano roll select tool | 2 | done |  |
| Piano roll: Zoom tool | Piano roll zoom tool | 2 | todo |  |
| Piano roll: Playback (scrub) tool | Piano roll scrub tool | 2 | todo |  |
| Piano roll: Chord stamp | Chord stamp | 2 | todo |  |
| Piano roll: preview keyboard and key labels | Preview keyboard | 2 | done |  |
| Piano roll: event editor lane and note properties | Note property lane | 2 | in-progress | Velocity and pan lanes. Other note properties are not in the model yet. |
| Piano roll: slide and portamento notes | Slide notes | 2 | todo |  |
| Piano roll: note colors (16 color groups) | Note colors | 2 | todo |  |
| Piano roll: ghost notes | Ghost notes | 2 | done |  |
| Piano roll: scale highlighting and snap to scale | Scale highlighting | 2 | todo |  |
| Piano roll: snap to grid | Piano roll snap | 2 | done |  |
| Piano roll: time markers and per-pattern time signatures | Pattern markers | 2 | todo |  |
| Piano roll: waveform helper view | Waveform helper | 2 | todo |  |
| Piano roll: Quantizer tool | Quantize | 2 | in-progress | Quantize starts and ends to the snap. No quantizer dialog with strength or groove templates yet. |
| Piano roll: Articulator tool and Quick legato | Articulate | 2 | todo |  |
| Piano roll: Chopper tool and Quick chop | Chop | 2 | todo |  |
| Piano roll: Glue | Glue | 2 | todo |  |
| Piano roll: Arpeggiator tool | Arpeggiate | 2 | todo |  |
| Piano roll: Strum tool | Strum | 2 | todo |  |
| Piano roll: Flam tool | Flam | 2 | todo |  |
| Piano roll: Claw machine tool | Rhythm reshaper | 2 | todo |  |
| Piano roll: Key limiter tool | Key limiter | 2 | todo |  |
| Piano roll: Flip tool | Flip | 2 | todo |  |
| Piano roll: Randomizer tool | Randomize | 2 | todo |  |
| Piano roll: Scale levels tool | Scale levels | 2 | todo |  |
| Piano roll: LFO tool | LFO tool | 2 | todo |  |
| Piano roll: Riff machine | Riff generator | 6 | todo | Not named in the plan's phase table, so placed in phase 6. |
| Piano roll scripting (Python) | Piano roll scripting | 6 | todo | Part of the plan's phase-6 scripting. |
| Piano roll: export as score sheet | Sheet music export | 6 | todo | Not named in the plan's phase table, so placed in phase 6. |
| Playlist: pattern clips | Pattern clips | 2 | done |  |
| Playlist: Draw tool | Playlist draw tool | 2 | done |  |
| Playlist: Paint tool | Playlist paint tool | 2 | done |  |
| Playlist: Delete tool | Playlist delete tool | 2 | done |  |
| Playlist: Mute tool | Playlist mute tool | 2 | done |  |
| Playlist: Slip edit tool | Slip edit | 2 | todo |  |
| Playlist: Slice tool | Playlist slice tool | 2 | todo |  |
| Playlist: Select tool | Playlist select tool | 2 | done |  |
| Playlist: Zoom tool | Playlist zoom tool | 2 | todo |  |
| Playlist: Playback tool | Playlist scrub tool | 2 | todo |  |
| Playlist tracks (name, color, mute, solo, resize) | Playlist tracks | 2 | in-progress | Name, mute and reordering. No color, solo or per-track resize yet. |
| Playlist: track grouping | Track groups | 2 | todo |  |
| Playlist: instrument tracks and audio tracks | Linked tracks | 2 | todo |  |
| Playlist: time markers | Time markers | 2 | todo |  |
| Playlist: arrangements | Arrangements | 2 | todo |  |
| Playlist: clip source menu and picker panel | Clip picker | 2 | done | Patterns, audio and automations can each be picked as the brush. |
| Playlist: clip grouping | Clip groups | 2 | todo |  |
| Playlist: make unique | Make unique | 2 | todo |  |
| Playlist: snap | Playlist snap | 2 | done |  |
| Playlist: timeline selection and loop region | Loop region | 2 | todo |  |
| Playlist: audio clip fades, crossfades and gain handles | Clip fades and gain | 2 | in-progress | Fade in, fade out and gain handles. No automatic crossfades yet. |
| Playlist: audio clip properties (gain, pan, pitch, reverse, normalize) | Audio clip properties | 2 | in-progress | Gain, pan, pitch and reverse. No normalize or independent time-stretch yet. |
| Playlist: automation clip editing (curve shapes, step mode, LFO mode) | Automation curve editor | 2 | in-progress | Points, bends and holds are edited in the clip. No LFO mode or multi-point selection yet. |
| Event editor | Event automation editor | 2 | todo |  |
| Playlist: audio clip stretch and pitch-shift | Audio clip stretch | 3 | todo | Candidate basis: Signalsmith Stretch (MIT). |
| Playlist: detect tempo and fit to tempo | Tempo detection | 3 | todo |  |
| Playlist: consolidate (freeze) tracks | Bounce in place | 3 | todo |  |
| Playlist: Deverb | Reverb removal | 5 | todo | Machine-learning feature not named in the plan; no candidate basis identified. |
| Mixer: insert tracks, master track and current track | Mixer tracks | 1 | done | FL has 500 insert tracks, one master and one current track. |
| Mixer: track fader, pan, mute and solo | Track level controls | 1 | done |  |
| Mixer: peak meters | Level meters | 1 | done | The phase-0 spike already shows live meters. |
| Mixer: 10 effect slots per track | Effect slots | 2 | done |  |
| Mixer: track routing and send levels | Routing and sends | 2 | done |  |
| Mixer: sidechain routing | Sidechain | 2 | todo |  |
| Mixer: integrated 3-band track EQ | Track EQ | 2 | todo |  |
| Mixer: phase invert, swap left/right and stereo separation | Track stereo utilities | 2 | todo |  |
| Mixer: plugin delay compensation (automatic and manual) | Plugin delay compensation | 2 | in-progress | Automatic compensation across the whole routing graph. No manual offset yet. |
| Mixer: track docks and layout views | Mixer layouts | 2 | todo |  |
| Mixer: track states (presets) | Mixer track presets | 2 | todo |  |
| Mixer: waveform meter view | Waveform meters | 2 | todo |  |
| Mixer: multi-track selection | Multi-track selection | 2 | todo |  |
| Mixer: external audio input and output per track | Track audio I/O | 3 | todo |  |
| Mixer: track record arm and disk recording | Record arm | 3 | todo |  |
| Mixer: render tracks to wave files | Render mixer tracks | 3 | todo |  |
| Browser: folder tree | Browser | 1 | done |  |
| Browser: sample preview | Sample preview | 1 | done |  |
| Browser: waveform preview | Waveform preview | 1 | done | Inline waveforms were added in FL Studio 2026. |
| Browser: search | Browser search | 1 | in-progress | Filters folders that have been opened. No recursive index yet. |
| Browser: drag and drop | Drag and drop | 1 | done |  |
| Browser: project backups folder | Backups list | 1 | todo |  |
| Browser: tags | Tags | 2 | todo |  |
| Browser: starred items | Favorites | 2 | todo |  |
| Browser: current project tab | Project tab | 2 | todo |  |
| Project picker | Project overview | 2 | todo |  |
| Browser: plugin database | Plugin database | 4 | todo |  |
| Plugin picker | Plugin picker | 4 | todo |  |
| Browser: Library and Sounds tabs (FL Cloud content) | Online content tabs | 1 | wont-do | Reason: These tabs deliver Image-Line's own samples, loops and presets, some of them paid, which can never be shipped. Windfall's browser indexes the user's folders and Windfall's own CC0 content instead. |

## Instruments

| FL feature | Windfall | Phase | Status | Notes |
| --- | --- | --- | --- | --- |
| Drumaxx | TBD | 5 | todo |  |
| Harmor | TBD | 5 | todo |  |
| Kepler Exo | TBD | 5 | todo | Candidate basis: Surge XT (GPL-3.0). |
| Morphine | TBD | 5 | todo |  |
| Ogun | TBD | 5 | todo |  |
| Poizone | TBD | 5 | todo | Candidate basis: Surge XT (GPL-3.0). |
| Sakura | TBD | 5 | todo |  |
| Sawer | TBD | 5 | todo | Candidate basis: Surge XT (GPL-3.0). |
| Toxic Biohazard | TBD | 5 | todo | Candidate basis: Dexed (GPL-3.0). |
| Transistor Bass | TBD | 5 | todo | Candidate basis: Surge XT (GPL-3.0). |
| DirectWave Full | TBD | 5 | todo |  |
| Harmless | TBD | 5 | todo |  |
| Kepler | TBD | 5 | todo | Candidate basis: Surge XT (GPL-3.0). |
| Slicex | TBD | 5 | todo |  |
| SoundFont Player | TBD | 5 | todo |  |
| Sytrus | TBD | 5 | todo | Candidate basis: Dexed (GPL-3.0). |
| 3x OSC | Subtractive synth | 2 | done | The plan's phase-2 subtractive synth. |
| Autogun | TBD | 5 | todo |  |
| BassDrum | TBD | 5 | todo |  |
| BeepMap | TBD | 5 | todo |  |
| BooBass | TBD | 5 | todo |  |
| Channel Sampler | TBD | 1 | in-progress | The plan's phase-1 sampler. One-shot and gated playback, tuning, trim, reverse, volume envelope, cut groups. No loop points, filter or time-stretch yet. |
| DirectWave Player | TBD | 5 | todo |  |
| Drumpad | TBD | 5 | todo |  |
| FLEX | TBD | 5 | todo | Its preset packs are Image-Line content; a Windfall equivalent needs its own. Candidate basis: Vital or Surge XT (GPL-3.0). |
| Fruity DrumSynth Live | TBD | 5 | todo |  |
| Fruity DX10 | TBD | 5 | todo | Candidate basis: Dexed (GPL-3.0). |
| Fruity Granulizer | TBD | 5 | todo |  |
| Fruity Kick | TBD | 5 | todo | The manual calls it Fruit Kick. |
| Fruity Pad Controller (FPC) | TBD | 5 | todo | Its bundled kits are Image-Line content; needs own. |
| Fruity Slicer | TBD | 5 | todo | May share one Windfall plugin with the phase-3 slicer. |
| Fruity Slicer 2 | TBD | 3 | todo | The plan's phase-3 slicer. |
| GMS | TBD | 5 | todo | The manual names it Groove Machine Synth. |
| MiniSynth | TBD | 5 | todo | Candidate basis: Surge XT (GPL-3.0). |
| Plucked! | TBD | 5 | todo |  |
| SimSynth | TBD | 5 | todo | Candidate basis: Surge XT (GPL-3.0). |
| Speech Synthesizer | TBD | 5 | todo | Windows only in FL. |
| Wave Traveller | TBD | 5 | todo |  |
| FL Keys | TBD | 5 | todo | Its samples are Image-Line content; needs own. |
| Dashboard | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. The manual calls it a legacy plugin kept for old projects, superseded by Control Surface. |
| Fruity Vibrator | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. Windows only in FL. |

## Effects

| FL feature | Windfall | Phase | Status | Notes |
| --- | --- | --- | --- | --- |
| Transmitter | TBD | 5 | todo | New in FL Studio 2026. |
| LuxeVerb | TBD | 5 | todo |  |
| Pitch Shifter | TBD | 5 | todo | Candidate basis: Signalsmith Stretch (MIT). |
| Transient Processor | TBD | 5 | todo |  |
| Emphasis | TBD | 5 | todo |  |
| Transporter | TBD | 5 | todo |  |
| Gross Beat | TBD | 5 | todo |  |
| Hardcore (11 Guitar FX) | TBD | 5 | todo |  |
| Low Lifter | TBD | 5 | todo |  |
| Pitcher | TBD | 5 | todo | Needs real-time pitch detection. |
| Vintage Chorus | TBD | 5 | todo |  |
| Vintage Phaser | TBD | 5 | todo |  |
| Frequency Shifter | TBD | 5 | todo |  |
| Hyper Chorus | TBD | 5 | todo |  |
| Maximus | TBD | 5 | todo |  |
| Multiband Delay | TBD | 5 | todo |  |
| Spreader | TBD | 5 | todo |  |
| Vocodex | TBD | 5 | todo |  |
| Control Surface | TBD | 5 | todo |  |
| Distructor | TBD | 5 | todo |  |
| Effector (12 FX) | TBD | 5 | todo |  |
| EQUO | TBD | 5 | todo |  |
| Frequency Splitter | TBD | 5 | todo |  |
| Fruity Balance | TBD | 5 | todo |  |
| Fruity Blood Overdrive | TBD | 5 | todo |  |
| Fruity Chorus | TBD | 5 | todo |  |
| Fruity Compressor | Compressor | 2 | done | Phase-2 core compressor per the plan. |
| Fruity Convolver | TBD | 5 | todo | Impulse responses shipped with FL are Image-Line content; needs own. |
| Fruity Delay 2 | TBD | 5 | todo |  |
| Fruity Delay 3 | Delay | 2 | done | Phase-2 core delay per the plan. |
| Fruity Delay Bank | TBD | 5 | todo |  |
| Fruity Fast Dist | TBD | 5 | todo |  |
| Fruity Filter | TBD | 5 | todo |  |
| Fruity Flanger | TBD | 5 | todo |  |
| Fruity Flangus | TBD | 5 | todo |  |
| Fruity Formula Controller | TBD | 5 | todo |  |
| Fruity HTML NoteBook | TBD | 5 | todo |  |
| Fruity Limiter | Limiter | 2 | done | Phase-2 core limiter per the plan. |
| Fruity Love Philter | TBD | 5 | todo |  |
| Fruity LSD | TBD | 5 | todo | Windows only in FL (built on Microsoft DirectX). |
| Fruity Multiband Compressor | TBD | 5 | todo |  |
| Fruity NoteBook | TBD | 5 | todo |  |
| Fruity NoteBook 2 | TBD | 5 | todo |  |
| Fruity PanOMatic | TBD | 5 | todo |  |
| Fruity Parametric EQ | TBD | 5 | todo |  |
| Fruity Parametric EQ2 | Parametric EQ | 2 | done | Phase-2 core EQ per the plan. |
| Fruity Phaser | TBD | 5 | todo |  |
| Fruity Reeverb 2 | Reverb | 2 | done | Phase-2 core reverb per the plan. |
| Fruity Scratcher | TBD | 5 | todo |  |
| Fruity Send | TBD | 5 | todo |  |
| Fruity Soft Clipper | TBD | 5 | todo |  |
| Fruity Squeeze | TBD | 5 | todo |  |
| Fruity Stereo Enhancer | TBD | 5 | todo |  |
| Fruity Stereo Shaper | TBD | 5 | todo |  |
| Fruity Vocoder | TBD | 5 | todo |  |
| Fruity WaveShaper | TBD | 5 | todo |  |
| Fruity X-Y Controller | TBD | 5 | todo |  |
| Fruity X-Y-Z Controller | TBD | 5 | todo |  |
| Patcher | TBD | 4 | todo | The plan's phase-4 modular plugin rack. |
| Fruity Peak Controller | TBD | 5 | todo |  |
| Razer Chroma | TBD | 5 | todo | Depends on Razer's own SDK; check its license against GPL-3.0 before starting. |
| Soundgoodizer | TBD | 5 | todo |  |
| Tuner | TBD | 5 | todo |  |
| VFX Color Mapper | TBD | 5 | todo | Only works inside FL's modular rack (Patcher); depends on fx-patcher. |
| VFX Envelope | TBD | 5 | todo | Only works inside FL's modular rack (Patcher); depends on fx-patcher. |
| VFX Level Scaler | TBD | 5 | todo | Only works inside FL's modular rack (Patcher); depends on fx-patcher. |
| VFX Keyboard Splitter | TBD | 5 | todo | Only works inside FL's modular rack (Patcher); depends on fx-patcher. |
| VFX Key Mapper | TBD | 5 | todo | Only works inside FL's modular rack (Patcher); depends on fx-patcher. |
| VFX Sequencer | TBD | 5 | todo | Only works inside FL's modular rack (Patcher); depends on fx-patcher. |
| Emphasizer | TBD | 5 | todo |  |
| VFX Script | TBD | 6 | todo | Only works inside FL's modular rack (Patcher); depends on fx-patcher. Grouped with the plan's phase-6 scripting. |
| Fruity 7 Band EQ | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. |
| Fruity Bass Boost | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. |
| Fruity Center | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. |
| Fruity Delay | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. |
| Fruity Fast LP | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. |
| Fruity Free Filter | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. |
| Fruity Mute 2 | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. |
| Fruity Phase Inverter | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. |
| Fruity Reeverb | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. The manual calls it a legacy plugin and recommends its successor. |

## Visual and video

| FL feature | Windfall | Phase | Status | Notes |
| --- | --- | --- | --- | --- |
| Fruity Video Player | TBD | 6 | todo | The plan's phase-6 video player. |
| Fruity Big Clock | TBD | 5 | todo |  |
| Fruity dB Meter | TBD | 5 | todo |  |
| Fruity Spectroman | TBD | 5 | todo |  |
| Video Visualizer (ZGameEditor) | TBD | 6 | todo | The plan's phase-6 visualizer. The manual names it ZGameEditor Visualizer. |
| Wave Candy | TBD | 5 | todo |  |
| Fruity Dance | TBD | 5 | todo | manual-only: listed in the online manual's plugin index, not on the compare-editions page, so outside its 39/71/6 totals. Its default character art is Image-Line's; a Windfall equivalent needs its own artwork. |

## Audio editors

| FL feature | Windfall | Phase | Status | Notes |
| --- | --- | --- | --- | --- |
| Newtone | TBD | 5 | todo | Needs pitch detection (the plan lists it under machine-learning features) and time-stretch. Candidate basis: Signalsmith Stretch (MIT). |
| Edison | TBD | 3 | todo | The plan's phase-3 audio editor. FL's version includes noise removal, time-stretch, EQ, convolution reverb and script tools. |
| Newtime | TBD | 5 | todo | Candidate basis: Signalsmith Stretch (MIT). |

## File formats and plugin hosting

| FL feature | Windfall | Phase | Status | Notes |
| --- | --- | --- | --- | --- |
| Project file save and open (.flp) | Windfall project format | 1 | done | Windfall uses its own open format: readable JSON plus audio in a folder or zip, with a version number. Windfall saves its own readable JSON format, .windfall. Reading .flp files is a separate row. |
| Zipped project (.zip) | Project archive | 1 | todo |  |
| Save new version | Save new version | 1 | todo |  |
| Autosave and backups | Autosave and backups | 1 | done | The plan builds autosave and timestamped backups into the project format. |
| New from template and Save as template | Project templates | 2 | todo | FL's bundled templates reference Image-Line content; needs own. |
| FL Studio project (.flp) as an import source | FL project import | 4 | todo | Candidate basis: PyFLP (GPL-3.0). Identical sound is only possible where every plugin has a Windfall equivalent or is an installed third-party plugin. |
| Back up to FL Cloud | Cloud backup (optional) | 6 | todo | New in FL Studio 2026. The plan lists optional cloud backup in phase 6. |
| State file (.fst) | Presets | 2 | todo | Windfall presets use its own format; FL's bundled presets cannot be shipped. |
| Score file (.fsc) | Score files | 2 | todo | FL 2026 treats .mid and .fsc interchangeably for scores. |
| Sample import: WAV | WAV import | 1 | done |  |
| Sample import: MP3 | MP3 import | 3 | todo | The plan lists MP3 as a read format without naming a phase; placed with the other codecs in phase 3. |
| Sample import: OGG | OGG import | 3 | todo | The plan lists OGG as a read format without naming a phase; placed with the other codecs in phase 3. |
| Sample import: FLAC | FLAC import | 3 | todo | unverified: FLAC as an FL sample import format (the manual pages checked name WAV, MP3 and OGG). The plan lists FLAC as a read format. |
| MIDI file import | MIDI import | 4 | todo |  |
| Sample import: ReCycle loops (.rex, .rx2, .rcy) | REX loop import | 6 | todo | Not in the plan. REX decoding normally relies on a proprietary SDK; check its license against GPL-3.0 (may become wont-do). |
| Sampler sources: DrumSynth (.ds), SimSynth (.syn) and speech (.speech) presets | Synth-preset sample sources | 6 | todo | Not in the plan; legacy formats. |
| BeatCreator/BeatSlicer grid file (.zgr) | ZGR import | 6 | todo | Not in the plan; legacy format. |
| Export: WAV | WAV export | 1 | done |  |
| Export: MP3 | MP3 export | 3 | todo |  |
| Export: OGG | OGG export | 3 | todo |  |
| Export: FLAC | FLAC export | 3 | todo |  |
| Export: MIDI file | MIDI export | 3 | todo |  |
| Export: split mixer tracks (stems) | Stem export | 3 | todo |  |
| Export: all playlist tracks | Track export | 3 | todo |  |
| Export options (song or pattern, tail, bit depth, dithering, resampling) | Export options | 3 | todo |  |
| Export: M4A (AAC) | M4A export | 6 | todo | Not in the plan's export list. AAC encoder licensing must be checked against GPL-3.0. |
| Export: loop, slice and note markers in WAV files | WAV marker export | 6 | todo | Not named in the plan's phase table, so placed in phase 6. |
| WavPack compressed audio | WavPack support | 6 | todo | Not in the plan. |
| Plugin hosting: VST3 | VST3 hosting | 4 | todo | The plan notes the VST3 SDK has been MIT-licensed since October 2025. |
| Plugin hosting: CLAP | CLAP hosting | 4 | todo |  |
| Plugin hosting: Audio Unit | AU hosting | 4 | todo | macOS only. |
| Plugin hosting: VST2 | VST2 hosting | 4 | todo | unverified: licensing path. Not in the plan; Steinberg no longer issues VST2 SDK licenses, so this needs a decision and may become wont-do. |
| Plugin manager (scan and verify) | Plugin scanner | 4 | todo | The plan runs scanning in a separate process. |
| Bridged plugins (separate process) | Out-of-process plugins | 4 | todo | The plan's crash protection. |
| Plugin wrapper options (smart disable, fixed-size buffers, threaded processing, scaling, detached window) | Plugin host options | 4 | todo |  |
| FL Studio as a VST or AU plugin | Windfall as a plugin | 6 | todo | The plan lists this as 'possible later' and assigns no phase. |

## Workflow, MIDI and settings

| FL feature | Windfall | Phase | Status | Notes |
| --- | --- | --- | --- | --- |
| Audio settings (driver, device, sample rate, buffer length) | Audio settings | 0 | in-progress | The plan uses cpal: WASAPI and ASIO on Windows, CoreAudio on macOS, ALSA, JACK and PipeWire on Linux. Device, sample rate and buffer through WASAPI, CoreAudio, ALSA and JACK. ASIO is behind a build feature that is off. |
| Transport (play, stop, record, pattern/song mode, song position) | Transport | 1 | in-progress | Play, stop, pattern and song mode, song position. No recording yet. |
| Tempo and tempo tapper | Tempo | 1 | in-progress | Tempo field. No tap tempo yet. |
| Metronome | Metronome | 1 | todo |  |
| Typing keyboard to piano keyboard | Typing keyboard | 1 | todo |  |
| Themes | Light and dark themes | 1 | done | Light and dark themes. |
| Interface scaling | Interface scaling | 1 | todo |  |
| Hint bar | Hints | 1 | done |  |
| Output meter and CPU/memory panels | Status meters | 1 | in-progress | Master meter and engine load. No memory readout. |
| Undo and edit history | Undo history | 2 | done | The plan calls for linear undo with a visible history list. |
| Multithreaded processing | Multi-core mixing | 2 | todo |  |
| Global snap | Global snap | 2 | todo |  |
| Detached windows | Detachable panels | 2 | todo |  |
| Keyboard and mouse shortcuts | Shortcuts with optional FL keymap | 2 | done | Shortcuts are scoped to the focused panel. An optional preset follows FL Studio's keys. |
| Tools menu macros | Utility commands | 2 | todo | Maps to named commands in Windfall's command palette. |
| Project info (title, author, genre, comments) | Project info | 2 | todo |  |
| Project settings (time signature, timebase, panning law) | Project settings | 2 | in-progress | Tempo, time signature and swing. |
| MIDI settings (input and output devices, ports, controller type) | MIDI settings | 3 | todo |  |
| Note recording from MIDI input | Note recording | 3 | todo |  |
| Step editing (step entry) | Step entry | 3 | todo |  |
| Score logger (dump score log to pattern) | Note logger | 3 | todo |  |
| Recording count-in | Count-in | 3 | todo |  |
| Loop recording (takes) | Loop recording | 3 | todo |  |
| Automation recording | Automation recording | 3 | todo |  |
| Link to controller (remote control settings, mapping formula, smoothing) | Controller mapping | 3 | todo |  |
| Multilink to controllers | Multi-link mapping | 3 | todo |  |
| Pickup (takeover) mode | Pickup mode | 3 | todo |  |
| MIDI clock output (send master sync) | MIDI clock out | 3 | todo |  |
| Internal controller linking | Internal modulation links | 5 | todo | Depends on the controller plugins in the effect and core lists. |
| Audio to notes (pitch editor 'Send score') | Audio-to-MIDI | 5 | todo | Candidate basis: Basic Pitch (Apache-2.0). |
| Remix a song wizard | Remix wizard | 5 | todo | New in FL Studio 2026. Depends on stem separation. |
| Performance mode | Performance mode | 6 | todo | The plan's phase-6 performance mode. |
| MIDI scripting (Python device scripts) | Controller scripting | 6 | todo | Part of the plan's phase-6 scripting. |
| Preconfigured controller support | Controller profiles | 6 | todo | Windfall would deliver these as controller scripts. |
| Touch controllers (virtual keyboard and drum pads) | Touch keyboard and pads | 6 | todo | Not named in the plan's phase table, so placed in phase 6. |
| Chord detection panel | Chord display | 6 | todo | New in FL Studio 2026. Not named in the plan's phase table, so placed in phase 6. |
| Mastering on export (FL Cloud) | Automatic mastering | 6 | todo | Runs as Image-Line's online service; a Windfall version would have to run locally. Not named in the plan's phase table, so placed in phase 6. |
| Interface languages | Translations | 7 | todo |  |
| Help menu and guided tutorials | Help and tutorials | 7 | todo | Guided tutorials were added in FL Studio 2026. |
