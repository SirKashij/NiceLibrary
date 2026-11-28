import { describe, expect, it } from 'vitest';
import { icoImages, withIcoImage } from './ico.ts';

const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const png = (marker: number, length: number) => Buffer.from([...PNG_SIGNATURE, ...Array(length).fill(marker)]);

/** An .ico as `tauri icon` writes it: a header, one 16-byte entry per size, then the PNG images. */
function ico(images: Array<[size: number, image: Buffer]>): Buffer {
  const header = Buffer.from([0, 0, 1, 0, images.length, 0]);
  let offset = 6 + 16 * images.length;
  const entries = images.map(([size, image]) => {
    const entry = Buffer.alloc(16);
    entry.writeUInt8(size % 256, 0);
    entry.writeUInt8(size % 256, 1);
    entry.writeUInt16LE(1, 4);
    entry.writeUInt16LE(32, 6);
    entry.writeUInt32LE(image.length, 8);
    entry.writeUInt32LE(offset, 12);
    offset += image.length;
    return entry;
  });
  return Buffer.concat([header, ...entries, ...images.map(([, image]) => image)]);
}

describe('withIcoImage', () => {
  const original = ico([
    [16, png(1, 4)],
    [32, png(2, 6)],
    [256, png(3, 8)],
  ]);

  it('replaces the image of one size and keeps the others', () => {
    const changed = withIcoImage(original, 16, png(9, 20));
    expect(icoImages(changed)).toEqual([
      { size: 16, image: png(9, 20) },
      { size: 32, image: png(2, 6) },
      { size: 256, image: png(3, 8) },
    ]);
  });

  it('refuses a size the icon does not hold', () => {
    expect(() => withIcoImage(original, 24, png(9, 4))).toThrow('no 24 px image');
  });

  it('refuses an image that is not a PNG', () => {
    expect(() => withIcoImage(original, 16, Buffer.from('not a png'))).toThrow('not a PNG');
  });
});
