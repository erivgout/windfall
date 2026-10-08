# Channel groups and filtering

Implementation is present; QA and artifact builds are deferred under the
user's feature-first instruction.

Each channel stores an optional named `group`. Missing membership in an older
project means ungrouped. Group names are trimmed, accept Unicode, have a
128-byte UTF-8 limit and reject control characters. Membership is organization
metadata and does not mute channels or change notes, source or mixer routing.
Cloning and the existing project/history paths carry it with the channel.

Native commands assign a set of channels, rename a group, or remove a group.
Assignment checks all ids before editing, ignores repeated ids, and uses one
undo entry. Rename to an existing name merges membership. Removing a group
moves its channels to Ungrouped; it does not delete channels. Named groups
are derived from current membership, so an empty group is not retained as a
separate project object. Names are case-sensitive.

The rack toolbar offers All channels, Ungrouped and each named group, with
counts. The group manager assigns multiple checked channels to an existing
or new group and includes Select all/visible/none. The row context menu has
a group submenu. Creation, rename, removal, management and Show all are
also registered palette actions.

Filtering chooses a visible selected channel when needed. Navigation to a
hidden channel reveals all channels. Project replacement clears the transient
filter and manager. Deleted/renamed last memberships reset invalid filters.
Filtered drag gaps map to the global rack order while retaining hidden
channels' relative order. New channels remain discoverable through All.

The manager captures project generation and revision, refuses a stale review,
and closes on replacement or history intent. Other naming actions retain the
existing prompt/dialog and document-dispatch lifecycle.

Type declarations are synchronized at source level for the implementation
pass. The checked-in shared WASM predates these new commands and needs
regeneration in the deferred build/artifact pass before browser execution.
No builds, QA runs, reviews, tests or GitHub Actions were performed for this
delivery. Native undo/persistence, filtered navigation/dragging and UI/runtime
behavior remain to be checked in the later QA pass.
