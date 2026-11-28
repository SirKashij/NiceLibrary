// Reads and rewrites a Windows .ico whose images are PNGs, as `tauri icon` writes it.
// The app icon swaps in a simplified image for the smallest size (scripts/build-icons.ts).

const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
const HEADER = 6;
const ENTRY = 16;

export interface IcoImage {
  /** Width and height in pixels; the .ico stores 256 as 0. */
  size: number;
  image: Buffer;
}

/** Every image in `ico`, in the order the file lists them. */
export function icoImages(ico: Buffer): IcoImage[] {
  if (ico.readUInt16LE(2) !== 1) throw new Error('The file is not an .ico.');
  return Array.from({ length: ico.readUInt16LE(4) }, (_, i) => {
    const entry = HEADER + ENTRY * i;
    const offset = ico.readUInt32LE(entry + 12);
    return {
      size: ico.readUInt8(entry) || 256,
      image: Buffer.from(ico.subarray(offset, offset + ico.readUInt32LE(entry + 8))),
    };
  });
}

/** `ico` with the image of `size` px replaced by the 32-bit PNG `png`. */
export function withIcoImage(ico: Buffer, size: number, png: Buffer): Buffer {
  if (!png.subarray(0, PNG_SIGNATURE.length).equals(PNG_SIGNATURE)) throw new Error('The image is not a PNG.');
  const images = icoImages(ico);
  if (!images.some((i) => i.size === size)) throw new Error(`The .ico holds no ${size} px image.`);
  return writeIco(images.map((i) => (i.size === size ? { size, image: png } : i)));
}

function writeIco(images: IcoImage[]): Buffer {
  const header = Buffer.alloc(HEADER);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);
  let offset = HEADER + ENTRY * images.length;
  const entries = images.map(({ size, image }) => {
    const entry = Buffer.alloc(ENTRY);
    entry.writeUInt8(size % 256, 0);
    entry.writeUInt8(size % 256, 1);
    entry.writeUInt16LE(1, 4); // colour planes
    entry.writeUInt16LE(32, 6); // bits per pixel
    entry.writeUInt32LE(image.length, 8);
    entry.writeUInt32LE(offset, 12);
    offset += image.length;
    return entry;
  });
  return Buffer.concat([header, ...entries, ...images.map((i) => i.image)]);
}
