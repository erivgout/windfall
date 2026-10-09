# Fine pitch preset stepping

The piano-roll fine-pitch lane offers Previous preset and Next preset for the
selected notes, or every note when none are selected. Each chosen note steps from
an octave down toward an octave up. An octave down stays an octave down when
moved previous. An octave up stays an octave up when moved next.

A pitch between two presets moves to the neighboring preset. A pitch within
0.001 cents of a preset counts as that preset. A note with no expression counts
as in tune. A note already at the end stays as it is while another chosen note
can still move. Release stays as it is.
