package com.shadowclient.menu.ui;

import com.mojang.blaze3d.platform.InputConstants;
import com.shadowclient.menu.module.MenuConfig;
import com.shadowclient.menu.module.Module;
import com.shadowclient.menu.setting.ActionSetting;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ChoiceSetting;
import com.shadowclient.menu.setting.ColorSetting;
import com.shadowclient.menu.setting.KeySetting;
import com.shadowclient.menu.setting.NumberSetting;
import com.shadowclient.menu.setting.Setting;
import com.shadowclient.menu.setting.TextSetting;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.input.CharacterEvent;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.world.item.ItemStack;
import org.jetbrains.annotations.Nullable;
import org.lwjgl.glfw.GLFW;

/**
 * A module's settings page inside the mod menu: a header (back, icon, name, description, on/off
 * switch) and one row per setting with its control: switch, slider, colour picker, choice, key,
 * text field or button. Groups ("Style", …) get a heading.
 */
final class SettingsPage {
	private static final int ROW = 24, PICKER = 92;
	private final ModMenuScreen menu;
	final String title;
	final String icon;
	@Nullable final ItemStack item;
	@Nullable final String description;
	@Nullable final Module module;
	private final List<Setting<?>> settings;
	private final Runnable reset;
	private final Anim anim;
	private final Font font;
	private double scroll, scrollTarget;
	private int contentH;

	// interaction state
	private final List<Hit> hits = new ArrayList<>();
	@Nullable private NumberSetting dragSlider;
	private int sliderX, sliderW;
	@Nullable private ColorSetting openColor;
	@Nullable private ColorSetting dragColor;
	private int dragArea; // 1 = saturation/value square, 2 = hue, 3 = alpha
	private int areaX, areaY, areaW, areaH;
	private float hue = -1;
	@Nullable private KeySetting listening;
	@Nullable private Setting<String> editing; // a text setting, or a colour's hex field
	private String editBuffer = "";
	private boolean confirmReset;
	@Nullable private Hit resetHit;

	/** A clickable area found while drawing. */
	private record Hit(int x, int y, int w, int h, @Nullable Runnable left, @Nullable Runnable right) {
	}

	SettingsPage(ModMenuScreen menu, String title, String icon, @Nullable ItemStack item, @Nullable String description, List<Setting<?>> settings,
			@Nullable Module module, Runnable reset, Anim anim) {
		this.menu = menu;
		this.title = title;
		this.icon = icon;
		this.item = item;
		this.description = description;
		this.module = module;
		this.reset = reset;
		this.anim = anim;
		this.font = Minecraft.getInstance().font;
		// settings without a heading first, then each heading in the order it first appears
		List<Setting<?>> top = new ArrayList<>(), rest = new ArrayList<>();
		List<String> groups = new ArrayList<>();
		for (Setting<?> s : settings) {
			if (s.id.equals("enabled") && module != null) continue; // the header has the switch
			if (s.group == null) top.add(s);
			else if (!groups.contains(s.group)) groups.add(s.group);
		}
		for (String gname : groups) for (Setting<?> s : settings) if (gname.equals(s.group) && !(s.id.equals("enabled") && module != null)) rest.add(s);
		this.settings = new ArrayList<>(top);
		this.settings.addAll(rest);
	}

	boolean capturingKeys() {
		return listening != null || editing != null;
	}

	private int c(int color, float alpha) {
		return Draw.fade(color, alpha);
	}

	// ------------------------------------------------------------------ drawing

