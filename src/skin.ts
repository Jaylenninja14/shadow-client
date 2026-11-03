// Minecraft skin geometry (64×64 layout), an original starter template, legacy 64×32
// conversion and front/back preview rendering. Pure functions over RGBA buffers.

export type SkinModel = "classic" | "slim";
export type Face = "top" | "bottom" | "right" | "front" | "left" | "back";
export interface Rect { x: number; y: number; w: number; h: number }
export interface Part { name: string; label: string; origin: [number, number]; size: [number, number, number]; overlay: boolean }

export const SIZE = 64;

export function parts(model: SkinModel): Part[] {
  const aw = model === "slim" ? 3 : 4;
  return [
    { name: "head", label: "Head", origin: [0, 0], size: [8, 8, 8], overlay: false },
    { name: "body", label: "Body", origin: [16, 16], size: [8, 12, 4], overlay: false },
    { name: "rarm", label: "Right arm", origin: [40, 16], size: [aw, 12, 4], overlay: false },
    { name: "larm", label: "Left arm", origin: [32, 48], size: [aw, 12, 4], overlay: false },
    { name: "rleg", label: "Right leg", origin: [0, 16], size: [4, 12, 4], overlay: false },
    { name: "lleg", label: "Left leg", origin: [16, 48], size: [4, 12, 4], overlay: false },
    { name: "hat", label: "Hat (overlay)", origin: [32, 0], size: [8, 8, 8], overlay: true },
    { name: "jacket", label: "Jacket (overlay)", origin: [16, 32], size: [8, 12, 4], overlay: true },
    { name: "rsleeve", label: "Right sleeve (overlay)", origin: [40, 32], size: [aw, 12, 4], overlay: true },
    { name: "lsleeve", label: "Left sleeve (overlay)", origin: [48, 48], size: [aw, 12, 4], overlay: true },
    { name: "rpants", label: "Right pants (overlay)", origin: [0, 32], size: [4, 12, 4], overlay: true },
    { name: "lpants", label: "Left pants (overlay)", origin: [0, 48], size: [4, 12, 4], overlay: true },
  ];
}

/** UV rectangle of one face of a cuboid part (standard Minecraft box UV unwrap). */
export function faceRect(p: Part, face: Face): Rect {
  const [u, v] = p.origin;
  const [w, h, d] = p.size;
  switch (face) {
    case "top": return { x: u + d, y: v, w, h: d };
    case "bottom": return { x: u + d + w, y: v, w, h: d };
    case "right": return { x: u, y: v + d, w: d, h };
    case "front": return { x: u + d, y: v + d, w, h };
    case "left": return { x: u + d + w, y: v + d, w: d, h };
    case "back": return { x: u + d + w + d, y: v + d, w, h };
  }
}

export const FACES: Face[] = ["top", "bottom", "right", "front", "left", "back"];

/** Which part/face a texture pixel belongs to (for hover labels). */
export function locate(model: SkinModel, x: number, y: number): { part: Part; face: Face } | null {
  for (const p of parts(model)) {
    for (const f of FACES) {
      const r = faceRect(p, f);
      if (x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h) return { part: p, face: f };
    }
  }
  return null;
}

export type Pixels = Uint8ClampedArray; // 64*64*4 RGBA

export function blank(): Pixels {
  return new Uint8ClampedArray(SIZE * SIZE * 4);
}

export function hexToRgba(hex: string, alpha = 255): [number, number, number, number] {
  const h = hex.replace("#", "");
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16), alpha];
}

export function rgbaToHex(r: number, g: number, b: number): string {
  return "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");
}

export function getPx(px: Pixels, x: number, y: number): [number, number, number, number] {
  const i = (y * SIZE + x) * 4;
  return [px[i], px[i + 1], px[i + 2], px[i + 3]];
}

export function setPx(px: Pixels, x: number, y: number, c: [number, number, number, number]) {
  if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) return;
  const i = (y * SIZE + x) * 4;
  px[i] = c[0];
  px[i + 1] = c[1];
  px[i + 2] = c[2];
  px[i + 3] = c[3];
}

function fillRect(px: Pixels, r: Rect, c: [number, number, number, number]) {
  for (let y = r.y; y < r.y + r.h; y++) for (let x = r.x; x < r.x + r.w; x++) setPx(px, x, y, c);
}

