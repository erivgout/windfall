# Editable ghost-note source implementation

The piano roll's View menu, toolbar and action palette offer Edit ghost notes
(Alt+Shift+G). Enabling it also shows ghost notes. The preference is retained
across app restarts; turning ghost visibility off suspends ghost editing.

With editing enabled, a left press on a ghost switches the source channel
and feeds the same press into its ordinary note editor. Draw can immediately
move/resize it, Select can select it and Erase can delete it. Source notes are
not copied into the previously selected lane. Regular note editing commands,
capture safeguards and undo apply to the source lane. Visible notes in the
current channel take hit priority; a stamp or busy editor retains its own
input ownership. Ghost hits use the existing spatial index/sliver tolerance.

Ghost rects carry their saved note ids. The source channel is resolved from
the live project at press time, and the current viewport is saved for the new
lane before the React channel selection updates. Existing source-channel
audition and note properties follow the selected lane.

No builds, tests, QA, reviews or artifact generation ran for this source pass.