	void render(GuiGraphics g, int x, int y, int w, int h, int mx, int my, float alpha) {
		hits.clear();
		scroll += (scrollTarget - scroll) * Math.min(1, anim.dt() * 18);
		// header
		int hx = x + 10, hy = y + 8;
		boolean backHover = Draw.inside(mx, my, hx, hy, 20, 20);
		Draw.round(g, hx, hy, 20, 20, 5, c(backHover ? Theme.chipHover() : Theme.chip(), alpha));
		g.drawCenteredString(font, "<", hx + 10, hy + 6, c(Theme.text(), alpha));
		hits.add(new Hit(hx, hy, 20, 20, menu::closePage, null));
		int ix = hx + 28;
		Draw.round(g, ix, hy - 2, 24, 24, 6, c(Draw.mix(Theme.card(), Theme.accent(), 0.18f), alpha));
		if (item != null) Draw.item(g, item, ix + 4, hy + 2, 16);
		else Draw.icon(g, icon, ix + 4, hy + 2, 16, c(Theme.text(), alpha));
		int tx = ix + 32;
		Draw.scaledText(g, font, title, tx, hy + 1, 1.25f, c(Theme.text(), alpha), false);
		int right = x + w - 12;
		if (module != null && module.toggleable()) {
			int sw = 30;
			toggle(g, "page-on:" + module.id, module.enabled(), right - sw, hy + 3, sw, 15, alpha);
			hits.add(new Hit(right - sw, hy + 3, sw, 15, () -> {
				module.setEnabled(!module.enabled());
				menu.click();
			}, null));
			String state = module.enabled() ? "Enabled" : "Disabled";
			g.drawString(font, state, right - sw - 6 - font.width(state), hy + 7, c(module.enabled() ? Theme.accentText() : Theme.dim(), alpha), false);
			right -= sw + 12 + font.width("Disabled");
		}
		int descY = hy + 14, descLines = 0;
		if (description != null) {
			List<net.minecraft.util.FormattedCharSequence> lines = font.split(net.minecraft.network.chat.Component.literal(description), Math.max(60, right - tx - 4));
			descLines = Math.min(3, lines.size());
			for (int i = 0; i < descLines; i++) g.drawString(font, lines.get(i), tx, descY + i * 10, c(Theme.dim(), alpha), false);
		}
		// the list starts below however many lines the description took
		int top = Math.max(y + 44, descY + descLines * 10 + 10);
		g.fill(x + 10, top - 6, x + w - 10, top - 5, c(Theme.line(), alpha));

		// rows
		int listX = x + 10, listW = w - 26, listH = y + h - top - 6;
		contentH = rowsHeight();
		clampScroll(listH);
		g.enableScissor(listX - 2, top, x + w - 4, top + listH);
		int ry = top - (int) Math.round(scroll);
		String lastGroup = null;
		for (Setting<?> s : settings) {
			if (!s.visible()) continue;
			if (s.group != null && !Objects.equals(s.group, lastGroup)) {
				lastGroup = s.group;
				ry += 6;
				g.drawString(font, s.group.toUpperCase(java.util.Locale.ROOT), listX + 2, ry + 2, c(Theme.accentText(), alpha), false);
				ry += 14;
			}
			int rh = rowHeight(s);
			if (ry + rh >= top && ry <= top + listH) row(g, s, listX, ry, listW, rh, mx, my, alpha, top, top + listH);
			ry += rh + 4;
		}
		// reset to defaults
		ry += 6;
		String label = confirmReset ? "Click again to reset" : "Reset to defaults";
		int bw = font.width(label) + 20;
		boolean rh = Draw.inside(mx, my, listX, ry, bw, 18) && my >= top && my < top + listH;
		Draw.round(g, listX, ry, bw, 18, 4, c(confirmReset ? 0xFFB83A3A : rh ? Theme.chipHover() : Theme.chip(), alpha));
		g.drawCenteredString(font, label, listX + bw / 2, ry + 5, c(confirmReset ? 0xFFFFFFFF : Theme.dim(), alpha));
		if (ry + 18 > top && ry < top + listH) {
			resetHit = new Hit(listX, ry, bw, 18, () -> {
				if (confirmReset) {
					reset.run();
					openColor = null;
					confirmReset = false;
				} else {
					confirmReset = true;
				}
				menu.click();
			}, null);
			hits.add(resetHit);
		}
		g.disableScissor();

		// scrollbar
		if (contentH > listH) {
			int track = x + w - 8;
			int thumb = Math.max(18, listH * listH / contentH);
			int ty = top + (int) ((listH - thumb) * (scroll / (contentH - listH)));
			Draw.round(g, track, ty, 3, thumb, 1.5f, c(Theme.line(), alpha));
		}
	}

	private int rowHeight(Setting<?> s) {
		int h = s.description != null ? ROW + 8 : ROW;
		if (s == openColor) h += PICKER;
		return h;
	}

	private int rowsHeight() {
		int total = 0;
		String lastGroup = null;
		for (Setting<?> s : settings) {
			if (!s.visible()) continue;
			if (s.group != null && !Objects.equals(s.group, lastGroup)) {
				lastGroup = s.group;
				total += 20;
			}
			total += rowHeight(s) + 4;
		}
		return total + 30;
	}

	private void clampScroll(int viewH) {
		double max = Math.max(0, contentH - viewH);
		scrollTarget = Math.max(0, Math.min(max, scrollTarget));
		scroll = Math.max(0, Math.min(max, scroll));
	}