/** Flood fill (4-neighbour) of the region with the same colour as (x, y). */
export function floodFill(px: Pixels, x: number, y: number, c: [number, number, number, number]): number {
  const target = getPx(px, x, y);
  if (target.every((v, i) => v === c[i])) return 0;
  const stack: [number, number][] = [[x, y]];
  let n = 0;
  while (stack.length) {
    const [cx, cy] = stack.pop()!;
    if (cx < 0 || cy < 0 || cx >= SIZE || cy >= SIZE) continue;
    const cur = getPx(px, cx, cy);
    if (!cur.every((v, i) => v === target[i])) continue;
    setPx(px, cx, cy, c);
    n++;
    stack.push([cx + 1, cy], [cx - 1, cy], [cx, cy + 1], [cx, cy - 1]);
  }
  return n;
}

/** An original starter skin (not Mojang's Steve/Alex): sunny shirt, jeans, shoes. */
export function template(model: SkinModel): Pixels {
  const px = blank();
  const skin = hexToRgba("#d8a273");
  const hair = hexToRgba("#5a3a22");
  const shirt = hexToRgba("#ffb627");
  const shirtDark = hexToRgba("#e0961a");
  const pants = hexToRgba("#3b4a6b");
  const shoes = hexToRgba("#2b2b30");
  const white = hexToRgba("#ffffff");
  const eye = hexToRgba("#3a6ea5");
  const mouth = hexToRgba("#a8644a");
  const ps = Object.fromEntries(parts(model).map((p) => [p.name, p]));
  // head
  for (const f of FACES) fillRect(px, faceRect(ps.head, f), skin);
  fillRect(px, faceRect(ps.head, "top"), hair);
  fillRect(px, faceRect(ps.head, "back"), hair);
  for (const f of ["front", "left", "right"] as Face[]) {
    const r = faceRect(ps.head, f);
    fillRect(px, { x: r.x, y: r.y, w: r.w, h: 2 }, hair);
  }
  const hf = faceRect(ps.head, "front");
  setPx(px, hf.x + 1, hf.y + 4, white);
  setPx(px, hf.x + 2, hf.y + 4, eye);
  setPx(px, hf.x + 5, hf.y + 4, eye);
  setPx(px, hf.x + 6, hf.y + 4, white);
  setPx(px, hf.x + 3, hf.y + 6, mouth);
  setPx(px, hf.x + 4, hf.y + 6, mouth);
  // body
  for (const f of FACES) fillRect(px, faceRect(ps.body, f), shirt);
  const bf = faceRect(ps.body, "front");
  fillRect(px, { x: bf.x, y: bf.y + 11, w: bf.w, h: 1 }, shirtDark);
  // arms: sleeve on the upper third, skin below
  for (const arm of [ps.rarm, ps.larm]) {
    for (const f of FACES) fillRect(px, faceRect(arm, f), skin);
    fillRect(px, faceRect(arm, "top"), shirt);
    for (const f of ["front", "back", "left", "right"] as Face[]) {
      const r = faceRect(arm, f);
      fillRect(px, { x: r.x, y: r.y, w: r.w, h: 4 }, shirt);
    }
  }
  // legs
  for (const leg of [ps.rleg, ps.lleg]) {
    for (const f of FACES) fillRect(px, faceRect(leg, f), pants);
    fillRect(px, faceRect(leg, "bottom"), shoes);
    for (const f of ["front", "back", "left", "right"] as Face[]) {
      const r = faceRect(leg, f);
      fillRect(px, { x: r.x, y: r.y + 10, w: r.w, h: 2 }, shoes);
    }
  }
  return px;
}

function copyRect(src: Pixels, from: Rect, dst: Pixels, to: Rect, flipH: boolean) {
  for (let y = 0; y < from.h; y++) {
    for (let x = 0; x < from.w; x++) {
      const sx = flipH ? from.x + from.w - 1 - x : from.x + x;
      setPx(dst, to.x + x, to.y + y, getPx(src, sx, from.y + y));
    }
  }
}

/**
 * Convert a legacy 64×32 skin (already drawn into the top half of a 64×64 buffer) by
 * mirroring the right arm/leg into the left arm/leg slots, as Minecraft does.
 */
export function convertLegacy(px: Pixels): Pixels {
  const out = new Uint8ClampedArray(px);
  const ps = Object.fromEntries(parts("classic").map((p) => [p.name, p]));
  for (const [src, dst] of [["rleg", "lleg"], ["rarm", "larm"]] as const) {
    const s = ps[src];
    const d = ps[dst];
    for (const f of ["top", "bottom", "front", "back"] as Face[]) copyRect(px, faceRect(s, f), out, faceRect(d, f), true);
    copyRect(px, faceRect(s, "left"), out, faceRect(d, "right"), true);
    copyRect(px, faceRect(s, "right"), out, faceRect(d, "left"), true);
  }
  return out;
}

