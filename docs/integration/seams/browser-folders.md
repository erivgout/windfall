# Browser folder commands

The browser menu and command palette offer **Collapse folders**
(`browser.collapseFolders`) and **Expand loaded folders**
(`browser.expandLoadedFolders`). Neither command has a shortcut.

Collapse closes every folder by clearing `useBrowserStore.expanded`.
Expand opens only folders the browser has already read: every root and
folder entry in a ready listing, under each root the folder belongs to.
Loading and failed listings contribute no child folders. Expand does not
invent intermediate paths or include files.

Neither command reads the disk. Both preserve cached listings and the
selection, and use the store's existing setter to save the open folders.
When the requested folder set already matches, the store is unchanged.

The pure `loadedFolderIds` helper is in
`apps/desktop/src/features/browser/folder-expand.ts`. Row ids and path
containment use `rowId` and `isUnder` from `tree-model.ts`, including when
roots overlap.

Focused verification from `apps/desktop`:

`pnpm test -- src/features/browser/folder-expand.test.ts`