	private void row(GuiGraphics g, Setting<?> s, int x, int y, int w, int h, int mx, int my, float alpha, int clipTop, int clipBottom) {
		boolean hover = Draw.inside(mx, my, x, y, w, h) && my >= clipTop && my < clipBottom;
		float hv = anim.get("row:" + s.id + title, hover ? 1 : 0, 16);
		Draw.round(g, x, y, w, h, 5, c(Draw.mix(Theme.card(), Theme.cardHover(), hv), alpha));
		int textY = s.description != null ? y + 5 : y + (ROW - 8) / 2;
		g.drawString(font, s.name, x + 10, textY, c(Theme.text(), alpha), false);
		if (s.description != null) g.drawString(font, font.plainSubstrByWidth(s.description, w / 2), x + 10, textY + 11, c(Theme.faint(), alpha), false);
		int cy = y + (s.description != null ? (ROW + 8) / 2 : ROW / 2);
		int right = x + w - 10;
		boolean clickable = my >= clipTop && my < clipBottom;
		switch (s) {
			case BoolSetting b -> {
				toggle(g, "set:" + title + b.id, b.on(), right - 30, cy - 7, 30, 15, alpha);
				if (clickable) hits.add(new Hit(x, y, w, ROW, () -> {
					b.toggle();
					menu.click();
				}, null));
			}
			case NumberSetting n -> slider(g, n, right - 130, cy, 130, mx, my, alpha, clickable);
			case ColorSetting col -> colorRow(g, col, x, y, w, right, cy, mx, my, alpha, clickable);
			case ChoiceSetting ch -> {
				String label = ch.label();
				int pw = 22;
				for (String l : ch.labels) pw = Math.max(pw, font.width(l) + 28);
				pw = Math.min(pw, w / 2);
				boolean ph = Draw.inside(mx, my, right - pw, cy - 8, pw, 16);
				Draw.round(g, right - pw, cy - 8, pw, 16, 4, c(ph ? Theme.chipHover() : Theme.chip(), alpha));
				g.drawString(font, "<", right - pw + 6, cy - 4, c(Theme.accentText(), alpha), false);
				g.drawString(font, ">", right - 6 - font.width(">"), cy - 4, c(Theme.accentText(), alpha), false);
				Draw.fitCentered(g, font, label, right - pw / 2, cy - 4, pw - 26, c(Theme.text(), alpha));
				if (clickable) {
					// later hits are checked first: the "<" third of the pill steps back
					hits.add(new Hit(x, y, w, ROW, () -> {
						ch.step(1);
						menu.click();
					}, () -> {
						ch.step(-1);
						menu.click();
					}));
					hits.add(new Hit(right - pw, cy - 8, pw / 3, 16, () -> {
						ch.step(-1);
						menu.click();
					}, null));
				}
			}
			case KeySetting k -> {
				boolean on = listening == k;
				String label = on ? "Press a key…" : k.keyName();
				int bw = Math.max(70, font.width(label) + 16);
				boolean bh = Draw.inside(mx, my, right - bw, cy - 8, bw, 16);
				Draw.round(g, right - bw, cy - 8, bw, 16, 4, c(on ? Theme.accent() : bh ? Theme.chipHover() : Theme.chip(), alpha));
				g.drawCenteredString(font, label, right - bw / 2, cy - 4, c(on ? Theme.onAccent() : Theme.text(), alpha));
				if (on) g.drawString(font, "Esc: none", right - bw - 6 - font.width("Esc: none"), cy - 4, c(Theme.faint(), alpha), false);
				if (clickable) hits.add(new Hit(x, y, w, ROW, () -> {
					listening = listening == k ? null : k;
					editing = null;
					menu.click();
				}, null));
			}
			case TextSetting t -> textField(g, t, right - 140, cy - 8, 140, 16, mx, my, alpha, clickable);
			case ActionSetting a -> {
				int bw = font.width(a.button()) + 20;
				boolean bh = Draw.inside(mx, my, right - bw, cy - 8, bw, 16);
				Draw.round(g, right - bw, cy - 8, bw, 16, 4, c(bh ? Draw.mix(Theme.accent(), 0xFFFFFFFF, 0.15f) : Theme.accent(), alpha));
				g.drawCenteredString(font, a.button(), right - bw / 2, cy - 4, c(Theme.onAccent(), alpha));
				if (clickable) hits.add(new Hit(right - bw, cy - 8, bw, 16, () -> {
					menu.click();
					MenuConfig.flush(true);
					a.run.accept(menu);
				}, null));
			}
			default -> {
			}
		}
	}

