// Terminal glyphs shown in a control (the statusline editor's bar styles): text in the terminal
// font, with the colors a sample needs when its shape alone does not tell it (a solid bar).

export interface GlyphRun {
  text: string;
  /** A CSS color, or `null` for the control's text color. */
  color: string | null;
  /** A CSS background, or `null` for none. */
  background: string | null;
}
