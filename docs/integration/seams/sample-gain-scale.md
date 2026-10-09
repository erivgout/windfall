# Sample gain scale

The sampler Sound section offers Half and Double after the sample-gain
presets. Each command sets the sample gain in one undo step. This is the
sample level before the channel volume, so the channel volume stays as it
is. Half divides the current gain by two. Double stops at the maximum gain
of 2. Half of silence stays silent. Gains are not rounded.

Each button is disabled when its change is smaller than 0.001. Each click
reads the latest channel source and only sends one `updateSampler` command
when that source is still a sampler and the latest gain changes by at least
0.001. The patch contains only `gain`.

Quiet, Unity, and Loud stay as they are. Tune, fine tune, root, reverse, and
cut group stay as they are, as do the knobs.
