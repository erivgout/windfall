import { useEffect, useState } from "react"

import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Spinner } from "@/components/ui/spinner"
import { Toggle } from "@/components/ui/toggle"
import { attempt } from "@/lib/errors"
import { backend, errorMessage } from "@/lib/ipc"

import {
  refreshLibrary,
  saveLibraryMetadata,
  useLibraryStore,
} from "./library-store"
import { useBrowserStore } from "./store"

/** Debounced requests and bounded polling. Late replies cannot replace a newer query or roots. */
export function LibraryControls() {
  const roots = useBrowserStore((s) => s.roots)
  const rootsStatus = useBrowserStore((s) => s.rootsStatus)
  const filter = useBrowserStore((s) => s.filter)
  const { favoritesOnly, tags, results, error, revision } = useLibraryStore()
  const tag = tags[0] ?? ""
  const availableTags = [
    ...new Set([...(results?.availableTags ?? []), ...tags]),
  ].sort()
  useEffect(() => {
    if (rootsStatus !== "ready") return
    let current = true
    let timer: ReturnType<typeof setTimeout>
    useLibraryStore.setState({ pending: true, error: null, results: null })
    async function search() {
      try {
        const results = await backend.librarySearch({
          query: filter,
          favoritesOnly,
          tags,
        })
        if (!current) return
        useLibraryStore.setState({ results, pending: false, error: null })
        if (results.status === "indexing")
          timer = setTimeout(() => void search(), 250)
      } catch (error) {
        if (current)
          useLibraryStore.setState({
            pending: false,
            error: errorMessage(error),
            results: null,
          })
      }
    }
    timer = setTimeout(() => void search(), filter ? 150 : 0)
    return () => {
      current = false
      clearTimeout(timer)
    }
  }, [roots, rootsStatus, filter, favoritesOnly, tags, revision])

  return (
    <div className="flex max-h-48 shrink-0 flex-col gap-1 overflow-y-auto border-b p-1.5">
      <div className="flex items-center gap-1">
        <Toggle
          size="sm"
          variant="outline"
          pressed={favoritesOnly}
          aria-label="Favorites only"
          onPressedChange={(favoritesOnly) =>
            useLibraryStore.setState({ favoritesOnly })
          }
        >
          Starred
        </Toggle>
        <Select
          value={tag}
          onValueChange={(value) =>
            useLibraryStore.setState({ tags: value ? [value] : [] })
          }
        >
          <SelectTrigger
            size="sm"
            aria-label="Filter by tag"
            className="min-w-0 flex-1"
          >
            <SelectValue placeholder="All tags" />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem value="">All tags</SelectItem>
              {availableTags.map((tag) => (
                <SelectItem key={tag} value={tag}>
                  {tag}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
        <Button
          variant="ghost"
          size="sm"
          onClick={() =>
            void attempt(refreshLibrary(), "Could not refresh the library")
          }
        >
          Refresh
        </Button>
      </div>
      <p className="text-muted-foreground">
        Search paths: <code>kick* AND NOT tight</code>,{" "}
        <code>kick OR snare</code>, <code>"vocal chop"</code>.
      </p>
      {results?.status === "indexing" && (
        <div role="status" className="flex items-center gap-1">
          <Spinner data-icon="inline-start" />
          Indexing {results.indexed.toLocaleString()} files…
          <Button
            variant="ghost"
            size="sm"
            onClick={() =>
              void attempt(
                backend.libraryCancel(results.generation).then(() =>
                  useLibraryStore.setState((s) => ({
                    revision: s.revision + 1,
                  }))
                ),
                "Could not cancel indexing"
              )
            }
          >
            Cancel
          </Button>
        </div>
      )}
      {results?.status === "cancelled" && (
        <p role="status">
          Indexing cancelled. Results cover {results.indexed.toLocaleString()}{" "}
          files. Refresh to continue.
        </p>
      )}
      {results?.truncated && (
        <p role="status">
          Index limit reached. Use smaller folders and refresh; some files were
          not indexed.
        </p>
      )}
      {results?.resultsTruncated && (
        <p role="status">
          Showing the first 500 results. Narrow your search or filter by a tag.
        </p>
      )}
      {results?.limitation && (
        <p role="note" className="text-muted-foreground">
          {results.limitation}
        </p>
      )}
      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
      {(results?.issues ?? []).map((issue) => (
        <Alert key={issue} variant="destructive">
          <AlertDescription className="wrap-anywhere">{issue}</AlertDescription>
        </Alert>
      ))}
    </div>
  )
}

function MetadataEditor({ path }: { path: string }) {
  const revision = useLibraryStore((s) => s.revision)
  const [favorite, setFavorite] = useState(false)
  const [tags, setTags] = useState("")
  const [savedTags, setSavedTags] = useState<string[]>([])
  const [ready, setReady] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    let current = true
    void backend.libraryMetadata(path).then(
      (meta) => {
        if (!current) return
        setFavorite(meta.favorite)
        setTags(meta.tags.join(", "))
        setSavedTags(meta.tags)
        setReady(true)
        setError(null)
      },
      (error) => {
        if (current) setError(errorMessage(error))
      }
    )
    return () => {
      current = false
    }
  }, [path, revision])

  async function save(nextFavorite: boolean) {
    setSaving(true)
    setError(null)
    try {
      const saved = await saveLibraryMetadata(path, {
        favorite: nextFavorite,
        tags: tags
          .split(",")
          .map((t) => t.trim())
          .filter(Boolean),
      })
      setFavorite(saved.favorite)
      setTags(saved.tags.join(", "))
      setSavedTags(saved.tags)
    } catch (error) {
      setError(errorMessage(error))
    } finally {
      setSaving(false)
    }
  }
  return (
    <div className="flex max-h-48 shrink-0 flex-col gap-1 overflow-y-auto border-t p-2">
      <p className="truncate text-muted-foreground" title={path}>
        {path}
      </p>
      <FieldGroup>
        <Field data-invalid={!!error} data-disabled={!ready || saving}>
          <div className="flex items-center justify-between gap-2">
            <FieldLabel htmlFor="browser-file-tags">Tags</FieldLabel>
            <Toggle
              size="sm"
              variant="outline"
              pressed={favorite}
              disabled={!ready || saving}
              aria-label="Star selected file"
              onPressedChange={(value) => void save(value)}
            >
              Star
            </Toggle>
          </div>
          <Input
            id="browser-file-tags"
            value={tags}
            disabled={!ready || saving}
            aria-invalid={!!error}
            maxLength={1054}
            onChange={(e) => setTags(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault()
                void save(favorite)
              }
            }}
          />
          <FieldDescription>
            Comma separated; up to 16 tags of 32 characters. Saved for this
            user.
          </FieldDescription>
        </Field>
      </FieldGroup>
      <div className="flex flex-wrap items-center gap-1">
        {savedTags.map((tag) => (
          <Badge key={tag} variant="secondary">
            {tag}
          </Badge>
        ))}
        <Button
          variant="outline"
          size="sm"
          disabled={!ready || saving}
          onClick={() => void save(favorite)}
        >
          Save tags
        </Button>
      </div>
      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
    </div>
  )
}

export function LibraryMetadataEditor() {
  const selected = useBrowserStore((s) => s.selected)
  return selected && selected.kind !== "folder" ? (
    <MetadataEditor key={selected.path} path={selected.path} />
  ) : null
}