	/** An animated on/off switch. */
	void toggle(GuiGraphics g, String key, boolean on, int x, int y, int w, int h, float alpha) {
		float p = anim.get(key, on ? 1 : 0, 14);
		Draw.round(g, x, y, w, h, h / 2f, c(Draw.mix(Theme.off(), Theme.accent(), p), alpha));
		float knob = h - 4;
		float kx = x + 2 + (w - 4 - knob) * Draw.ease(p);
		Draw.round(g, kx, y + 2, knob, knob, knob / 2, c(0xFFFFFFFF, alpha));
	}

	private void slider(GuiGraphics g, NumberSetting n, int x, int cy, int w, int mx, int my, float alpha, boolean clickable) {
		String v = n.text();
		g.drawString(font, v, x - 8 - font.width(v), cy - 4, c(Theme.dim(), alpha), false);
		float f = anim.get("slider:" + title + n.id, (float) n.fraction(), 20);
		Draw.round(g, x, cy - 2, w, 4, 2, c(Theme.off(), alpha));
		Draw.round(g, x, cy - 2, Math.max(4, w * f), 4, 2, c(Theme.accent(), alpha));
		boolean hover = dragSlider == n || Draw.inside(mx, my, x - 4, cy - 7, w + 8, 14);
		float knob = hover ? 10 : 8;
		Draw.circle(g, x + w * f, cy, knob / 2, c(0xFFFFFFFF, alpha));
		if (clickable) hits.add(new Hit(x - 4, cy - 7, w + 8, 14, () -> {
			dragSlider = n;
			sliderX = x;
			sliderW = w;
			n.setFraction((Minecraft.getInstance().mouseHandler.getScaledXPos(Minecraft.getInstance().getWindow()) - x) / w);
		}, null));
	}

	private void colorRow(GuiGraphics g, ColorSetting col, int x, int y, int w, int right, int cy, int mx, int my, float alpha, boolean clickable) {
		int sw = 34;
		checker(g, right - sw, cy - 7, sw, 14);
		Draw.round(g, right - sw, cy - 7, sw, 14, 3, col.argb());
		Draw.roundOutline(g, right - sw, cy - 7, sw, 14, 3, c(Theme.line(), alpha));
		String hex = col.chroma() ? "Chroma" : col.hex();
		g.drawString(font, hex, right - sw - 8 - font.width(hex), cy - 4, c(Theme.dim(), alpha), false);
		if (clickable) hits.add(new Hit(x, y, w, ROW + (col.description != null ? 8 : 0), () -> {
			openColor = openColor == col ? null : col;
			hue = -1;
			menu.click();
		}, null));
		if (openColor != col) return;

		// the picker: saturation/value square, hue bar, alpha bar, chroma, presets and hex
		int py = y + (col.description != null ? ROW + 8 : ROW) + 2;
		int px = x + 10;
		int sqW = 110, sqH = 72;
		float[] hsv = Draw.toHsv(col.get());
		if (hue < 0 || dragColor == null) hue = hsv[1] > 0.01f && hsv[2] > 0.01f ? hsv[0] : Math.max(0, hue);
		// square: columns from white to the hue, darkened downwards
		for (int i = 0; i < sqW; i++) {
			int top = Draw.hsv(hue, i / (float) (sqW - 1), 1f);
			g.fillGradient(px + i, py, px + i + 1, py + sqH, c(top, alpha), c(0xFF000000, alpha));
		}
		int kx = px + Math.round(hsv[1] * (sqW - 1)), ky = py + Math.round((1 - hsv[2]) * (sqH - 1));
		Draw.roundOutline(g, kx - 3, ky - 3, 7, 7, 3.5f, 0xFFFFFFFF);
		if (clickable) hits.add(new Hit(px, py, sqW, sqH, () -> startColorDrag(col, 1, px, py, sqW, sqH), null));
		// hue bar
		int hx = px + sqW + 8, barW = 10;
		for (int i = 0; i < sqH; i++) g.fill(hx, py + i, hx + barW, py + i + 1, c(Draw.hsv(i / (float) sqH, 1, 1), alpha));
		int hy = py + Math.round(hue * (sqH - 1));
		g.fill(hx - 2, hy - 1, hx + barW + 2, hy + 1, 0xFFFFFFFF);
		if (clickable) hits.add(new Hit(hx - 2, py, barW + 4, sqH, () -> startColorDrag(col, 2, hx, py, barW, sqH), null));
		int nx = hx + barW + 8;
		if (col.alpha) {
			checker(g, nx, py, barW, sqH);
			int opaque = col.get() | 0xFF000000;
			g.fillGradient(nx, py, nx + barW, py + sqH, opaque, opaque & 0xFFFFFF);
			int ax = nx;
			int ay = py + Math.round((1 - (col.get() >>> 24) / 255f) * (sqH - 1));
			g.fill(nx - 2, ay - 1, nx + barW + 2, ay + 1, 0xFFFFFFFF);
			if (clickable) hits.add(new Hit(nx - 2, py, barW + 4, sqH, () -> startColorDrag(col, 3, ax, py, barW, sqH), null));
			nx += barW + 10;
		}
		// right side: chroma, presets, hex
		int ox = nx + 4;
		g.drawString(font, "Chroma", ox, py + 2, c(Theme.text(), alpha), false);
		toggle(g, "chroma:" + title + col.id, col.chroma(), ox + font.width("Chroma") + 8, py, 26, 13, alpha);
		if (clickable) hits.add(new Hit(ox, py - 1, font.width("Chroma") + 36, 15, () -> {
			col.setChroma(!col.chroma());
			menu.click();
		}, null));
		int sx = ox;
		for (int preset : Theme.PRESETS) {
			int pc = col.alpha ? (col.get() & 0xFF000000) | (preset & 0xFFFFFF) : preset;
			Draw.round(g, sx, py + 22, 14, 14, 3, c(pc | 0xFF000000, alpha));
			if (clickable) hits.add(new Hit(sx, py + 22, 14, 14, () -> {
				col.setChroma(false);
				col.set(pc);
				hue = -1;
				menu.click();
			}, null));
			sx += 17;
		}
		textField(g, col, ox, py + 44, 80, 16, mx, my, alpha, clickable);
	}

