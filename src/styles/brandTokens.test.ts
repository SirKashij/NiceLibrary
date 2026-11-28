// The brand mark (#92) looks the same in both themes: its tokens are defined once, outside the themes.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const TOKENS = readFileSync(join(__dirname, 'tokens.css'), 'utf8');
const BRAND = ['--brand-mark-size', '--brand-plate', '--brand-gradient-start', '--brand-gradient-end'];

function definitions(name: string): number {
  return TOKENS.split('\n').filter((line) => line.trim().startsWith(`${name}:`)).length;
}

describe('brand tokens', () => {
  it('define the mark size, the plate and both gradient stops exactly once', () => {
    expect(BRAND.map((name) => [name, definitions(name)])).toEqual(BRAND.map((name) => [name, 1]));
  });

  it('keep the gradient of the app icon', () => {
    expect(TOKENS).toMatch(/--brand-gradient-start:\s*#e4ff7a;/);
    expect(TOKENS).toMatch(/--brand-gradient-end:\s*#7fd66b;/);
    expect(TOKENS).toMatch(/--brand-plate:\s*#121314;/);
    expect(TOKENS).toMatch(/--brand-mark-size:\s*22px;/);
  });

  it('no longer size the mark as a rectangle', () => {
    expect(TOKENS).not.toContain('--brand-mark-width');
    expect(TOKENS).not.toContain('--brand-mark-height');
  });
});
