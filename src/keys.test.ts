import { describe, expect, it } from "vitest";
import { keyFromCode, keyFromMouse, keyLabel, UNBOUND } from "./keys";

describe("Minecraft key names", () => {
  it("maps browser key codes to options.txt names", () => {
    expect(keyFromCode("KeyC")).toBe("key.keyboard.c");
    expect(keyFromCode("Digit4")).toBe("key.keyboard.4");
    expect(keyFromCode("ShiftRight")).toBe("key.keyboard.right.shift");
    expect(keyFromCode("ControlLeft")).toBe("key.keyboard.left.control");
    expect(keyFromCode("F5")).toBe("key.keyboard.f5");
    expect(keyFromCode("Numpad7")).toBe("key.keyboard.keypad.7");
    expect(keyFromCode("Backquote")).toBe("key.keyboard.grave.accent");
    expect(keyFromCode("Escape")).toBeNull();
    expect(keyFromCode("MediaPlayPause")).toBeNull();
  });

  it("maps mouse buttons like Minecraft does", () => {
    expect(keyFromMouse(0)).toBe("key.mouse.left");
    expect(keyFromMouse(1)).toBe("key.mouse.middle");
    expect(keyFromMouse(2)).toBe("key.mouse.right");
    expect(keyFromMouse(3)).toBe("key.mouse.4");
    expect(keyFromMouse(4)).toBe("key.mouse.5");
  });

  it("gives readable labels", () => {
    expect(keyLabel("key.keyboard.right.shift")).toBe("Right Shift");
    expect(keyLabel("key.keyboard.left.control")).toBe("Left Ctrl");
    expect(keyLabel("key.keyboard.f5")).toBe("F5");
    expect(keyLabel("key.keyboard.c")).toBe("C");
    expect(keyLabel("key.mouse.4")).toBe("Mouse 4");
    expect(keyLabel("key.mouse.left")).toBe("Left Click");
    expect(keyLabel("key.keyboard.keypad.7")).toBe("Numpad 7");
    expect(keyLabel("key.keyboard.page.up")).toBe("Page Up");
    expect(keyLabel(UNBOUND)).toBe("Not bound");
  });
});
