# Automation filter

The playlist picker's Automation section has a **Filter automations** input
when the project has at least one automation. It matches the automation
name or the target words already displayed in each row, ignoring case and
trimming the query. A blank query shows every automation in its original
order. When nothing matches, the section shows **No automations match.**
When the project has no automations, the existing empty message stays and
the filter is hidden.

The filter only hides automation rows. It does not delete automations or
change the brush, even when the selected automation is hidden. The pattern
and audio lists are not filtered by it.

The query is local to the picker and is not saved. Unmounting the picker
clears it. Arrow keys inside the input stop before parent handlers, so they
stay in the field.

The pure `matchingAutomationIds` helper is in
`apps/desktop/src/features/playlist/automation/picker-filter.ts`.

Focused verification from `apps/desktop`:

- `pnpm test -- src/features/playlist/automation/picker-filter.test.ts`
