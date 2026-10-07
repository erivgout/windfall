/** How much of the preview waveform's color is the brand color. */
export const PREVIEW_BRAND_SHARE = 0.82

/**
 * The color of the waveform in the preview's window. The window is the
 * dark display in both themes. Drawn in the display's own near-white, a
 * loud sound that fills the window was the color of the light theme around
 * it: an empty box with dark bars at its edges. Mostly brand color, it is
 * told apart from the window and from what is around the window.
 */
export const PREVIEW_WAVE = `color-mix(in oklch, var(--wf-brand) ${PREVIEW_BRAND_SHARE * 100}%, var(--wf-display-foreground))`