/** Where each visible face goes in a 16×32 front/back preview. */
export function previewLayout(model: SkinModel, side: "front" | "back"): { part: string; face: Face; dx: number; dy: number }[] {
  const slim = model === "slim";
  const armX = (left: boolean) => (left ? (slim ? 1 : 0) : 12);
  if (side === "front") {
    return [
      { part: "head", face: "front", dx: 4, dy: 0 },
      { part: "body", face: "front", dx: 4, dy: 8 },
      { part: "rarm", face: "front", dx: armX(true), dy: 8 },
      { part: "larm", face: "front", dx: armX(false), dy: 8 },
      { part: "rleg", face: "front", dx: 4, dy: 20 },
      { part: "lleg", face: "front", dx: 8, dy: 20 },
      { part: "hat", face: "front", dx: 4, dy: 0 },
      { part: "jacket", face: "front", dx: 4, dy: 8 },
      { part: "rsleeve", face: "front", dx: armX(true), dy: 8 },
      { part: "lsleeve", face: "front", dx: armX(false), dy: 8 },
      { part: "rpants", face: "front", dx: 4, dy: 20 },
      { part: "lpants", face: "front", dx: 8, dy: 20 },
    ];
  }
  return [
    { part: "head", face: "back", dx: 4, dy: 0 },
    { part: "body", face: "back", dx: 4, dy: 8 },
    { part: "larm", face: "back", dx: armX(true), dy: 8 },
    { part: "rarm", face: "back", dx: armX(false), dy: 8 },
    { part: "lleg", face: "back", dx: 4, dy: 20 },
    { part: "rleg", face: "back", dx: 8, dy: 20 },
    { part: "hat", face: "back", dx: 4, dy: 0 },
    { part: "jacket", face: "back", dx: 4, dy: 8 },
    { part: "lsleeve", face: "back", dx: armX(true), dy: 8 },
    { part: "rsleeve", face: "back", dx: armX(false), dy: 8 },
    { part: "lpants", face: "back", dx: 4, dy: 20 },
    { part: "rpants", face: "back", dx: 8, dy: 20 },
  ];
}

/** Draw a front or back preview (16×32 pixels × scale) from texture pixels. */
export function drawPreview(ctx: CanvasRenderingContext2D, px: Pixels, model: SkinModel, side: "front" | "back", scale: number, showOverlay = true) {
  const ps = Object.fromEntries(parts(model).map((p) => [p.name, p]));
  ctx.clearRect(0, 0, 16 * scale, 32 * scale);
  for (const item of previewLayout(model, side)) {
    const p = ps[item.part];
    if (p.overlay && !showOverlay) continue;
    const r = faceRect(p, item.face);
    for (let y = 0; y < r.h; y++) {
      for (let x = 0; x < r.w; x++) {
        const [cr, cg, cb, ca] = getPx(px, r.x + x, r.y + y);
        if (ca === 0) continue;
        // Base layers are drawn opaque, like the game does.
        ctx.fillStyle = `rgba(${cr},${cg},${cb},${p.overlay ? ca / 255 : 1})`;
        ctx.fillRect((item.dx + x) * scale, (item.dy + y) * scale, scale, scale);
      }
    }
  }
}

/** Decode a PNG data URL into 64×64 pixels (legacy 64×32 skins are converted). */
export async function loadPixels(dataUrl: string): Promise<{ px: Pixels; legacy: boolean }> {
  const img = new Image();
  img.src = dataUrl;
  await img.decode();
  const c = document.createElement("canvas");
  c.width = SIZE;
  c.height = SIZE;
  const ctx = c.getContext("2d")!;
  ctx.drawImage(img, 0, 0);
  const px = new Uint8ClampedArray(ctx.getImageData(0, 0, SIZE, SIZE).data);
  const legacy = img.height === 32;
  return { px: legacy ? convertLegacy(px) : px, legacy };
}

export function toDataUrl(px: Pixels): string {
  const c = document.createElement("canvas");
  c.width = SIZE;
  c.height = SIZE;
  const ctx = c.getContext("2d")!;
  ctx.putImageData(new ImageData(new Uint8ClampedArray(px), SIZE, SIZE), 0, 0);
  return c.toDataURL("image/png");
}
