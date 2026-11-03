package com.shadowclient.menu.feature;

import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ChoiceSetting;
import com.shadowclient.menu.setting.ColorSetting;
import com.shadowclient.menu.setting.NumberSetting;
import com.shadowclient.menu.ui.Draw;
import java.util.EnumSet;
import java.util.List;
import net.minecraft.client.AttackIndicatorStatus;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.level.GameType;

/** Your own crosshair: shape, size, gap, thickness, colour and an outline; red over players and mobs if you like. */
public final class Crosshair extends Feature {
	public static final Crosshair I = new Crosshair();
	final ChoiceSetting style = add(new ChoiceSetting("style", "Shape", "cross", List.of("cross", "plus", "t", "dot", "circle", "square", "x"),
			List.of("Cross", "Plus", "T", "Dot", "Circle", "Square", "X")));
	final NumberSetting size = add(new NumberSetting("size", "Size", 5, 1, 15, 1));
	final NumberSetting gap = add(new NumberSetting("gap", "Gap", 2, 0, 8, 1).showIf(() -> style.is("cross") || style.is("t") || style.is("x")));
	final NumberSetting thickness = add(new NumberSetting("thickness", "Thickness", 1, 1, 4, 1));
	final BoolSetting dot = add(new BoolSetting("dot", "Centre dot", false).showIf(() -> !style.is("dot")));
	final ColorSetting color = add(new ColorSetting("color", "Colour", 0xFFFFFFFF, true).group("Colour"));
	final BoolSetting outline = add(new BoolSetting("outline", "Black outline", true).group("Colour"));
	final BoolSetting targetColor = add(new BoolSetting("target", "Different colour over a target", false).group("Colour"));
	final ColorSetting target = add(new ColorSetting("target-color", "Target colour", 0xFFFF4040, true).group("Colour").showIf(targetColor::on));
	final BoolSetting thirdPerson = add(new BoolSetting("third-person", "Show in third person", false));

	private Crosshair() {
		super("crosshair", "Custom Crosshair", "crosshair", EnumSet.of(Tab.PVP, Tab.VISUAL, Tab.HUD), false);
		describe("Your own crosshair shape and colour.");
	}

	/** Draw the custom crosshair; false lets Minecraft draw its own. */
	public static boolean render(GuiGraphics g, DeltaTracker delta) {
		Crosshair c = I;
		if (!c.on()) return false;
		Minecraft mc = Minecraft.getInstance();
		if (mc.player == null || mc.gameMode == null) return false;
		if (mc.debugEntries.isOverlayVisible()) return false; // F3 shows its 3D axes
		if (!mc.options.getCameraType().isFirstPerson() && !c.thirdPerson.on()) return true;
		if (mc.gameMode.getPlayerMode() == GameType.SPECTATOR) return false;
		g.nextStratum();
		int cx = g.guiWidth() / 2, cy = g.guiHeight() / 2;
		int col = c.targetColor.on() && mc.crosshairPickEntity instanceof LivingEntity ? c.target.argb() : c.color.argb();
		if (c.outline.on()) c.shape(g, cx, cy, 0xC0000000, 1);
		c.shape(g, cx, cy, col, 0);
		if (mc.options.attackIndicator().get() == AttackIndicatorStatus.CROSSHAIR) {
			float f = mc.player.getAttackStrengthScale(0);
			if (f < 1) {
				int w = 16, y = cy + 9;
				g.fill(cx - w / 2, y, cx + w / 2, y + 2, 0x80000000);
				g.fill(cx - w / 2, y, cx - w / 2 + Math.round(w * f), y + 2, Draw.fade(col, 0.9f));
			}
		}
		return true;
	}

	/** One pass of the shape; {@code grow} > 0 draws it fatter (the outline behind it). */
	private void shape(GuiGraphics g, int cx, int cy, int col, int grow) {
		int s = size.i(), t = thickness.i(), gp = gap.i();
		int h0 = t / 2, h1 = t - h0; // thickness either side of the centre line
		switch (style.get()) {
			case "dot" -> rect(g, cx - Math.max(1, s / 3) - grow, cy - Math.max(1, s / 3) - grow, cx + Math.max(1, s / 3) + grow, cy + Math.max(1, s / 3) + grow, col);
			case "circle" -> {
				Draw.roundOutline(g, cx - s - grow - 0.5f, cy - s - grow - 0.5f, (s + grow) * 2 + 1, (s + grow) * 2 + 1, s + grow, col);
				if (grow > 0) Draw.roundOutline(g, cx - s + 0.5f, cy - s + 0.5f, s * 2 - 1, s * 2 - 1, s, col);
			}
			case "square" -> {
				int r = s + grow;
				g.renderOutline(cx - r, cy - r, r * 2 + 1, r * 2 + 1, col);
				if (grow > 0) g.renderOutline(cx - s + 1, cy - s + 1, s * 2 - 1, s * 2 - 1, col);
			}
			case "x" -> {
				for (int i = gp; i <= gp + s; i++) {
					for (int k = -grow; k <= grow + t - 1; k++) {
						rect(g, cx + i + k, cy + i, cx + i + k + 1, cy + i + 1, col);
						rect(g, cx - i + k, cy + i, cx - i + k + 1, cy + i + 1, col);
						rect(g, cx + i + k, cy - i, cx + i + k + 1, cy - i + 1, col);
						rect(g, cx - i + k, cy - i, cx - i + k + 1, cy - i + 1, col);
					}
				}
			}
			default -> {
				int gapUsed = style.is("plus") ? 0 : gp;
				// left, right, bottom, top arms
				rect(g, cx - gapUsed - s - grow, cy - h0 - grow, cx - gapUsed + grow, cy + h1 + grow, col);
				rect(g, cx + gapUsed + 1 - grow, cy - h0 - grow, cx + gapUsed + 1 + s + grow, cy + h1 + grow, col);
				rect(g, cx - h0 - grow, cy + gapUsed + 1 - grow, cx + h1 + grow, cy + gapUsed + 1 + s + grow, col);
				if (!style.is("t")) rect(g, cx - h0 - grow, cy - gapUsed - s - grow, cx + h1 + grow, cy - gapUsed + grow, col);
			}
		}
		if (dot.on() && !style.is("dot")) rect(g, cx - h0 - grow, cy - h0 - grow, cx + h1 + grow, cy + h1 + grow, col);
	}

	private static void rect(GuiGraphics g, int x1, int y1, int x2, int y2, int col) {
		g.fill(x1, y1, x2, y2, col);
	}
}
