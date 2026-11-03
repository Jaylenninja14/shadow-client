// Minecraft key names (options.txt "key_…" values) <-> browser key events and readable labels.

const SPECIAL: Record<string, string> = {
  Space: "space", Tab: "tab", CapsLock: "caps.lock", Enter: "enter", Backspace: "backspace",
  ShiftLeft: "left.shift", ShiftRight: "right.shift", ControlLeft: "left.control", ControlRight: "right.control",
  AltLeft: "left.alt", AltRight: "right.alt", MetaLeft: "left.win", MetaRight: "right.win",
  Insert: "insert", Delete: "delete", Home: "home", End: "end", PageUp: "page.up", PageDown: "page.down",
  ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right",
  Backquote: "grave.accent", Minus: "minus", Equal: "equal", BracketLeft: "left.bracket", BracketRight: "right.bracket",
  Backslash: "backslash", Semicolon: "semicolon", Quote: "apostrophe", Comma: "comma", Period: "period", Slash: "slash",
  NumpadAdd: "keypad.add", NumpadSubtract: "keypad.subtract", NumpadMultiply: "keypad.multiply",
  NumpadDivide: "keypad.divide", NumpadDecimal: "keypad.decimal", NumpadEnter: "keypad.enter",
};

/** KeyboardEvent.code → Minecraft key name, or null for keys Minecraft can't bind. */
export function keyFromCode(code: string): string | null {
  let m = /^Key([A-Z])$/.exec(code);
  if (m) return `key.keyboard.${m[1].toLowerCase()}`;
  m = /^Digit([0-9])$/.exec(code);
  if (m) return `key.keyboard.${m[1]}`;
  m = /^Numpad([0-9])$/.exec(code);
  if (m) return `key.keyboard.keypad.${m[1]}`;
  m = /^F([1-9]|1[0-9]|2[0-5])$/.exec(code);
  if (m) return `key.keyboard.f${m[1]}`;
  return SPECIAL[code] ? `key.keyboard.${SPECIAL[code]}` : null;
}

/** MouseEvent.button → Minecraft key name. */
export function keyFromMouse(button: number): string {
  return ["key.mouse.left", "key.mouse.middle", "key.mouse.right"][button] ?? `key.mouse.${button + 1}`;
}

export const UNBOUND = "key.keyboard.unknown";

/** "key.keyboard.right.shift" → "Right Shift", "key.mouse.4" → "Mouse 4". */
export function keyLabel(key: string): string {
  if (key === UNBOUND) return "Not bound";
  const mouse = /^key\.mouse\.(.+)$/.exec(key);
  if (mouse) return { left: "Left Click", right: "Right Click", middle: "Middle Click" }[mouse[1]] ?? `Mouse ${mouse[1]}`;
  const name = key.replace(/^key\.keyboard\./, "");
  const words: Record<string, string> = { "grave.accent": "`", "left.bracket": "[", "right.bracket": "]", minus: "-", equal: "=", backslash: "\\", semicolon: ";", apostrophe: "'", comma: ",", period: ".", slash: "/", "caps.lock": "Caps Lock", "page.up": "Page Up", "page.down": "Page Down" };
  if (words[name]) return words[name];
  if (/^f\d+$/.test(name)) return name.toUpperCase();
  return name
    .split(".")
    .map((w) => (w === "keypad" ? "Numpad" : w === "control" ? "Ctrl" : w.charAt(0).toUpperCase() + w.slice(1)))
    .join(" ");
}
