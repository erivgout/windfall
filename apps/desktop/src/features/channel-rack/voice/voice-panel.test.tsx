import { StrictMode } from "react"
import { cleanup, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, expect, it, vi } from "vitest"

import {
  createDefaultChannelVoiceSettings,
  sanitizeChannelVoiceSettings,
} from "./settings"
import { ChannelVoicePanel } from "./voice-panel"

afterEach(cleanup)

it("reports one complete draft per edit in Strict Mode, preserving earlier edits", () => {
  const onChange = vi.fn()
  render(
    <StrictMode>
      <ChannelVoicePanel onChange={onChange} />
    </StrictMode>
  )
  expect(onChange).not.toHaveBeenCalled()
  fireEvent.click(screen.getByRole("switch", { name: "Enable note echo" }))
  expect(onChange).toHaveBeenCalledTimes(1)
  expect(onChange.mock.calls[0][0].echo.enabled).toBe(true)
  fireEvent.click(screen.getByRole("switch", { name: "Mono legato" }))
  expect(onChange).toHaveBeenCalledTimes(2)
  expect(onChange.mock.calls[1][0].echo.enabled).toBe(true)
  expect(onChange.mock.calls[1][0].polyphony.monoLegato).toBe(true)
})

it("edits tempo echo timing and a pitch envelope without mutating initial settings", () => {
  const initial = createDefaultChannelVoiceSettings()
  const onChange = vi.fn()
  render(<ChannelVoicePanel initialSettings={initial} onChange={onChange} />)
  fireEvent.click(screen.getByRole("button", { name: "Tempo" }))
  expect(onChange.mock.lastCall?.[0].echo.time).toEqual({
    unit: "division",
    division: "eighth",
  })
  fireEvent.click(screen.getByRole("switch", { name: "Enable pitch envelope" }))
  expect(onChange.mock.lastCall?.[0].envelopes.pitch.enabled).toBe(true)
  fireEvent.keyDown(
    screen.getByRole("slider", { name: "Pitch envelope depth" }),
    { key: "ArrowUp" }
  )
  expect(onChange.mock.lastCall?.[0].envelopes.pitch.depth).toBeGreaterThan(0)
  expect(initial).toEqual(createDefaultChannelVoiceSettings())
})

it("creates independent defaults and sanitizes finite limits and integer fields", () => {
  const value = createDefaultChannelVoiceSettings()
  value.envelopes.filter.depth = 20
  expect(value.envelopes.pitch.depth).toBe(0)
  value.echo.feedback = 2
  value.echo.repeats = 99
  value.echo.pitchSemitones = -200
  value.polyphony.maxVoices = 0
  value.polyphony.portamentoMs = Number.NaN
  value.arpeggiator.rangeOctaves = 2.7
  const clean = sanitizeChannelVoiceSettings(value)
  expect(clean.echo).toMatchObject({
    feedback: 0.95,
    repeats: 8,
    pitchSemitones: -48,
  })
  expect(clean.polyphony).toMatchObject({ maxVoices: 1, portamentoMs: 0 })
  expect(clean.arpeggiator.rangeOctaves).toBe(2)
  expect(clean.envelopes.filter.depth).toBe(8)
})
