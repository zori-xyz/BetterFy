import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { BOT_CARD_FILES, botCardFile } from "../src/index.mjs";

const ASSET_DIRECTORY = new URL("../../../website/public/bot/", import.meta.url);

function jpegDimensions(bytes) {
  assert.equal(bytes[0], 0xff);
  assert.equal(bytes[1], 0xd8);
  let offset = 2;
  while (offset + 8 < bytes.length) {
    if (bytes[offset] !== 0xff) {
      offset += 1;
      continue;
    }
    const marker = bytes[offset + 1];
    offset += 2;
    if (marker === 0xd8 || marker === 0xd9) continue;
    const segmentLength = bytes.readUInt16BE(offset);
    if (marker >= 0xc0 && marker <= 0xc3) {
      return {
        height: bytes.readUInt16BE(offset + 3),
        width: bytes.readUInt16BE(offset + 5),
      };
    }
    offset += segmentLength;
  }
  throw new Error("jpeg_dimensions_missing");
}

test("every bot state has a distinct Russian and English 16:9 card", async () => {
  const names = Object.values(BOT_CARD_FILES).flatMap(({ en, ru }) => [en, ru]);
  assert.equal(names.length, 10);
  assert.equal(new Set(names).size, names.length);

  for (const name of names) {
    assert.match(name, /^[a-z-]+\.(?:jpg|jpeg)$/);
    const bytes = await readFile(new URL(name, ASSET_DIRECTORY));
    assert.ok(bytes.length < 10 * 1024 * 1024, `${name} exceeds Telegram's photo limit`);
    assert.deepEqual(jpegDimensions(bytes), { width: 1280, height: 720 });
  }
});

test("card selection is localized and defaults unknown languages to English", () => {
  assert.equal(botCardFile("mainMenu", "ru"), "main-menu-ru.jpg");
  assert.equal(botCardFile("mainMenu", "en"), "main-menu-en.jpg");
  assert.equal(botCardFile("mainMenu", "nl"), "main-menu-en.jpg");
  assert.throws(() => botCardFile("missing", "ru"), /unknown_bot_card/);
});
