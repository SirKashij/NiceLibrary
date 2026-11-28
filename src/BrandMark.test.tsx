import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { BrandMark } from './BrandMark';

describe('the brand mark', () => {
  it('is decoration that assistive technology skips', () => {
    const { container } = render(<BrandMark />);
    const svg = container.querySelector('svg');
    expect(svg).not.toBeNull();
    expect(svg).toHaveAttribute('aria-hidden', 'true');
    expect(svg).toHaveAttribute('focusable', 'false');
  });

  it('draws the same monogram as the app icon', () => {
    const icon = new DOMParser().parseFromString(
      readFileSync(join(__dirname, '..', 'src-tauri', 'icon.svg'), 'utf8'),
      'image/svg+xml',
    );
    const { container } = render(<BrandMark />);
    const shape = (svg: ParentNode) => {
      const path = svg.querySelector('path');
      const plate = svg.querySelector('rect');
      return [path?.getAttribute('d'), path?.getAttribute('stroke-width'), plate?.getAttribute('rx')];
    };
    expect(shape(container)).toEqual(shape(icon));
  });

  it('strokes its monogram with its own gradient, even when it is shown twice', () => {
    const { container } = render(
      <>
        <BrandMark />
        <BrandMark />
      </>,
    );
    const ids = [...container.querySelectorAll('linearGradient')].map((g) => g.id);
    expect(ids).toHaveLength(2);
    expect(new Set(ids).size).toBe(2);
    const strokes = [...container.querySelectorAll('path')].map((p) => p.getAttribute('stroke'));
    expect(strokes).toEqual(ids.map((id) => `url(#${id})`));
  });
});
