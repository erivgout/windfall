import { Cancel01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { Component, type ErrorInfo, type ReactNode } from "react"

import { Button } from "@/components/ui/button"
import { Kbd } from "@/components/ui/kbd"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { runAction, useShortcutLabel } from "@/lib/actions"
import { errorMessage } from "@/lib/ipc"
import { cn } from "@/lib/utils"

type BoundaryProps = { name: string; children: ReactNode }
type BoundaryState = { error: unknown }

/** Keeps a panel that throws from taking the rest of the window with it. */
export class PanelBoundary extends Component<BoundaryProps, BoundaryState> {
  state: BoundaryState = { error: null }

  static getDerivedStateFromError(error: unknown): BoundaryState {
    return { error }
  }

  componentDidCatch(error: unknown, info: ErrorInfo) {
    console.error(`The ${this.props.name} panel crashed`, error, info)
  }

  render() {
    if (this.state.error === null) return this.props.children
    return (
      <div
        role="alert"
        className="flex h-full flex-col items-center justify-center gap-2 p-4 text-center"
      >
        <p className="font-medium">The {this.props.name} stopped working.</p>
        <p className="max-w-sm text-muted-foreground">
          {errorMessage(this.state.error)}
        </p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => this.setState({ error: null })}
        >
          Try again
        </Button>
      </div>
    )
  }
}

function HideButton({ title, action }: { title: string; action: string }) {
  const shortcut = useShortcutLabel(action)
  const label = `Hide ${title.toLowerCase()}`
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label={label}
            className="text-muted-foreground"
            onClick={() => void runAction(action)}
          />
        }
      >
        <HugeiconsIcon icon={Cancel01Icon} strokeWidth={2} />
      </TooltipTrigger>
      <TooltipContent side="bottom">
        {label}
        {shortcut && <Kbd>{shortcut}</Kbd>}
      </TooltipContent>
    </Tooltip>
  )
}

type PanelFrameProps = {
  title: string
  /** Registry action that shows and hides this panel. Adds a close button. */
  hideAction?: string
  /** Extra controls for the header, left of the close button. */
  actions?: ReactNode
  className?: string
  children: ReactNode
}

/** The frame every docked panel sits in: a title strip above a scrolling body. */
export function PanelFrame({
  title,
  hideAction,
  actions,
  className,
  children,
}: PanelFrameProps) {
  return (
    <section
      aria-label={title}
      className="flex h-full min-h-0 min-w-0 flex-col bg-background"
    >
      <header className="flex h-7 shrink-0 items-center gap-1 border-b bg-chassis/60 pr-1 pl-2.5">
        <h2 className="truncate text-xs font-medium">{title}</h2>
        <div className="ml-auto flex items-center gap-0.5">
          {actions}
          {hideAction && <HideButton title={title} action={hideAction} />}
        </div>
      </header>
      <div className={cn("min-h-0 flex-1 overflow-auto", className)}>
        <PanelBoundary name={title.toLowerCase()}>{children}</PanelBoundary>
      </div>
    </section>
  )
}
