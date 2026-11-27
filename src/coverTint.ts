// A Detail Page's Tint: background and accent derived from the dominant hue of the Game's Cover
// (ADR-0007), checked against the shown theme's colours from src/styles/tokens.css. Where the
// contrast rules of contrast.test.ts cannot be kept, there is no Tint and the page stays neutral.
import { useEffect, useState } from 'react';
import type { CoverHue, Operations } from './operations';

/** The colours of the shown theme a Tint is checked against, as six-digit hex values. */
export interface ThemeColours {
  bg: string;
  surface: string;
  surfaceRaised: string;
  text: string;
  textMuted: string;
  controlBorder: string;
}

/** The colours a Detail Page takes from its Cover, as six-digit hex values. */
export interface CoverTint {
  /** The page background: the theme's background, tinted with the Cover hue. */
  background: string;
  /** The main action, progress and highlights. */
  accent: string;
  /** Labels on the accent. */
  accentInk: string;
}

/** WCAG 2.2: text 4.5:1 (1.4.3), the parts of controls 3:1 (1.4.11). */
const TEXT = 4.5;
const CONTROL = 3;

/** How strongly the background takes the Cover hue, from a grey Cover to a vivid one. */
const BACKGROUND_SATURATION = { base: 0.12, vivid: 0.28 };
/** How vivid the accent is, from a grey Cover to a vivid one. */
const ACCENT_SATURATION = { base: 0.35, vivid: 0.5 };
/** The accent's preferred lightness on a dark and on a light background. */
const ACCENT_LIGHTNESS = { onDark: 0.72, onLight: 0.36 };

type Rgb = [number, number, number];

function parse(hex: string): Rgb | null {
  if (!/^#[0-9a-f]{6}$/i.test(hex)) return null;
  return [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255) as Rgb;
}

function format(rgb: Rgb): string {
  return `#${rgb.map((c) => Math.round(c * 255).toString(16).padStart(2, '0')).join('')}`;
}

/** The colour of `hue` (degrees), `saturation` and `lightness` (0 to 1) in HSL, rounded to hex. */
function hsl(hue: number, saturation: number, lightness: number): string {
  const a = saturation * Math.min(lightness, 1 - lightness);
  const channel = (n: number) => {
    const k = (n + hue / 30) % 12;
    return lightness - a * Math.max(-1, Math.min(k - 3, 9 - k, 1));
  };
  return format([channel(0), channel(8), channel(4)]);
}

/** HSL lightness of a colour. */
function lightnessOf([r, g, b]: Rgb): number {
  return (Math.max(r, g, b) + Math.min(r, g, b)) / 2;
}

/** Relative luminance (WCAG 2.2). */
function luminance(rgb: Rgb): number {
  const [r, g, b] = rgb.map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: Rgb, b: Rgb): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

/**
 * The Tint for a Cover of `cover`'s hue in a theme of `theme`'s colours: a background of the
 * theme's lightness in the Cover hue, and an accent in the Cover hue as close to its preferred
 * lightness as the contrast rules allow. `null` when they cannot be kept, or `theme` is unreadable.
 */
export function coverTint(cover: CoverHue, theme: ThemeColours): CoverTint | null {
  const [bg, surface, surfaceRaised, text, textMuted, controlBorder] = [
    theme.bg,
    theme.surface,
    theme.surfaceRaised,
    theme.text,
    theme.textMuted,
    theme.controlBorder,
  ].map(parse);
  if (!bg || !surface || !surfaceRaised || !text || !textMuted || !controlBorder) return null;
  const vividness = Math.min(1, Math.max(0, cover.saturation));

  const background = hsl(
    cover.hue,
    BACKGROUND_SATURATION.base + BACKGROUND_SATURATION.vivid * vividness,
    lightnessOf(bg),
  );
  const tinted = parse(background)!;
  const readable =
    contrast(text, tinted) >= TEXT && contrast(textMuted, tinted) >= TEXT && contrast(controlBorder, tinted) >= CONTROL;
  if (!readable) return null;

  const preferred = luminance(bg) < 0.5 ? ACCENT_LIGHTNESS.onDark : ACCENT_LIGHTNESS.onLight;
  const saturation = ACCENT_SATURATION.base + ACCENT_SATURATION.vivid * vividness;
  const fits = (accent: Rgb) =>
    contrast(tinted, accent) >= TEXT && [tinted, bg, surface, surfaceRaised].every((s) => contrast(accent, s) >= CONTROL);
  // Lightness steps outwards from the preferred one; the first accent that fits is the closest.
  for (let step = 0; step <= 200; step++) {
    const offset = (step % 2 === 0 ? 1 : -1) * Math.ceil(step / 2) * 0.005;
    const lightness = preferred + offset;
    if (lightness < 0 || lightness > 1) continue;
    const accent = hsl(cover.hue, saturation, lightness);
    if (fits(parse(accent)!)) return { background, accent, accentInk: background };
  }
  return null;
}

/** The shown theme's colours, as the token file sets them on the document element. */
function shownThemeColours(): ThemeColours {
  const style = getComputedStyle(document.documentElement);
  const read = (name: string) => style.getPropertyValue(name).trim();
  return {
    bg: read('--color-bg'),
    surface: read('--color-surface'),
    surfaceRaised: read('--color-surface-raised'),
    text: read('--color-text'),
    textMuted: read('--color-text-muted'),
    controlBorder: read('--color-control-border'),
  };
}

/**
 * The Tint of the Game `gameId`'s Detail Page in the shown theme, following a change of theme;
 * `null`, the neutral look, while the Cover hue is asked, when the Game has none, when it cannot
 * be read, or when no Tint keeps the contrast rules.
 */
export function useCoverTint(operations: Operations, gameId: string, coverChanges = 0): CoverTint | null {
  const [hue, setHue] = useState<CoverHue | null>(null);
  // The theme is `data-theme` on the document element (src/theme.tsx).
  const [theme, setTheme] = useState(() => document.documentElement.dataset.theme);

  useEffect(() => {
    let active = true;
    setHue(null);
    operations.coverHue(gameId).then(
      (answer) => active && setHue(answer),
      () => {},
    );
    return () => {
      active = false;
    };
  }, [operations, gameId, coverChanges]);

  useEffect(() => {
    const observer = new MutationObserver(() => setTheme(document.documentElement.dataset.theme));
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    return () => observer.disconnect();
  }, []);

  const [tint, setTint] = useState<CoverTint | null>(null);
  useEffect(() => setTint(hue && coverTint(hue, shownThemeColours())), [hue, theme]);
  return tint;
}
