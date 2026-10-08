# Retained audio-take comping — source implementation

Select retained loop takes (or other audio source clips) in the playlist and
choose **Comp selected audio takes…** from the clip menu or command palette.
The comp editor captures those exact source clips, proposes ranges across
their common span, and lets each range choose its source. Range positions use
absolute song beats. Reset ranges partitions the span at the chosen size;
individual ranges can be split, removed or have their boundaries changed.
The editor offers a new-lane name, crossfade duration and original-source mute.

`CompAudioClips` is one native project/history command. It bounds source and
range counts, requires unchanged captured audio clips, validates each range
against its selected source and rejects overlapping selection ranges. Adjacent
ranges of the same source coalesce, avoiding artificial edit seams. Adjacent
ranges from different sources can extend into their original audio windows
to form bounded equal-power fades at the join. A source edge with no spare
audio receives only the available overlap. Gaps between ranges stay gaps.

The composite consists of normal editable audio clips on a new playlist lane;
the command API can also target an existing lane. It retains source sample,
gain, pan, reversal, tuning/stretch and Mixer/Direct output routing. New clip
offsets follow the existing playlist trim contract. Remaining source fade
lengths are bounded to each selected window. Original source clips remain in
the document, optionally muted, with every original WAV retained. Applying or
undoing the command adds/removes the entire composite and restores source
mutes atomically. There is no rendered intermediate WAV or duplicate sample.
The saved composite uses the ordinary clip/project/save/export pipeline.

The first implementation accepts 1–256 captured sources and 1–2048 ranges.
The independent selection workflow creates one composite lane. Saved recording
associations and synchronized multitrack choices are now implemented through
the [take-group workflow](TAKE-GROUPS.md).
The browser and native document share the Rust command, with provisional
source bindings; the checked-in WASM still awaits regeneration. No builds,
tests, generators, listening checks, browser QA or review rounds were run.
No GitHub CI or Actions are used.
