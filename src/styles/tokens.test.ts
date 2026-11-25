// ADR-0007: raw colour, type, radius and spacing values live in one token file. Components use tokens only.
import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';

const SRC = join(__dirname, '..');
const TOKEN_FILE = join(__dirname, 'tokens.css');

function filesUnder(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory() ? filesUnder(join(dir, entry.name)) : [join(dir, entry.name)],
  );
}

const RAW_VALUE = [
  /#[0-9a-f]{3,8}\b/i, // hex colour
  /\b(rgba?|hsla?|oklch|color-mix)\(/i, // colour functions
  /\b\d*\.?\d+(px|rem|em)\b/, // lengths
  /font-family\s*:\s*(?!\s|var\()/i, // font names outside var(...)
];

describe('design tokens', () => {
  it('keep every raw value in tokens.css', () => {
    const offenders = filesUnder(SRC)
      .filter((file) => /\.(css|tsx)$/.test(file) && file !== TOKEN_FILE)
      .flatMap((file) =>
        readFileSync(file, 'utf8')
          .split('\n')
          .map((line, i) => ({ line, at: `${relative(SRC, file)}:${i + 1}` }))
          .filter(({ line }) => RAW_VALUE.some((pattern) => pattern.test(line)))
          .map(({ at, line }) => `${at}  ${line.trim()}`),
      );

    expect(offenders).toEqual([]);
  });

  it('name both shipped typefaces', () => {
    const tokens = readFileSync(TOKEN_FILE, 'utf8');
    expect(tokens).toContain('Bricolage Grotesque Variable');
    expect(tokens).toContain('Instrument Sans Variable');
  });
});
