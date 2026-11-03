package com.shadowclient.menu.hud;

import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ColorSetting;
import com.shadowclient.menu.setting.NumberSetting;
import com.shadowclient.menu.ui.Draw;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.util.Mth;

/** A compass strip: N, NE, E, … slide past a centre marker as you turn. */
final class Compass extends HudElement {
	final NumberSetting stripWidth = add(new NumberSetting("width", "Width", 140, 80, 260, 10));
	final BoolSetting degrees = add(new BoolSetting("degrees", "Show degrees", true));
	final BoolSetting background = add(new BoolSetting("background", "Background", true).group("Style"));
	final ColorSetting backgroundColor = add(new ColorSetting("background-color", "Background colour", 0x6F000000, true).group("Style").showIf(background::on));
	final ColorSetting textColor = add(new ColorSetting("text-color", "Text colour", 0xFFFFFFFF, false).group("Style"));
	final ColorSetting markerColor = add(new ColorSetting("marker-color", "Marker colour", 0xFF9B7BFF, false).group("Style"));
	final BoolSetting shadow = add(new BoolSetting("shadow", "Text shadow", true).group("Style"));
	private static final String[] NAMES = {"N", "NE", "E", "SE", "S", "SW", "W", "NW"};

	Compass() {
		super("direction", "Direction", "compass", InfoElements.HUD, false, 0.5, 0.05);
		describe("A compass strip at the top of the screen.");
		anchor(Anchor.TOP_CENTER);
	}

	@Override
	public int width(boolean preview) {
		return stripWidth.i();
	}

	@Override
	public int height(boolean preview) {
		return degrees.on() ? 24 : 15;
	}

	@Override
	public void render(GuiGraphics g, boolean preview) {
		int w = width(preview);
		if (background.on()) Draw.round(g, 0, 0, w, 15, 3, backgroundColor.argb());
		// Minecraft's yaw is 0 facing south; headings here start at north
		float heading = mc.player == null ? 0 : Mth.wrapDegrees(mc.player.getYRot() + 180);
		if (heading < 0) heading += 360;
		float pxPerDeg = w / 180f;
		var font = font();
		for (int deg = 0; deg < 360; deg += 15) {
			float rel = Mth.wrapDegrees(deg - heading);
			if (Math.abs(rel) > 88) continue;
			float x = w / 2f + rel * pxPerDeg;
			float edge = 1 - Math.abs(rel) / 90f;
			int c = Draw.fade(textColor.argb(), 0.25f + 0.75f * edge);
			if (deg % 45 == 0) {
				String n = NAMES[deg / 45];
				g.drawString(font, n, Math.round(x - font.width(n) / 2f), 4, deg % 90 == 0 ? c : Draw.fade(c, 0.8f), shadow.on());
			} else {
				g.fill(Math.round(x), 6, Math.round(x) + 1, 9, Draw.fade(c, 0.6f));
			}
		}
		g.fill(w / 2, 0, w / 2 + 1, 3, markerColor.argb());
		g.fill(w / 2, 12, w / 2 + 1, 15, markerColor.argb());
		if (degrees.on()) {
			String d = Math.round(heading) % 360 + "°";
			g.drawString(font, d, (w - font.width(d)) / 2, 16, textColor.argb(), shadow.on());
		}
	}
}
