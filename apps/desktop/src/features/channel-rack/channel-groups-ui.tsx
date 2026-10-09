import { useId, useRef, useState } from "react"

import { ActionButton } from "@/components/action-button"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
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
import { useProjectStore } from "@/lib/store/project"

import {
  assignChannelGroups,
  channelGroups,
  closeChannelGroups,
  groupRequestIsCurrent,
  useChannelGroups,
  validGroupName,
  type GroupRequest,
} from "./channel-groups"
import { useRackStore } from "./rack-store"

const ALL = "all:"
const UNGROUPED = "ungrouped:"
const NEW = "new:"
const groupValue = (name: string) => `group:${name}`

/** Filtering changes the view and selection, never playback or project data. */
export function ChannelGroupFilter() {
  const channels = useProjectStore((state) => state.project.channels)
  const filter = useRackStore((state) => state.groupFilter)
  const setFilter = useRackStore((state) => state.setGroupFilter)
  const groups = channelGroups(channels)
  const count = channels.filter(
    (channel) => filter === null || (channel.group ?? "") === filter
  ).length
  return (
    <div className="flex items-center gap-1">
      <Select
        value={
          filter === null ? ALL : filter === "" ? UNGROUPED : groupValue(filter)
        }
        onValueChange={(value) => {
          if (value === ALL) setFilter(null)
          else if (value === UNGROUPED) setFilter("")
          else if (typeof value === "string" && value.startsWith("group:"))
            setFilter(value.slice(6))
        }}
      >
        <SelectTrigger size="sm" aria-label="Channel group filter">
          <SelectValue>
            {filter === null
              ? `All channels (${channels.length})`
              : filter === ""
                ? `Ungrouped (${count})`
                : `${filter} (${count})`}
          </SelectValue>
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem value={ALL}>
              All channels ({channels.length})
            </SelectItem>
            <SelectItem value={UNGROUPED}>
              Ungrouped ({channels.filter((channel) => !channel.group).length})
            </SelectItem>
            {groups.map((name) => (
              <SelectItem key={name} value={groupValue(name)}>
                {name} (
                {channels.filter((channel) => channel.group === name).length})
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
      <span className="sr-only" aria-live="polite">
        {count} visible channels
      </span>
      <ActionButton action="channelRack.groups" variant="ghost" size="xs">
        Groups…
      </ActionButton>
    </div>
  )
}

function GroupForm({ request }: { request: GroupRequest }) {
  const groups = channelGroups(request.channels)
  const currentGroup =
    request.channels.find((channel) => channel.id === request.selected)
      ?.group ||
    request.filter ||
    ""
  const [target, setTarget] = useState(
    currentGroup ? groupValue(currentGroup) : UNGROUPED
  )
  const [name, setName] = useState("")
  const [selected, setSelected] = useState(
    new Set(request.selected === null ? [] : [request.selected])
  )
  const [pending, setPending] = useState(false)
  const [failure, setFailure] = useState(false)
  const submitting = useRef(false)
  const nameId = useId()
  const listId = useId()
  useProjectStore((state) => state.revision)
  const current = groupRequestIsCurrent(request)
  const group =
    target === NEW ? name.trim() : target === UNGROUPED ? "" : target.slice(6)
  const valid = validGroupName(group) && (target !== NEW || group !== "")
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !submitting.current) closeChannelGroups()
      }}
    >
      <DialogContent
        className="max-h-[85dvh] overflow-y-auto sm:max-w-lg"
        showCloseButton={!pending}
      >
        <DialogHeader>
          <DialogTitle>Channel groups</DialogTitle>
          <DialogDescription>
            Choose channels and assign a group. This changes their organization
            in one undo step and keeps their music and routing.
          </DialogDescription>
        </DialogHeader>
        <form
          className="flex flex-col gap-4"
          onSubmit={async (event) => {
            event.preventDefault()
            if (!current || !valid || selected.size === 0 || submitting.current)
              return
            submitting.current = true
            setPending(true)
            setFailure(false)
            try {
              if (!(await assignChannelGroups(request, [...selected], group)))
                setFailure(true)
            } finally {
              submitting.current = false
              setPending(false)
            }
          }}
        >
          <FieldGroup>
            <Field>
              <FieldLabel>Target group</FieldLabel>
              <Select
                value={target}
                disabled={pending}
                onValueChange={(value) => {
                  if (typeof value === "string") setTarget(value)
                }}
              >
                <SelectTrigger aria-label="Target channel group">
                  <SelectValue>
                    {target === NEW
                      ? "New group…"
                      : target === UNGROUPED
                        ? "Ungrouped"
                        : target.slice(6)}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    <SelectItem value={UNGROUPED}>Ungrouped</SelectItem>
                    {groups.map((group) => (
                      <SelectItem key={group} value={groupValue(group)}>
                        {group}
                      </SelectItem>
                    ))}
                    <SelectItem value={NEW}>New group…</SelectItem>
                  </SelectGroup>
                </SelectContent>
              </Select>
            </Field>
            {target === NEW && (
              <Field data-invalid={!valid}>
                <FieldLabel htmlFor={nameId}>Group name</FieldLabel>
                <Input
                  id={nameId}
                  value={name}
                  disabled={pending}
                  maxLength={128}
                  aria-invalid={!valid}
                  onChange={(event) => setName(event.target.value)}
                />
                <FieldDescription>
                  Use a nonempty name up to 128 UTF-8 bytes. An existing name
                  joins that group.
                </FieldDescription>
              </Field>
            )}
            <FieldSet>
              <FieldLegend>Channels ({selected.size} selected)</FieldLegend>
              <div className="flex flex-wrap gap-1">
                <Button
                  type="button"
                  variant="outline"
                  size="xs"
                  disabled={pending}
                  onClick={() =>
                    setSelected(
                      new Set(request.channels.map((channel) => channel.id))
                    )
                  }
                >
                  Select all
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="xs"
                  disabled={pending}
                  onClick={() =>
                    setSelected(
                      new Set(
                        request.channels
                          .filter(
                            (channel) =>
                              request.filter === null ||
                              (channel.group ?? "") === request.filter
                          )
                          .map((channel) => channel.id)
                      )
                    )
                  }
                >
                  Select visible
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="xs"
                  disabled={pending}
                  onClick={() => setSelected(new Set())}
                >
                  Select none
                </Button>
              </div>
              <FieldGroup className="max-h-64 overflow-y-auto rounded-md border p-3">
                {request.channels.map((channel) => (
                  <Field
                    key={channel.id}
                    orientation="horizontal"
                    data-disabled={pending}
                  >
                    <Checkbox
                      id={`${listId}-${channel.id}`}
                      checked={selected.has(channel.id)}
                      disabled={pending}
                      onCheckedChange={(checked) =>
                        setSelected((previous) => {
                          const next = new Set(previous)
                          if (checked) next.add(channel.id)
                          else next.delete(channel.id)
                          return next
                        })
                      }
                    />
                    <FieldLabel
                      htmlFor={`${listId}-${channel.id}`}
                      className="flex-1"
                    >
                      {channel.name}
                      <span className="text-muted-foreground">
                        {channel.group || "Ungrouped"}
                      </span>
                    </FieldLabel>
                  </Field>
                ))}
                {request.channels.length === 0 && (
                  <FieldDescription>
                    Add a channel to create your first group.
                  </FieldDescription>
                )}
              </FieldGroup>
            </FieldSet>
          </FieldGroup>
          {!current && (
            <Alert variant="destructive">
              <AlertDescription>
                The project changed. Close and reopen Channel groups to see the
                current channels.
              </AlertDescription>
            </Alert>
          )}
          {failure && (
            <Alert variant="destructive">
              <AlertDescription>
                The assignment could not be applied. Close and reopen the
                manager to try again.
              </AlertDescription>
            </Alert>
          )}
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              disabled={pending}
              onClick={closeChannelGroups}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={pending || !current || !valid || selected.size === 0}
            >
              {pending ? "Assigning…" : "Assign selected"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export function ChannelGroupsDialog() {
  const request = useChannelGroups((state) => state.request)
  return request ? <GroupForm key={request.id} request={request} /> : null
}
