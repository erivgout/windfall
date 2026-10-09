# Getting started

Parity id: `wf-help-tutorials`.

Help > Getting started runs `help.gettingStarted` and opens
`features/help/getting-started-dialog.tsx`, mounted by `AppShell` alongside the
other dialogs. Keyboard shortcuts still opens the command palette, and About
keeps its existing behavior.

This is a list of existing actions, not a guided tour that watches the project.
The ordered buttons run `channel.add`, `view.pianoRoll`, `view.playlist`,
`transport.play`, and `file.export` through the existing action registry, using
the actions' existing enabled states and behavior. There is no automatic step
advancement, tutorial overlay, or new project command.

Opening and closing help only changes its local, non-persisted open state and
dispatches no command. Each step leaves help open. Its state is separate from
the shared dialog slot so Export can open the existing audio export dialog
without replacing help. Project replacement closes help through
`onProjectReplaced`; closing removes that lifecycle subscription.

Coverage lives in `apps/desktop/src/features/help/getting-started.test.tsx`.
Run from `apps/desktop`: `pnpm test -- src/features/help/getting-started.test.tsx`.
