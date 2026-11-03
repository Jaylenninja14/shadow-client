package com.shadowclient.menu.ui;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.resources.Identifier;
import net.minecraft.util.Mth;
import net.minecraft.world.item.ItemStack;

/**
 * Drawing helpers for the menu and HUD: smooth rounded boxes (drawn at the screen's real pixel
 * size, with soft edges), icons, items and colour maths. Colours are ARGB.
 */
public final class Draw {
	private Draw() {
	}

	/**
	 * How many sub-pixels per GUI pixel corners are drawn with: the screen's real pixels, but at most
	 * 2 (soft edges hide the rest, and a full-HD GUI scale of 4 would mean 4x the fill calls).
	 */
	private static float guiScale() {
		return (float) Math.max(1, Math.min(2, Minecraft.getInstance().getWindow().getGuiScale()));
	}

	/** Filled box with ~3px rounded corners (the menu's small chips). */
	public static void box(GuiGraphics g, int x, int y, int w, int h, int color) {
		round(g, x, y, w, h, 3, color);
	}

	/** Rounded box with a 1px outline. */
	public static void outlined(GuiGraphics g, int x, int y, int w, int h, int fill, int outline) {
		round(g, x, y, w, h, 3, outline);
		round(g, x + 1, y + 1, w - 2, h - 2, 2, fill);
	}

	/** Filled box with rounded corners of radius {@code r} (GUI pixels). */
	public static void round(GuiGraphics g, float x, float y, float w, float h, float r, int color) {
		if (w <= 0 || h <= 0 || (color >>> 24) == 0) return;
		float k = guiScale();
		r = Math.min(r, Math.min(w, h) / 2f);
		if (r < 0.5f) {
			g.pose().pushMatrix();
			g.pose().scale(1 / k, 1 / k);
			g.fill(Math.round(x * k), Math.round(y * k), Math.round((x + w) * k), Math.round((y + h) * k), color);
			g.pose().popMatrix();
			return;
		}
		// sub-pixels (see guiScale), so corners are smoother than whole GUI pixels allow
		int px = Math.round(x * k), py = Math.round(y * k), pw = Math.round(w * k), ph = Math.round(h * k);
		float pr = r * k;
		int rows = (int) Math.ceil(pr);
		g.pose().pushMatrix();
		g.pose().scale(1 / k, 1 / k);
		g.fill(px, py + rows, px + pw, py + ph - rows, color);
		int alpha = color >>> 24, rgb = color & 0xFFFFFF;
		for (int i = 0; i < rows; i++) {
			// distance of this pixel row's centre from the corner circle's centre
			float dy = pr - i - 0.5f;
			float inset = pr - (float) Math.sqrt(Math.max(0, pr * pr - dy * dy));
			int full = (int) Math.ceil(inset);
			int top = py + i, bottom = py + ph - 1 - i;
			g.fill(px + full, top, px + pw - full, top + 1, color);
			g.fill(px + full, bottom, px + pw - full, bottom + 1, color);
			float cover = full - inset;
			if (cover > 0.05f && full > 0) {
				int edge = (Math.round(alpha * cover) << 24) | rgb;
				g.fill(px + full - 1, top, px + full, top + 1, edge);
				g.fill(px + pw - full, top, px + pw - full + 1, top + 1, edge);
				g.fill(px + full - 1, bottom, px + full, bottom + 1, edge);
				g.fill(px + pw - full, bottom, px + pw - full + 1, bottom + 1, edge);
			}
		}
		g.pose().popMatrix();
	}

	/** 1px rounded outline. */
	public static void roundOutline(GuiGraphics g, float x, float y, float w, float h, float r, int color) {
		if (w <= 2 || h <= 2) return;
		float k = guiScale();
		int px = Math.round(x * k), py = Math.round(y * k), pw = Math.round(w * k), ph = Math.round(h * k);
		int t = Math.max(1, Math.round(k * 0.75f));
		float pr = Math.min(r * k, Math.min(pw, ph) / 2f);
		int rows = (int) Math.ceil(pr);
		g.pose().pushMatrix();
		g.pose().scale(1 / k, 1 / k);
		g.fill(px + rows, py, px + pw - rows, py + t, color);
		g.fill(px + rows, py + ph - t, px + pw - rows, py + ph, color);
		g.fill(px, py + rows, px + t, py + ph - rows, color);
		g.fill(px + pw - t, py + rows, px + pw, py + ph - rows, color);
		for (int i = 0; i < rows; i++) {
			float dy = pr - i - 0.5f;
			int outer = (int) Math.floor(pr - Math.sqrt(Math.max(0, pr * pr - dy * dy)));
			float dyIn = dy;
			float rin = Math.max(0, pr - t);
			int inner = dyIn >= rin ? rows : (int) Math.ceil(pr - Math.sqrt(Math.max(0, rin * rin - dyIn * dyIn)));
			inner = Math.max(inner, outer + 1);
			int top = py + i, bottom = py + ph - 1 - i;
			g.fill(px + outer, top, px + Math.min(inner, rows), top + 1, color);
			g.fill(px + pw - Math.min(inner, rows), top, px + pw - outer, top + 1, color);
			g.fill(px + outer, bottom, px + Math.min(inner, rows), bottom + 1, color);
			g.fill(px + pw - Math.min(inner, rows), bottom, px + pw - outer, bottom + 1, color);
		}
		g.pose().popMatrix();
	}

