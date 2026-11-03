package com.shadowclient.menu.hud;

import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ChoiceSetting;
import com.shadowclient.menu.setting.ColorSetting;
import com.shadowclient.menu.setting.NumberSetting;
import com.shadowclient.menu.ui.Draw;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.gui.GuiGraphics;

/** WASD, mouse buttons (with CPS) and the space bar, lighting up as you press them. */
final class Keystrokes extends HudElement {
	private static final String STYLE = "Style";
	final BoolSetting mouse = add(new BoolSetting("mouse", "Show mouse buttons", true));
	final BoolSetting cps = add(new BoolSetting("cps", "CPS on mouse buttons", true).showIf(mouse::on));
	final BoolSetting space = add(new BoolSetting("space", "Show space bar", true));
	final BoolSetting realNames = add(new BoolSetting("key-names", "Show your bound keys", true));
	final NumberSetting size = add(new NumberSetting("key-size", "Key size", 20, 14, 30, 1));
	final NumberSetting gap = add(new NumberSetting("gap", "Gap", 2, 0, 5, 1));
	final NumberSetting fade = add(new NumberSetting("fade", "Fade time", 120, 0, 400, 10).suffix(" ms"));
	final ColorSetting keyColor = add(new ColorSetting("key-color", "Key colour", 0x6F000000, true).group(STYLE));
	final ColorSetting pressedColor = add(new ColorSetting("pressed-color", "Pressed colour", 0xCCFFFFFF, true).group(STYLE));
	final ColorSetting textColor = add(new ColorSetting("text-color", "Text colour", 0xFFFFFFFF, false).group(STYLE));
	final ColorSetting pressedText = add(new ColorSetting("pressed-text", "Pressed text colour", 0xFF111111, false).group(STYLE));
	final BoolSetting shadow = add(new BoolSetting("shadow", "Text shadow", true).group(STYLE));
	final ChoiceSetting corners = add(new ChoiceSetting("corners", "Corners", "rounded", List.of("square", "rounded"), List.of("Square", "Rounded")).group(STYLE));
	final BoolSetting border = add(new BoolSetting("border", "Border", false).group(STYLE));
	final ColorSetting borderColor = add(new ColorSetting("border-color", "Border colour", 0xFF9B7BFF, false).group(STYLE).showIf(border::on));
	private final Map<String, Float> glow = new HashMap<>();
	private long lastFrame = System.nanoTime();

	Keystrokes() {
		super("keystrokes", "Keystrokes", "keystrokes", InfoElements.PVP, true, 0.004, 0.24);
		describe("WASD, mouse buttons and space bar, lit while pressed.");
		anchor(Anchor.TOP_LEFT);
	}

	private int k() {
		return size.i();
	}

	private int g() {
		return gap.i();
	}

	@Override
	public int width(boolean preview) {
		return k() * 3 + g() * 2;
	}

	@Override
	public int height(boolean preview) {
		int h = k() * 2 + g();
		if (mouse.on()) h += g() + k();
		if (space.on()) h += g() + Math.max(8, k() / 2);
		return h;
	}

	private String name(KeyMapping key, String fallback) {
		if (!realNames.on()) return fallback;
		String s = key.getTranslatedKeyMessage().getString();
		if (s.length() > 3) s = s.length() <= 5 ? s : fallback;
		return s.toUpperCase(java.util.Locale.ROOT);
	}

	@Override
	public void render(GuiGraphics g, boolean preview) {
		long now = System.nanoTime();
		float dt = Math.min(0.1f, (now - lastFrame) / 1e9f);
		lastFrame = now;
		var o = mc.options;
		int k = k(), gp = g();
		key(g, "w", name(o.keyUp, "W"), o.keyUp.isDown(), k + gp, 0, k, k, dt, null);
		key(g, "a", name(o.keyLeft, "A"), o.keyLeft.isDown(), 0, k + gp, k, k, dt, null);
		key(g, "s", name(o.keyDown, "S"), o.keyDown.isDown(), k + gp, k + gp, k, k, dt, null);
		key(g, "d", name(o.keyRight, "D"), o.keyRight.isDown(), (k + gp) * 2, k + gp, k, k, dt, null);
		int y = (k + gp) * 2;
		int full = width(preview);
		if (mouse.on()) {
			int half = (full - gp) / 2;
			key(g, "lmb", "LMB", o.keyAttack.isDown(), 0, y, half, k, dt, cps.on() ? HudStats.leftCps() + " CPS" : null);
			key(g, "rmb", "RMB", o.keyUse.isDown(), half + gp, y, full - half - gp, k, dt, cps.on() ? HudStats.rightCps() + " CPS" : null);
			y += k + gp;
		}
		if (space.on()) {
			int h = Math.max(8, k / 2);
			boolean down = o.keyJump.isDown();
			float p = glow(dt, "space", down);
			int bg = Draw.mix(keyColor.argb(), pressedColor.argb(), p);
			box(g, 0, y, full, h, bg);
			int bar = Draw.mix(textColor.argb(), pressedText.argb(), p);
			int bw = Math.max(6, full / 3);
			g.fill((full - bw) / 2, y + h / 2, (full - bw) / 2 + bw, y + h / 2 + 1, bar);
		}
	}

	private float glow(float dt, String id, boolean down) {
		float cur = glow.getOrDefault(id, 0f);
		float ms = fade.f();
		if (ms <= 0) cur = down ? 1 : 0;
		else cur = down ? Math.min(1, cur + dt * 1000 / ms * 2) : Math.max(0, cur - dt * 1000 / ms);
		glow.put(id, cur);
		return cur;
	}

	private void box(GuiGraphics g, int x, int y, int w, int h, int color) {
		if (corners.is("rounded")) Draw.round(g, x, y, w, h, 3, color);
		else g.fill(x, y, x + w, y + h, color);
		if (border.on()) {
			if (corners.is("rounded")) Draw.roundOutline(g, x, y, w, h, 3, borderColor.argb());
			else g.renderOutline(x, y, w, h, borderColor.argb());
		}
	}

	private void key(GuiGraphics g, String id, String label, boolean down, int x, int y, int w, int h, float dt, String sub) {
		float p = glow(dt, id, down);
		box(g, x, y, w, h, Draw.mix(keyColor.argb(), pressedColor.argb(), p));
		int tc = Draw.mix(textColor.argb(), pressedText.argb(), p);
		var font = font();
		if (sub == null) {
			g.drawString(font, label, x + (w - font.width(label)) / 2, y + (h - 8) / 2, tc, shadow.on() && p < 0.5f);
		} else {
			g.drawString(font, label, x + (w - font.width(label)) / 2, y + h / 2 - 8, tc, shadow.on() && p < 0.5f);
			Draw.scaledText(g, font, sub, x + (w - font.width(sub) * 0.6f) / 2, y + h / 2 + 1.5f, 0.6f, tc, false);
		}
	}
}
