# Sample trim presets

The sampler Sample section offers Whole (0 to 1), First half (0 to 0.5),
Second half (0.5 to 1), and Middle (0.25 to 0.75). Each preset sets both trim
edges in one undo step. Whole stores 0 and 1, so the channel plays the whole
sample. The chosen sample, gain, tune, reverse, and the loop stay as they are.
The matching preset is disabled when both edges differ by less than 0.001.
Missing start and end values are treated as 0 and 1, respectively.

Each click reads the latest channel source and only sends an `updateSampler`
command when that source is still a sampler and its trim differs. The patch
contains only `start` and `end`.
