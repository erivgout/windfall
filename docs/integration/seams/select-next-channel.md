# Select the next muted or solo channel

"Select next muted channel" (`channel.selectNextMuted`) and "Select next solo
channel" (`channel.selectNextSolo`) appear immediately after "Reset levels" in
the rack menu and each channel button menu, muted first and solo second. Neither
has a shortcut.

Each command only changes which channel is selected. Mute and solo stay as they
are. Neither command dispatches a project command or opens channel settings.
When the open channel is the only match, the selection stays. When no channel
matches, the selection also stays. Each action is enabled only when another
matching channel can be selected.

The pure `nextFlaggedChannelId(channels, currentId, flag)` helper in
`apps/desktop/src/features/channel-rack/select-next.ts` searches all project
channels in the given rack order, starting after the current channel and wrapping
to the beginning. It excludes the current channel and returns `null` when no
other channel matches. A null or missing current id starts at the first matching
channel. Muted and solo are checked independently, and the input is not mutated.

Run the helper tests from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/select-next.test.ts
```
