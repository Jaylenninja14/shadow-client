import { describe, expect, it } from "vitest";
import { bytes, count, duration, percent, relativeTime } from "./format";

describe("format", () => {
  it("formats bytes", () => {
    expect(bytes(0)).toBe("0 B");
    expect(bytes(1536)).toBe("1.5 KB");
    expect(bytes(5 * 1024 * 1024)).toBe("5.0 MB");
    expect(bytes(null)).toBe("—");
  });
  it("formats counts without inventing data", () => {
    expect(count(null)).toBe("—");
    expect(count(1234)).toBe("1.2K");
    expect(count(2_500_000)).toBe("2.5M");
  });
  it("relative time", () => {
    const now = new Date("2026-10-29T12:00:00Z");
    expect(relativeTime(null, now)).toBe("never");
    expect(relativeTime("2026-10-29T11:30:00Z", now)).toBe("30 min ago");
    expect(relativeTime("2026-10-27T12:00:00Z", now)).toBe("2 d ago");
  });
  it("duration and percent", () => {
    expect(duration(3720)).toBe("1 h 2 min");
    expect(percent(50, 200)).toBe(25);
    expect(percent(5, null)).toBe(0);
  });
});
