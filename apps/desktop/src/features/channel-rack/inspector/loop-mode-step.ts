const LOOP_MODES = ["off", "forward", "pingPong"] as const

export function nextLoopMode(
  mode: string = "off",
  direction: "previous" | "next"
): "off" | "forward" | "pingPong" | null {
  const index = LOOP_MODES.findIndex((item) => item === mode)
  if (index === -1) return null

  return LOOP_MODES[index + (direction === "previous" ? -1 : 1)] ?? null
}