	private void startColorDrag(ColorSetting col, int area, int x, int y, int w, int h) {
		dragColor = col;
		dragArea = area;
		areaX = x;
		areaY = y;
		areaW = w;
		areaH = h;
		Minecraft mc = Minecraft.getInstance();
		dragColorTo(mc.mouseHandler.getScaledXPos(mc.getWindow()), mc.mouseHandler.getScaledYPos(mc.getWindow()));
	}

	private void dragColorTo(double mx, double my) {
		ColorSetting col = dragColor;
		if (col == null) return;
		float fx = (float) Math.max(0, Math.min(1, (mx - areaX) / Math.max(1, areaW - 1)));
		float fy = (float) Math.max(0, Math.min(1, (my - areaY) / Math.max(1, areaH - 1)));
		int a = col.get() & 0xFF000000;
		float[] hsv = Draw.toHsv(col.get());
		col.setChroma(false);
		switch (dragArea) {
			case 1 -> col.set(a | (Draw.hsv(hue, fx, 1 - fy) & 0xFFFFFF));
			case 2 -> {
				hue = Math.min(0.999f, fy);
				col.set(a | (Draw.hsv(hue, Math.max(hsv[1], 0.05f), Math.max(hsv[2], 0.05f)) & 0xFFFFFF));
			}
			case 3 -> col.set(Math.round((1 - fy) * 255) << 24 | (col.get() & 0xFFFFFF));
			default -> {
			}
		}
	}

	private void checker(GuiGraphics g, int x, int y, int w, int h) {
		for (int i = 0; i < w; i += 3) {
			for (int j = 0; j < h; j += 3) {
				g.fill(x + i, y + j, Math.min(x + w, x + i + 3), Math.min(y + h, y + j + 3), ((i + j) / 3 & 1) == 0 ? 0xFFCCCCCC : 0xFF888888);
			}
		}
	}

	private void textField(GuiGraphics g, Setting<?> s, int x, int y, int w, int h, int mx, int my, float alpha, boolean clickable) {
		boolean focused = editing == s || editing != null && hexTarget == s;
		Draw.round(g, x, y, w, h, 4, c(Theme.bg(), alpha));
		Draw.roundOutline(g, x, y, w, h, 4, c(focused ? Theme.accent() : Theme.line(), alpha));
		String value = focused ? editBuffer : s instanceof ColorSetting col ? col.hex() : String.valueOf(s.get());
		String shown = font.plainSubstrByWidth(value, w - 10, true);
		g.drawString(font, shown, x + 5, y + (h - 8) / 2, c(Theme.text(), alpha), false);
		if (focused && (System.currentTimeMillis() / 500) % 2 == 0) {
			int cx = x + 5 + font.width(shown);
			g.fill(cx, y + 3, cx + 1, y + h - 3, c(Theme.accent(), alpha));
		}
		if (clickable) hits.add(new Hit(x, y, w, h, () -> focus(s), null));
	}

