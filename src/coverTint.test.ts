// A Detail Page's Tint, derived from the Cover hue, keeps the contrast rules of both themes
// (WCAG 2.2: 4.5:1 for text, 3:1 for the parts of controls), or there is none and the page stays neutral.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { coverTint, type ThemeColours } from './coverTint';

const TOKENS = readFileSync(join(__dirname, 'styles', 'tokens.css'), 'utf8');

/** The six-digit hex value of `name` in the tokens.css rule for `selector`. */
function token(selector: string, name: string): string {
  const start = TOKENS.indexOf(`${selector} {`);
  const rule = TOKENS.slice(start, TOKENS.indexOf('}', start));
  const value = new RegExp(`${name}\\s*:\\s*(#[0-9a-f]{6})\\s*;`, 'i').exec(rule)?.[1];
  if (start < 0 || !value) throw new Error(`no ${name} in ${selector}`);
  return value;
}

function themeColours(selector: string): ThemeColours {
  return {
    bg: token(selector, '--color-bg'),
    surface: token(selector, '--color-surface'),
    surfaceRaised: token(selector, '--color-surface-raised'),
    text: token(selector, '--color-text'),
    textMuted: token(selector, '--color-text-muted'),
    controlBorder: token(selector, '--color-control-border'),
  };
}

const THEMES = { dark: themeColours(':root'), light: themeColours(":root[data-theme='light']") };

function rgb(hex: string): [number, number, number] {
  expect(hex).toMatch(/^#[0-9a-f]{6}$/);
  return [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255) as [number, number, number];
}

/** Relative luminance (WCAG 2.2). */
function luminance(hex: string): number {
  const [r, g, b] = rgb(hex).map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

/** The hue of a colour in degrees. */
function hueOf(hex: string): number {
  const [r, g, b] = rgb(hex);
  const max = Math.max(r, g, b);
  const delta = max - Math.min(r, g, b);
  const hue = max === r ? (g - b) / delta : max === g ? (b - r) / delta + 2 : (r - g) / delta + 4;
  return (hue * 60 + 360) % 360;
}

function hueDistance(a: number, b: number): number {
  const d = Math.abs(a - b) % 360;
  return Math.min(d, 360 - d);
}

const HUES = Array.from({ length: 24 }, (_, i) => i * 15);
const SATURATIONS = [0.2, 0.6, 1];

describe.each(['dark', 'light'] as const)('in the %s theme', (name) => {
  const theme = THEMES[name];

  it.each(SATURATIONS)('every Cover hue of saturation %s gives a Tint that keeps the contrast rules', (saturation) => {
    for (const hue of HUES) {
      const tint = coverTint({ hue, saturation }, theme);
      if (tint === null) throw new Error(`no Tint for hue ${hue}`);
      const ratios = {
        hue,
        text: contrast(theme.text, tint.background) >= 4.5,
        mutedText: contrast(theme.textMuted, tint.background) >= 4.5,
        controlBorders: contrast(theme.controlBorder, tint.background) >= 3,
        accentOnTint: contrast(tint.accent, tint.background) >= 3,
        accentOnSurfaces: [theme.bg, theme.surface, theme.surfaceRaised].every((s) => contrast(tint.accent, s) >= 3),
        labelOnAccent: contrast(tint.accentInk, tint.accent) >= 4.5,
      };
      expect(ratios).toEqual({
        hue,
        text: true,
        mutedText: true,
        controlBorders: true,
        accentOnTint: true,
        accentOnSurfaces: true,
        labelOnAccent: true,
      });
    }
  });

  it('takes the Cover hue into the background and the accent', () => {
    for (const hue of HUES) {
      const tint = coverTint({ hue, saturation: 0.8 }, theme)!;
      expect(tint.background).not.toBe(theme.bg);
      expect({ hue, background: hueDistance(hueOf(tint.background), hue) <= 12 }).toEqual({ hue, background: true });
      expect({ hue, accent: hueDistance(hueOf(tint.accent), hue) <= 6 }).toEqual({ hue, accent: true });
    }
  });
});

it('gives no Tint where no accent could stand out, so the page stays neutral', () => {
  // A mid-grey page: nothing reaches 3:1 against it and also carries a 4.5:1 label.
  const grey: ThemeColours = {
    bg: '#777777',
    surface: '#777777',
    surfaceRaised: '#777777',
    text: '#000000',
    textMuted: '#000000',
    controlBorder: '#000000',
  };

  expect(coverTint({ hue: 200, saturation: 0.8 }, grey)).toBeNull();
});

it('gives no Tint where the theme colours cannot be read', () => {
  expect(coverTint({ hue: 200, saturation: 0.8 }, { ...THEMES.dark, text: '' })).toBeNull();
});
