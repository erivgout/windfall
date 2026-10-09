import { useId, useRef, useState } from "react"

import type { Notebook, NotebookPage } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
  Field,
  FieldDescription,
  FieldError,
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
import { Textarea } from "@/components/ui/textarea"

import { duplicatePage } from "./duplicate-page"

const MAX_PAGES = 8
const TITLE_BYTES = 128
const BODY_BYTES = 16_384
const blankPage = (): NotebookPage => ({ title: "", body: "" })
const byteLength = (value: string) => new TextEncoder().encode(value).length
const normalize = (pages: NotebookPage[]): Notebook => ({
  pages: pages.map((page) => ({ ...page, title: page.title.trim() })),
})

export function NotebookPanel({
  notebook,
  onSave,
}: {
  notebook?: Notebook
  onSave(notebook: Notebook): Promise<boolean>
}) {
  const id = useId()
  const [pages, setPages] = useState(() => notebook?.pages ?? [])
  const [saved, setSaved] = useState(() => normalize(notebook?.pages ?? []))
  const [selected, setSelected] = useState(0)
  const [saving, setSaving] = useState(false)
  const pending = useRef(false)
  const [error, setError] = useState<string | null>(null)
  // The temporary blank page belongs only to the editor until an edit or add.
  const visiblePages = pages.length ? pages : [blankPage()]
  const page = visiblePages[selected] ?? visiblePages[0]
  const next = normalize(pages)
  const changed = JSON.stringify(next) !== JSON.stringify(saved)
  const titleInvalid = byteLength(page.title.trim()) > TITLE_BYTES
  const bodyInvalid = byteLength(page.body) > BODY_BYTES
  const invalidPage = next.pages.findIndex(
    (item) =>
      byteLength(item.title) > TITLE_BYTES || byteLength(item.body) > BODY_BYTES
  )
  const invalid = invalidPage !== -1 || pages.length > MAX_PAGES
  const items = visiblePages.map((item, index) => ({
    value: index,
    label: `${index + 1}. ${item.title.trim() || "Untitled page"}`,
  }))

  const edit = (patch: Partial<NotebookPage>) => {
    setPages((current) => {
      const base = current.length ? current : [blankPage()]
      return base.map((item, index) =>
        index === selected ? { ...item, ...patch } : item
      )
    })
  }

  const save = async () => {
    if (pending.current || !changed || invalid) return
    pending.current = true
    setSaving(true)
    setError(null)
    try {
      if (await onSave(next)) {
        setSaved(next)
        setPages(next.pages)
      } else {
        setError("Could not save notebook.")
      }
    } catch {
      setError("Could not save notebook.")
    } finally {
      pending.current = false
      setSaving(false)
    }
  }

  return (
    <form
      aria-label="Song notebook"
      aria-busy={saving}
      onSubmit={(event) => {
        event.preventDefault()
        void save()
      }}
      onKeyDown={(event) => event.stopPropagation()}
    >
      <FieldGroup>
        <Field data-disabled={saving}>
          <FieldLabel htmlFor={`${id}-page`}>Page</FieldLabel>
          <Select
            items={items}
            value={selected}
            disabled={saving}
            onValueChange={(value: number | null) => {
              if (value !== null) setSelected(value)
            }}
          >
            <SelectTrigger id={`${id}-page`} className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {items.map((item) => (
                  <SelectItem key={item.value} value={item.value}>
                    {item.label}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
          <FieldDescription>
            {pages.length
              ? `${pages.length} of ${MAX_PAGES} pages`
              : "No saved pages. Start with this blank page."}
          </FieldDescription>
          <div className="flex gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={saving || visiblePages.length >= MAX_PAGES}
              onClick={() => {
                if (visiblePages.length >= MAX_PAGES) return
                setPages([...visiblePages, blankPage()])
                setSelected(visiblePages.length)
              }}
            >
              Add page
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={saving || !pages.length}
              onClick={() => {
                setPages(pages.filter((_, index) => index !== selected))
                setSelected(Math.max(0, selected - 1))
              }}
            >
              Remove page
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={saving || !pages.length || pages.length >= MAX_PAGES}
              onClick={() => {
                const duplicated = duplicatePage(pages, selected, MAX_PAGES)
                if (!duplicated) return
                setPages(duplicated)
                setSelected(selected + 1)
              }}
            >
              Duplicate page
            </Button>
          </div>
        </Field>
        <Field data-disabled={saving} data-invalid={titleInvalid}>
          <FieldLabel htmlFor={`${id}-title`}>Page title</FieldLabel>
          <Input
            id={`${id}-title`}
            value={page.title}
            disabled={saving}
            aria-invalid={titleInvalid}
            aria-describedby={titleInvalid ? `${id}-title-error` : undefined}
            onChange={(event) => edit({ title: event.target.value })}
          />
          {titleInvalid && (
            <FieldError id={`${id}-title-error`}>
              Page title must be at most 128 bytes.
            </FieldError>
          )}
        </Field>
        <Field data-disabled={saving} data-invalid={bodyInvalid}>
          <FieldLabel htmlFor={`${id}-body`}>Page body</FieldLabel>
          <Textarea
            id={`${id}-body`}
            className="min-h-48"
            value={page.body}
            disabled={saving}
            aria-invalid={bodyInvalid}
            aria-describedby={bodyInvalid ? `${id}-body-error` : undefined}
            onChange={(event) => edit({ body: event.target.value })}
          />
          {bodyInvalid && (
            <FieldError id={`${id}-body-error`}>
              Page body must be at most 16,384 bytes.
            </FieldError>
          )}
          <FieldDescription>Plain text saved with the song.</FieldDescription>
        </Field>
        {invalidPage !== -1 && invalidPage !== selected && (
          <FieldError role="alert">
            Page {invalidPage + 1} exceeds a text limit.
          </FieldError>
        )}
        {pages.length > MAX_PAGES && (
          <FieldError role="alert">
            A notebook can have at most 8 pages.
          </FieldError>
        )}
        {error && <FieldError role="alert">{error}</FieldError>}
        <Button
          type="submit"
          size="sm"
          disabled={saving || !changed || invalid}
        >
          {saving ? "Saving…" : "Save notebook"}
        </Button>
      </FieldGroup>
    </form>
  )
}
