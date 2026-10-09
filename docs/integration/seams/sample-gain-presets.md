# Sample gain presets

The sampler Sound section offers Quiet (0.5), Unity (1), and Loud (1.5).
Each preset sets the sample gain in one undo step. Unity stores 1, which is
0 dB. This is the sample level before the channel volume, so the channel
volume stays as it is. Tune and the loop stay as they are. The matching
preset is disabled when the gain differs by less than 0.001.

Each click reads the latest channel source and only sends an `updateSampler`
command when that source is still a sampler and its gain differs. The patch
contains only `gain`.
