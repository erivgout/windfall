import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { runAction } from "@/lib/actions"

import {
  loadRoots,
  requestFilterFocus,
  setFilter,
  useBrowserStore,
} from "./store"

function RootsUnreadable({ message }: { message: string | null }) {
  return (
    <div role="alert" className="flex flex-col items-start gap-2 p-2.5">
      <p className="font-medium">The browser could not be read.</p>
      {message && <p className="text-muted-foreground">{message}</p>}
      <Button variant="outline" size="sm" onClick={() => void loadRoots()}>
        Try again
      </Button>
    </div>
  )
}

function NoMatches({ filter }: { filter: string }) {
  return (
    <div className="flex flex-col items-start gap-2 p-2.5" role="status">
      <p className="font-medium wrap-anywhere">
        Nothing is named like “{filter.trim()}”
      </p>
      <p className="text-muted-foreground">
        The filter looks only in folders you have opened. Open a folder to
        include what is in it. Searching every folder at once comes in a later
        version.
      </p>
      <Button
        variant="outline"
        size="sm"
        onClick={() => {
          setFilter("")
          requestFilterFocus()
        }}
      >
        Show everything
      </Button>
    </div>
  )
}

function AddFolderPrompt() {
  return (
    <div className="m-2 flex flex-col items-start gap-2 rounded-md border border-dashed p-2.5">
      <p className="text-muted-foreground">
        Your own samples go here too. Add a folder and it stays in the browser.
      </p>
      <Button
        variant="outline"
        size="sm"
        onClick={() => void runAction("browser.addFolder")}
      >
        Add folder…
      </Button>
    </div>
  )
}

type TreeFooterProps = {
  filtering: boolean
  rowCount: number
}

/** What sits under the rows: a loading line, an error, or what to do next. */
export function TreeFooter({ filtering, rowCount }: TreeFooterProps) {
  const status = useBrowserStore((state) => state.rootsStatus)
  const error = useBrowserStore((state) => state.rootsError)
  const filter = useBrowserStore((state) => state.filter)
  const hasUserFolder = useBrowserStore((state) =>
    state.roots.some((root) => root.kind === "user")
  )

  if (status === "error") return <RootsUnreadable message={error} />
  if (status !== "ready") {
    return rowCount === 0 ? (
      <p className="flex items-center gap-1.5 p-2.5 text-muted-foreground">
        <Spinner className="size-3" />
        Reading the browser…
      </p>
    ) : null
  }
  if (filtering) return rowCount === 0 ? <NoMatches filter={filter} /> : null
  return hasUserFolder ? null : <AddFolderPrompt />
}
