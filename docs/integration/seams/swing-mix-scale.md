# Channel swing mix scaling

The channel timing section halves or doubles that channel's swing mix. Halve
divides the continuous mix by two; halve of a straight mix stays straight.
Double stops at full. Changes smaller than 0.001 send no command, and each
click reads the latest timing before updating swing mix in one undo step.

The existing Half preset still sets 50%. Gate and shift stay as they are
because the command preserves the latest timing settings. Project swing is
unchanged.
