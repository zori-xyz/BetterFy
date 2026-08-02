import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { inflateSync } from "node:zlib";

const root = resolve(import.meta.dirname, "..");

function decodePng(buffer) {
  const signature = buffer.subarray(0, 8).toString("hex");
  if (signature !== "89504e470d0a1a0a") throw new Error("Invalid PNG signature");

  let width;
  let height;
  let bitDepth;
  let colorType;
  const idat = [];

  for (let offset = 8; offset < buffer.length; ) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.subarray(offset + 4, offset + 8).toString("ascii");
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
    } else if (type === "IDAT") {
      idat.push(data);
    } else if (type === "IEND") {
      break;
    }
    offset += length + 12;
  }

  if (bitDepth !== 8 || colorType !== 6) {
    throw new Error(`Expected an 8-bit RGBA PNG, received depth=${bitDepth}, type=${colorType}`);
  }

  const bytesPerPixel = 4;
  const stride = width * bytesPerPixel;
  const compressed = Buffer.concat(idat);
  const filtered = inflateSync(compressed);
  const pixels = Buffer.alloc(stride * height);

  for (let y = 0; y < height; y += 1) {
    const rowStart = y * (stride + 1);
    const filter = filtered[rowStart];
    for (let x = 0; x < stride; x += 1) {
      const raw = filtered[rowStart + 1 + x];
      const left = x >= bytesPerPixel ? pixels[y * stride + x - bytesPerPixel] : 0;
      const up = y > 0 ? pixels[(y - 1) * stride + x] : 0;
      const upperLeft = y > 0 && x >= bytesPerPixel
        ? pixels[(y - 1) * stride + x - bytesPerPixel]
        : 0;

      let value;
      if (filter === 0) value = raw;
      else if (filter === 1) value = raw + left;
      else if (filter === 2) value = raw + up;
      else if (filter === 3) value = raw + Math.floor((left + up) / 2);
      else if (filter === 4) {
        const prediction = left + up - upperLeft;
        const leftDistance = Math.abs(prediction - left);
        const upDistance = Math.abs(prediction - up);
        const upperLeftDistance = Math.abs(prediction - upperLeft);
        const predictor = leftDistance <= upDistance && leftDistance <= upperLeftDistance
          ? left
          : upDistance <= upperLeftDistance ? up : upperLeft;
        value = raw + predictor;
      } else {
        throw new Error(`Unsupported PNG filter ${filter}`);
      }
      pixels[y * stride + x] = value & 0xff;
    }
  }

  return { width, height, pixels };
}

function markBounds(png) {
  let minX = png.width;
  let maxX = -1;
  let minY = png.height;
  let maxY = -1;

  for (let y = 0; y < png.height; y += 1) {
    for (let x = 0; x < png.width; x += 1) {
      const offset = (y * png.width + x) * 4;
      const [red, green, blue, alpha] = png.pixels.subarray(offset, offset + 4);
      const isWhiteMark = red + green + blue > 570;
      const isPurpleMark = red > 115 && blue > 145 && red - green > 35;
      if (alpha > 180 && (isWhiteMark || isPurpleMark)) {
        minX = Math.min(minX, x);
        maxX = Math.max(maxX, x);
        minY = Math.min(minY, y);
        maxY = Math.max(maxY, y);
      }
    }
  }

  if (maxX < minX || maxY < minY) throw new Error("No readable BetterFy mark found");
  return {
    widthRatio: (maxX - minX + 1) / png.width,
    heightRatio: (maxY - minY + 1) / png.height,
  };
}

const source = decodePng(await readFile(resolve(root, "src-tauri/icons/icon-source.png")));
const compact = decodePng(await readFile(resolve(root, "src-tauri/icons/32x32.png")));
const sourceBounds = markBounds(source);
const compactBounds = markBounds(compact);

if (sourceBounds.widthRatio < 0.72 || sourceBounds.heightRatio < 0.18) {
  throw new Error(`Source wordmark is too small: ${JSON.stringify(sourceBounds)}`);
}
if (compactBounds.widthRatio < 0.65 || compactBounds.heightRatio < 0.16) {
  throw new Error(`32px wordmark is too small: ${JSON.stringify(compactBounds)}`);
}

const ico = await readFile(resolve(root, "src-tauri/icons/icon.ico"));
if (ico.readUInt16LE(0) !== 0 || ico.readUInt16LE(2) !== 1) throw new Error("Invalid ICO header");
const entryCount = ico.readUInt16LE(4);
const sizes = new Set();
for (let index = 0; index < entryCount; index += 1) {
  const size = ico[6 + index * 16] || 256;
  sizes.add(size);
}
for (const requiredSize of [16, 24, 32, 48, 64, 256]) {
  if (!sizes.has(requiredSize)) throw new Error(`Windows ICO is missing ${requiredSize}px`);
}

console.log(
  `BetterFy Windows icon verified: source ${(sourceBounds.widthRatio * 100).toFixed(1)}%, ` +
  `32px ${(compactBounds.widthRatio * 100).toFixed(1)}%, ICO ${[...sizes].sort((a, b) => a - b).join("/")}px`,
);
