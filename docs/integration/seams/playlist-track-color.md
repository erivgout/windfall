# Playlist track color

Windfall saves `PlaylistTrack.color` as a `u32`: **color 0 is uncolored**, and nonzero values are `0xRRGGBB`. Missing color fields in old projects load as 0. Zero is omitted when serializing, and newly added tracks start at 0. Playlist track solo is still absent.

The track header's right-click **Color** submenu offers the existing mixer `TRACK_COLORS` palette and **No color**. Choosing a different color dispatches one `updatePlaylistTrack` with `{ color }`; its undo label is **Color playlist track**. The document's existing track edit and playlist patch carry the color through save/load, undo/redo, and the desktop mirror. The two playlist track TypeScript bindings were updated by hand.

Nonzero color gives the header a subtle background tint and adds a full-width tint to the lane's canvas underlay. Zero or a missing desktop color leaves the existing header and lane appearance intact. Lane tint uses the existing visible track rows and pixel conversion, so scrolling, collapsed groups, and session-only row height overrides keep it aligned. The resize handle and link label retain their behavior.

The Rust command integration test covers new-track defaults, missing-field load, nonzero save/load, the undo label, undo/redo, and clearing back to zero. The desktop test covers the palette, one-command dispatch, mirror rendering, zero/missing color without lane tint, clearing, and a colored lane's resize alignment. Its backend response is mocked; the Rust test verifies command application.

From the repository root (PowerShell with the MSVC and Windows SDK library paths configured), redirect ts-rs exports to a discard directory:

```powershell
$env:TS_RS_EXPORT_DIR = "$PWD/target/ts-rs-discard"
cargo test -p windfall-project --test playlist_track_color
```

From `apps/desktop`:

```powershell
pnpm test -- src/features/playlist/track-color.test.tsx src/features/playlist/track-resize.test.tsx src/features/playlist/track-link.test.tsx
```
