import { describe, expect, it } from "vitest";
import { convertLegacy, faceRect, floodFill, getPx, locate, parts, previewLayout, setPx, template } from "./skin";

const part = (name: string, model: "classic" | "slim" = "classic") => parts(model).find((p) => p.name === name)!;

describe("skin geometry", () => {
  it("matches the standard 64x64 UV layout", () => {
    expect(faceRect(part("head"), "front")).toEqual({ x: 8, y: 8, w: 8, h: 8 });
    expect(faceRect(part("hat"), "front")).toEqual({ x: 40, y: 8, w: 8, h: 8 });
    expect(faceRect(part("body"), "front")).toEqual({ x: 20, y: 20, w: 8, h: 12 });
    expect(faceRect(part("rarm"), "back")).toEqual({ x: 52, y: 20, w: 4, h: 12 });
    expect(faceRect(part("rarm", "slim"), "back")).toEqual({ x: 51, y: 20, w: 3, h: 12 });
    expect(faceRect(part("larm"), "front")).toEqual({ x: 36, y: 52, w: 4, h: 12 });
    expect(faceRect(part("lpants"), "front")).toEqual({ x: 4, y: 52, w: 4, h: 12 });
  });

  it("locates pixels", () => {
    expect(locate("classic", 10, 10)?.part.name).toBe("head");
    expect(locate("classic", 10, 10)?.face).toBe("front");
    expect(locate("classic", 0, 0)).toBeNull();
  });

  it("previews only use in-bounds faces", () => {
    for (const model of ["classic", "slim"] as const) {
      for (const side of ["front", "back"] as const) {
        for (const item of previewLayout(model, side)) {
          const r = faceRect(part(item.part, model), item.face);
          expect(r.x + r.w).toBeLessThanOrEqual(64);
          expect(item.dx + r.w).toBeLessThanOrEqual(16);
        }
      }
    }
  });
});

describe("skin editing", () => {
  it("template paints base layers and leaves overlays transparent", () => {
    const px = template("classic");
    expect(getPx(px, 10, 10)[3]).toBe(255); // head front
    expect(getPx(px, 42, 10)[3]).toBe(0); // hat front is transparent
  });

  it("flood fill stays within a colour region", () => {
    const px = template("classic");
    const before = getPx(px, 40, 10);
    const n = floodFill(px, 40, 10, [255, 0, 0, 255]);
    expect(n).toBeGreaterThan(0);
    expect(getPx(px, 40, 10)).toEqual([255, 0, 0, 255]);
    expect(before[3]).toBe(0);
    expect(getPx(px, 10, 10)[0]).not.toBe(255);
  });

  it("converts legacy skins by mirroring the right limbs", () => {
    const px = new Uint8ClampedArray(64 * 64 * 4);
    const rlegFront = faceRect(part("rleg"), "front");
    setPx(px, rlegFront.x, rlegFront.y, [1, 2, 3, 255]); // left-most pixel of right leg front
    const out = convertLegacy(px);
    const llegFront = faceRect(part("lleg"), "front");
    expect(getPx(out, llegFront.x + llegFront.w - 1, llegFront.y)).toEqual([1, 2, 3, 255]);
  });
});
