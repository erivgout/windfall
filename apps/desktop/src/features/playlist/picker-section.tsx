import type { ReactNode } from "react"

/** One list of the picker: a heading, an optional button, and its rows. */
export function PickerSection({
  title,
  action,
  children,
}: {
  title: string
  action?: ReactNode
  children: ReactNode
}) {
  return (
    <section aria-label={title} className="flex shrink-0 flex-col">
      <div className="sticky top-0 z-10 flex h-6 shrink-0 items-center border-b bg-chassis pr-0.5 pl-2 text-muted-foreground">
        <h3 className="flex-1 text-[0.6875rem] font-medium">{title}</h3>
        {action}
      </div>
      {children}
    </section>
  )
}

/** What a list says while it has nothing in it. */
export function PickerEmpty({ children }: { children: ReactNode }) {
  return (
    <p className="px-2 py-1.5 text-[0.6875rem] leading-snug text-balance text-muted-foreground">
      {children}
    </p>
  )
}

/** The classes every row of the picker shares. */
export const PICKER_ROW =
  "group flex h-7 w-full shrink-0 items-center gap-2 border-l-2 border-transparent pr-2 pl-1.5 text-left outline-none hover:bg-accent/60 focus-visible:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset aria-pressed:border-brand aria-pressed:bg-accent aria-pressed:text-accent-foreground"