	/** A filled circle. */
	public static void circle(GuiGraphics g, float cx, float cy, float radius, int color) {
		round(g, cx - radius, cy - radius, radius * 2, radius * 2, radius, color);
	}

	/** Soft shadow around a box (a few widening, fading rounded boxes). */
	public static void shadow(GuiGraphics g, float x, float y, float w, float h, float r, int size, int alpha) {
		for (int i = size; i >= 1; i--) {
			int a = Math.round(alpha * (1 - i / (float) (size + 1)) / size * 2);
			round(g, x - i, y - i + 1, w + i * 2, h + i * 2, r + i, Math.min(255, a) << 24);
		}
	}

	/** Draw one of the 64x64 menu icons at {@code size} pixels, tinted. */
	public static void icon(GuiGraphics g, String name, float x, float y, float size, int color) {
		Identifier id = Identifier.fromNamespaceAndPath("shadowmenu", "textures/gui/icon/" + name + ".png");
		g.pose().pushMatrix();
		g.pose().translate(x, y);
		g.pose().scale(size / 64f, size / 64f);
		g.blit(RenderPipelines.GUI_TEXTURED, id, 0, 0, 0, 0, 64, 64, 64, 64, color);
		g.pose().popMatrix();
	}

	/** Draw an item at {@code size} pixels (items are 16x16). */
	public static void item(GuiGraphics g, ItemStack stack, float x, float y, float size) {
		g.pose().pushMatrix();
		g.pose().translate(x, y);
		g.pose().scale(size / 16f, size / 16f);
		g.renderFakeItem(stack, 0, 0);
		g.pose().popMatrix();
	}

	/** Text that shrinks (down to 70%) to fit {@code maxW}, and is cut only if it still doesn't. */
	public static void fitText(GuiGraphics g, Font font, String text, int x, int y, int maxW, int color) {
		int w = font.width(text);
		if (w <= maxW) {
			g.drawString(font, text, x, y, color, false);
			return;
		}
		float s = Math.max(0.7f, maxW / (float) w);
		String shown = w * s <= maxW ? text : font.plainSubstrByWidth(text, (int) (maxW / s));
		g.pose().pushMatrix();
		g.pose().translate(x, y + (1 - s) * 4);
		g.pose().scale(s, s);
		g.drawString(font, shown, 0, 0, color, false);
		g.pose().popMatrix();
	}

	/** Centred version of {@link #fitText}. */
	public static void fitCentered(GuiGraphics g, Font font, String text, int cx, int y, int maxW, int color) {
		int w = Math.min(maxW, font.width(text));
		float s = font.width(text) <= maxW ? 1 : Math.max(0.7f, maxW / (float) font.width(text));
		fitText(g, font, text, cx - Math.round(Math.min(w, font.width(text) * s)) / 2, y, maxW, color);
	}

	/** Text drawn at a fractional scale. */
	public static void scaledText(GuiGraphics g, Font font, String text, float x, float y, float scale, int color, boolean shadow) {
		g.pose().pushMatrix();
		g.pose().translate(x, y);
		g.pose().scale(scale, scale);
		g.drawString(font, text, 0, 0, color, shadow);
		g.pose().popMatrix();
	}

	public static boolean inside(double mx, double my, double x, double y, double w, double h) {
		return mx >= x && my >= y && mx < x + w && my < y + h;
	}

	// ------------------------------------------------------------------ colours

	/** {@code a} → {@code b} by {@code t} (0..1), alpha included. */
	public static int mix(int a, int b, float t) {
		t = Mth.clamp(t, 0, 1);
		int aa = a >>> 24, ar = a >> 16 & 255, ag = a >> 8 & 255, ab = a & 255;
		int ba = b >>> 24, br = b >> 16 & 255, bg = b >> 8 & 255, bb = b & 255;
		return Math.round(aa + (ba - aa) * t) << 24 | Math.round(ar + (br - ar) * t) << 16 | Math.round(ag + (bg - ag) * t) << 8 | Math.round(ab + (bb - ab) * t);
	}

	/** Multiply a colour's alpha by {@code f}. */
	public static int fade(int color, float f) {
		int a = Math.round((color >>> 24) * Mth.clamp(f, 0, 1));
		return a << 24 | (color & 0xFFFFFF);
	}

	public static int withAlpha(int color, int alpha) {
		return Mth.clamp(alpha, 0, 255) << 24 | (color & 0xFFFFFF);
	}

	/** Perceived brightness 0..1, to pick dark or light text on a colour. */
	public static float luma(int color) {
		return ((color >> 16 & 255) * 0.299f + (color >> 8 & 255) * 0.587f + (color & 255) * 0.114f) / 255f;
	}

	public static int hsv(float h, float s, float v) {
		return 0xFF000000 | (java.awt.Color.HSBtoRGB(h, s, v) & 0xFFFFFF);
	}

	/** {h, s, v} of a colour. */
	public static float[] toHsv(int color) {
		return java.awt.Color.RGBtoHSB(color >> 16 & 255, color >> 8 & 255, color & 255, null);
	}

	/** Eases 0..1 progress (fast start, gentle stop). */
	public static float ease(float t) {
		t = Mth.clamp(t, 0, 1);
		return 1 - (1 - t) * (1 - t) * (1 - t);
	}
}
