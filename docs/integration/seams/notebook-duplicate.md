# Duplicate notebook page

Duplicate page inserts a copy of the selected page immediately after it, keeps
the same title and body, and selects the copy. Following pages keep their order.
The pure `duplicatePage` helper returns a new array and a separate page object
without mutating the input. Invalid indices, an empty list, and a list at the
page limit return `null`.

The copy stays in the draft until Save notebook. Duplicating does not dispatch
a command or save by itself. The existing save validation still limits titles
to 128 UTF-8 bytes and bodies to 16,384 UTF-8 bytes.

A notebook holds at most eight pages. An eighth page cannot be duplicated when
the notebook is full. Duplicate page is disabled while saving, for an empty
draft showing only the temporary blank page, and when the page count is eight.

Focused verification from `apps/desktop`:

- `pnpm test -- src/features/notebook/duplicate-page.test.ts`
- `pnpm test -- src/features/notebook/panel.test.tsx`
