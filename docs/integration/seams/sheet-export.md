# Piano-roll sheet export

Windfall's piano-roll action **Export sheet music…** (`pianoRoll.exportSheetMusic`)
is enabled when a pattern channel is open, including an empty channel. It is
available in the piano-roll menus and action registry. There is no engraved
preview. The action writes MusicXML for a notation program to open and engrave.

## Score builder

`apps/desktop/src/features/piano-roll/sheet-music.ts` builds uncompressed
score-partwise MusicXML 4.0 with one part, using only the open channel's lane.
Divisions equal Windfall's project ticks per quarter note (`PPQ`, currently 960).
MIDI key 60 becomes C4; chromatic pitches use sharp alterations. Start times and
lengths retain their tick precision. Same-start notes become chords, gaps become
rests, and different-start overlaps use independent voices with measure backups.
Notes crossing bar lines are split with sounding and notation ties. Each measure
is filled with rests through its end; an empty channel produces exactly one
whole-measure rest. Notes beyond the pattern loop length are retained in export.

The score uses the pattern's tick-zero meter override, otherwise its base time
signature, otherwise 4/4. Later meter changes are not exported. Project-level
meter inheritance does not replace the specified 4/4 fallback.

The chord ordering and tie elements follow the
[MusicXML chord reference](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/chord/)
and [note reference](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/note/).
This is a pitch/duration export; expression curves, velocity, articulation,
instrument rendering, and engraved layout are outside this seam.

## Desktop save seam

`sheet-export.ts` captures the open pattern channel before opening a dialog.
`Backend.sheetMusicSave` uses the existing Tauri `save` dialog with a `.musicxml`
default filename and MusicXML/XML filters. A cancelled dialog returns `null` and
does not invoke a write. The native `write_sheet_music` command writes UTF-8 XML
on a blocking task and reports write errors through the existing action error
handling. It does not dispatch edits, change project history, or access the audio
engine. The browser mock offers a MusicXML download after its export-path prompt.

## Focused verification

Run from `apps/desktop`:

```text
pnpm test src/features/piano-roll/sheet-music.test.ts
```

The builder tests cover C4 pitch/duration, channel isolation, same-start chords,
leading/interior rests, an empty channel, initial meter selection, bar-crossing
ties, overlapping voices, and escaped XML labels. The save dialog and Rust
command are not exercised by this unit test.
