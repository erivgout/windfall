# Offline mixer track rendering — source implementation

This packet supplies the selected/armed workflow for `win-mixer-render-tracks`.
It records source coverage only. Builds, tests, review rounds, browser checks and
artifact generation remain deferred until the feature pass finishes. No GitHub
CI or Actions are used.

## Selecting an export

The mixer toolbar, insert/Master context menus and command palette expose
“Render selected mixer tracks” and “Render armed mixer tracks”. Selection uses
the mixer's multi-track selection with its existing primary-track fallback.
Armed rendering reads saved mixer record arms. A request captures the insert IDs
and project generation before opening the existing export dialog. Current is
excluded because it is an analysis utility. Choosing Master includes the full
mix, including when Master is the only selected or armed track.

The opened draft defaults to track-output stems, captured IDs, mixer-order file
numbers and a separate stem folder. It uses the existing pattern/song, selected
song-region, format, sample rate, bit depth/bitrate, duration and tail controls.
The user chooses the output path and can change every export option before
starting. The stem panel also has “Use mixer selection”, “Use armed tracks” and
“Clear track selection” controls alongside individual insert checkboxes. Current
is removed from that checkbox inventory.

The one-shot request is consumed when the form opens and cleared on project
replacement. A still-open form refuses a request from a replaced project. A
removed requested insert produces a visible selection error until the user
chooses a valid track set. Request IDs never silently become same-numbered
tracks in a replacement project. These are transient UI choices, not saved
musical data or record-arm mutations.

## Existing native renderer used by this workflow

The workflow submits the existing `ExportOptions.stems` contract. Track-output
rendering processes the complete project in one offline pass and taps selected
post-effect/post-fader track outputs. This retains routing and detector inputs
from the rest of the song. The source-through-Master option instead renders each
source in isolation through its buses and Master; nonlinear effects therefore
respond to each isolated pass. Its detector sources follow that isolated source
policy as well. Full-mix inclusion is a separate stream. This packet does not
change that engine policy.

The existing native export path handles independent file encoders, bounded
streamed output, mixer-order naming and name collision handling, sampler/plugin
preparation, latency alignment, progress, cancellation and completion/error
notifications. The browser adapter retains its existing render/export behavior;
it does not gain native device or hosted-plugin capabilities from these controls.

## Deferred validation

Later QA must exercise selected IDs after reorder/removal, armed inserts and
Master, Current exclusion, project replacement with reused IDs, both stem modes,
sidechain inputs, pattern/song/region scopes, format choices, naming/collisions,
long-file streaming, cancellation/error cleanup and progress. Existing renderer
tests have not been rerun during this source packet. The wider project remains
in scope and the whole-project goal remains active.
