import type { ComponentProps, ReactNode } from "react"

import { Button } from "@/components/ui/button"
import { Kbd } from "@/components/ui/kbd"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { useHint } from "@/lib/store/hint"
import {
  runAction,
  useAction,
  useActionEnabled,
  useShortcutLabel,
} from "@/lib/actions"

type ActionButtonProps = Omit<
  ComponentProps<typeof Button>,
  "onClick" | "children"
> & {
  /** Id of the registry action the button runs. */
  action: string
  /** Usually an icon. Leave out to show the action's title as text. */
  children?: ReactNode
  tooltipSide?: "top" | "bottom" | "left" | "right"
}

/**
 * A button for a registry action. Its label, tooltip, shortcut hint and
 * disabled state all come from the registry, so a button and the menu item
 * for the same action never disagree.
 */
export function ActionButton({
  action: id,
  children,
  tooltipSide = "bottom",
  disabled,
  ...props
}: ActionButtonProps) {
  const action = useAction(id)
  const enabled = useActionEnabled(id)
  const shortcut = useShortcutLabel(id)
  const title = action?.title.replace(/…$/, "") ?? id
  const hint = useHint(shortcut ? `${title} (${shortcut})` : title)

  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <Button
            aria-label={children ? title : undefined}
            disabled={disabled || !enabled}
            onClick={() => void runAction(id)}
            {...hint}
            {...props}
          />
        }
      >
        {children ?? title}
      </TooltipTrigger>
      <TooltipContent side={tooltipSide}>
        {title}
        {shortcut && <Kbd>{shortcut}</Kbd>}
      </TooltipContent>
    </Tooltip>
  )
}
