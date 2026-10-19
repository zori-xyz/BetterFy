import { toJpeg } from "html-to-image";

// Pictures for a tester's report: a snapshot of BetterFy's own window, drawn
// from the page itself (no screen capture, nothing outside the app), and a
// picture the tester picks, such as a Steam screenshot of Dota. Both become
// JPEGs within the auth service's 5 MB limit and Telegram's photo limits.

const MAX_BYTES = 5 * 1024 * 1024;
const MAX_SIDE = 1920;
// A transparent pixel stands in for a picture the page cannot read back
// (catalog previews from other hosts are outside connect-src).
const PLACEHOLDER =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

const dataUrlBytes = (dataUrl: string) =>
  Math.floor(((dataUrl.length - dataUrl.indexOf(",") - 1) * 3) / 4);

export async function snapshotWindow() {
  const root = document.body;
  const scale = Math.min(1, MAX_SIDE / Math.max(root.clientWidth, root.clientHeight));
  for (const quality of [0.86, 0.7, 0.55]) {
    const dataUrl = await toJpeg(root, {
      quality,
      pixelRatio: scale,
      cacheBust: false,
      skipFonts: false,
      imagePlaceholder: PLACEHOLDER,
      backgroundColor:
        getComputedStyle(document.documentElement).getPropertyValue("--s-bg").trim() || "#111116",
      filter: (node) => !(node instanceof HTMLElement && node.dataset.testerUi !== undefined),
    });
    if (dataUrlBytes(dataUrl) <= MAX_BYTES) return dataUrl;
  }
  throw new Error("tester_image_too_large");
}

function loadImage(file: File) {
  return new Promise<HTMLImageElement>((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const image = new Image();
    image.onload = () => {
      URL.revokeObjectURL(url);
      resolve(image);
    };
    image.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("tester_image_invalid"));
    };
    image.src = url;
  });
}

/** A picked picture, scaled to at most 1920 px on its long side. */
export async function attachPicture(file: File) {
  if (!/^image\/(png|jpeg|webp)$/.test(file.type) || file.size > 40 * 1024 * 1024) {
    throw new Error("tester_image_invalid");
  }
  const image = await loadImage(file);
  const scale = Math.min(1, MAX_SIDE / Math.max(image.naturalWidth, image.naturalHeight));
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(image.naturalWidth * scale));
  canvas.height = Math.max(1, Math.round(image.naturalHeight * scale));
  const context = canvas.getContext("2d");
  if (!context) throw new Error("tester_image_invalid");
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
  for (const quality of [0.88, 0.72, 0.55]) {
    const dataUrl = canvas.toDataURL("image/jpeg", quality);
    if (dataUrlBytes(dataUrl) <= MAX_BYTES) return dataUrl;
  }
  throw new Error("tester_image_too_large");
}

export const dataUrlBase64 = (dataUrl: string) => dataUrl.slice(dataUrl.indexOf(",") + 1);
