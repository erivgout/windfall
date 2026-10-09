# Send gain scaling

The send knob's Send level submenu offers Half and Double after Off, Quiet,
Unity, and Loud. Each command sets that one send's gain in one undo step,
using its latest gain when clicked. Other sends are unchanged. If the send
has disappeared or the change is smaller than 0.001, no command is sent.

Half divides the linear gain by two without rounding. Off stays in place,
and half of silence stays silent. Double stops at the maximum gain of 2.
Half and Double are disabled when their change would be smaller than 0.001.

Off, Quiet, Unity, Loud, and Remove send are unchanged. The fader is unchanged.
