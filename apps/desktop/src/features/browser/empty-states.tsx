import { Button } from "@/components/ui/button"
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
  EmptyDescription,
  EmptyContent,
} from "@/components/ui/empty"
import { Spinner } from "@/components/ui/spinner"
import { runAction } from "@/lib/actions"

import { useLibraryStore } from "./library-store"
import {
  loadRoots,
  requestFilterFocus,
  setFilter,
  useBrowserStore,
} from "./store"

export function TreeFooter({
  filtering,
  rowCount,
}: {
  filtering: boolean
  rowCount: number
}) {
  const status = useBrowserStore((s) => s.rootsStatus)
  const error = useBrowserStore((s) => s.rootsError)
  const hasUserFolder = useBrowserStore((s) =>
    s.roots.some((r) => r.kind === "user")
  )
  const library = useLibraryStore()
  if (status === "error")
    return (
      <Empty role="alert">
        <EmptyHeader>
          <EmptyTitle>The browser could not be read.</EmptyTitle>
          <EmptyDescription>{error}</EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <Button variant="outline" size="sm" onClick={() => void loadRoots()}>
            Try again
          </Button>
        </EmptyContent>
      </Empty>
    )
  if (status !== "ready")
    return rowCount === 0 ? (
      <p className="flex items-center gap-1.5 p-2.5 text-muted-foreground">
        <Spinner />
        Reading the browser...
      </p>
    ) : null
  if (filtering) {
    if (rowCount > 0 || library.error) return null
    if (library.pending || library.results?.status === "indexing")
      return (
        <p
          role="status"
          className="flex items-center gap-1.5 p-2.5 text-muted-foreground"
        >
          <Spinner />
          Searching the library...
        </p>
      )
    return (
      <Empty role="status">
        <EmptyHeader>
          <EmptyTitle>No library files match these filters.</EmptyTitle>
          <EmptyDescription>
            Try fewer terms, clear Starred or the tag filter, or refresh after
            changing files on disk.
          </EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              setFilter("")
              useLibraryStore.setState({ favoritesOnly: false, tags: [] })
              requestFilterFocus()
            }}
          >
            Show everything
          </Button>
        </EmptyContent>
      </Empty>
    )
  }
  return hasUserFolder ? null : (
    <Empty>
      <EmptyHeader>
        <EmptyDescription>
          Your own samples go here too. Add a folder and it stays in the
          browser.
        </EmptyDescription>
      </EmptyHeader>
      <EmptyContent>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void runAction("browser.addFolder")}
        >
          Add folder…
        </Button>
      </EmptyContent>
    </Empty>
  )
}
