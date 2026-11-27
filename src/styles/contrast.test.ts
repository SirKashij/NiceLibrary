// Both themes are defined in tokens.css alone, and their text and controls meet WCAG 2.2 contrast:
// 4.5:1 for text (1.4.3), 3:1 for the parts of controls that identify them and their states (1.4.11).
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const TOKENS = readFileSync(join(__dirname, 'tokens.css'), 'utf8');

/** The custom properties declared in the rule for `selector`, as written. */
function declarations(selector: string): Record<string, string> {
  const escaped = selector.replace(/[[\]'()]/g, (c) => `\\${c}`);
  const block = new RegExp(`(^|\\n)${escaped}\\s*\\{([^}]*)\\}`).exec(TOKENS);
  if (!block) throw new Error(`tokens.css has no rule for ${selector}`);
  const body = block[2].replace(/\/\*[\s\S]*?\*\//g, '');
  return Object.fromEntries([...body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map((m) => [m[1], m[2].trim()]));
}

/** The colour tokens of a theme: the base rule, with the theme's rule on top. */
function theme(name: 'dark' | 'light'): Record<string, string> {
  const base = declarations(':root');
  return name === 'dark' ? base : { ...base, ...declarations(":root[data-theme='light']") };
}

/** A token's colour as a hex value, following `var(--other)`. */
function colour(tokens: Record<string, string>, name: string): string {
  const value = tokens[name];
  if (value === undefined) throw new Error(`no token ${name}`);
  const reference = /^var\((--[\w-]+)\)$/.exec(value);
  if (reference) return colour(tokens, reference[1]);
  if (!/^#[0-9a-f]{6}$/i.test(value)) throw new Error(`${name} is not a six-digit hex colour: ${value}`);
  return value;
}

/** Relative luminance (WCAG 2.2). */
function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

const SURFACES = ['--color-bg', '--color-surface', '--color-surface-raised'];

/** Foreground and background pairs the interface puts together, with the contrast each needs. */
const PAIRS: { what: string; fg: string; bg: string[]; min: number }[] = [
  { what: 'text', fg: '--color-text', bg: SURFACES, min: 4.5 },
  { what: 'muted text, disabled buttons, placeholders', fg: '--color-text-muted', bg: SURFACES, min: 4.5 },
  { what: 'a complete Progress', fg: '--color-complete', bg: SURFACES, min: 4.5 },
  { what: 'text on the accent (Play)', fg: '--color-accent-ink', bg: ['--color-accent'], min: 4.5 },
  { what: 'borders of buttons and fields', fg: '--color-control-border', bg: SURFACES, min: 3 },
  { what: 'focus ring, radio buttons, current navigation', fg: '--color-accent', bg: SURFACES, min: 3 },
];

describe.each(['dark', 'light'] as const)('the %s theme', (name) => {
  const tokens = theme(name);

  it.each(PAIRS)('has enough contrast for $what', ({ fg, bg, min }) => {
    for (const background of bg) {
      const ratio = contrast(colour(tokens, fg), colour(tokens, background));
      expect({ fg, background, ratio: Math.round(ratio * 100) / 100 }).toEqual({
        fg,
        background,
        ratio: expect.toSatisfy((r: number) => r >= min),
      });
    }
  });

  it('tells the browser which scheme it is, for scrollbars and native controls', () => {
    const scheme = /color-scheme\s*:\s*(\w+)/.exec(
      name === 'dark' ? TOKENS.split(":root[data-theme='light']")[0] : TOKENS.split(":root[data-theme='light']")[1],
    );
    expect(scheme?.[1]).toBe(name);
  });
});

describe('the light theme', () => {
  it('gives every colour of the dark theme its own value', () => {
    const dark = Object.keys(declarations(':root')).filter((name) => name.startsWith('--color-'));
    const light = declarations(":root[data-theme='light']");
    expect(dark.filter((name) => !(name in light))).toEqual([]);
  });
});