	@SuppressWarnings("unchecked")
	private void focus(Setting<?> s) {
		commitEdit();
		listening = null;
		if (s instanceof TextSetting t) {
			editing = t;
			editBuffer = t.get();
		} else if (s instanceof ColorSetting col) {
			editing = (Setting<String>) (Setting<?>) new TextSetting("hex", "Hex", col.hex(), 9);
			hexTarget = col;
			editBuffer = col.hex();
		}
	}

	@Nullable private ColorSetting hexTarget;

	private void commitEdit() {
		if (editing == null) return;
		if (hexTarget != null) {
			Integer parsed = ColorSetting.parse(editBuffer);
			if (parsed != null) {
				hexTarget.setChroma(false);
				hexTarget.set(hexTarget.alpha || editBuffer.replace("#", "").length() == 8 ? parsed : parsed | 0xFF000000);
			}
			hexTarget = null;
		} else {
			editing.set(editBuffer);
		}
		editing = null;
	}

	// ------------------------------------------------------------------ input

	/** A click on the page ({@code button} < 0: somewhere else in the menu, which just ends typing or key listening). */
	boolean mouseClicked(double mx, double my, int button) {
		if (button < 0) {
			listening = null;
			commitEdit();
			return false;
		}
		if (listening != null) {
			// any mouse button but the left one can be a key (the left one cancels)
			if (button != 0) listening.bind(InputConstants.Type.MOUSE.getOrCreate(button));
			listening = null;
			return true;
		}
		boolean wasEditing = editing != null;
		if (editing != null) commitEdit();
		if (button != 0) confirmReset = false;
		for (int i = hits.size() - 1; i >= 0; i--) {
			Hit h = hits.get(i);
			if (!Draw.inside(mx, my, h.x(), h.y(), h.w(), h.h())) continue;
			if (button == 0 && h.left() != null) {
				if (h != resetHit) confirmReset = false;
				h.left().run();
				return true;
			}
			if (button == 1 && h.right() != null) {
				h.right().run();
				return true;
			}
			if (button == 1) continue;
		}
		confirmReset = false;
		return wasEditing;
	}

	boolean mouseDragged(double mx, double my) {
		if (dragSlider != null) {
			dragSlider.setFraction((mx - sliderX) / sliderW);
			return true;
		}
		if (dragColor != null) {
			dragColorTo(mx, my);
			return true;
		}
		return false;
	}

	void mouseReleased() {
		dragSlider = null;
		dragColor = null;
	}

	void mouseScrolled(double amount) {
		scrollTarget -= amount * 30;
	}

	boolean keyPressed(KeyEvent e) {
		if (listening != null) {
			listening.bind(e.key() == GLFW.GLFW_KEY_ESCAPE ? InputConstants.UNKNOWN : InputConstants.getKey(e));
			listening = null;
			return true;
		}
		if (editing != null) {
			switch (e.key()) {
				case GLFW.GLFW_KEY_ESCAPE -> {
					editing = null;
					hexTarget = null;
				}
				case GLFW.GLFW_KEY_ENTER, GLFW.GLFW_KEY_KP_ENTER, GLFW.GLFW_KEY_TAB -> commitEdit();
				case GLFW.GLFW_KEY_BACKSPACE -> {
					if (!editBuffer.isEmpty()) editBuffer = e.hasControlDown() ? "" : editBuffer.substring(0, editBuffer.length() - 1);
					liveText();
				}
				default -> {
					if (e.isPaste()) {
						editBuffer += Minecraft.getInstance().keyboardHandler.getClipboard().replace("\n", "");
						liveText();
					}
				}
			}
			return true;
		}
		return false;
	}

	boolean charTyped(CharacterEvent e) {
		if (editing == null || !e.isAllowedChatCharacter()) return editing != null;
		int max = hexTarget != null ? 9 : editing instanceof TextSetting t ? t.maxLength : 64;
		if (editBuffer.length() < max) editBuffer += e.codepointAsString();
		liveText();
		return true;
	}

	/** Text settings update while typing (a colour's hex waits for Enter). */
	private void liveText() {
		if (editing instanceof TextSetting t && hexTarget == null) t.set(editBuffer);
	}

	void close() {
		commitEdit();
		listening = null;
	}
}
