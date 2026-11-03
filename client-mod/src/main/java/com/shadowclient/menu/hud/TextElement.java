package com.shadowclient.menu.hud;

import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ChoiceSetting;
import com.shadowclient.menu.setting.ColorSetting;
import com.shadowclient.menu.setting.NumberSetting;
import com.shadowclient.menu.ui.Draw;
import java.util.List;
import java.util.Set;
import net.minecraft.client.gui.GuiGraphics;
import org.jetbrains.annotations.Nullable;

/**
 * A HUD module that shows one line of text in a box (FPS, ping, coordinates, …), with the style
 * options every such module shares: colours, background, border, text shadow and [brackets].
 */
public abstract class TextElement extends HudElement {
	private static final String STYLE = "Style";
	public final ColorSetting textColor;
	public final BoolSetting shadow;
	public final BoolSetting background;
	public final ColorSetting backgroundColor;
	public final BoolSetting border;
	public final ColorSetting borderColor;
	public final BoolSetting brackets;
	public final ChoiceSetting corners;
	public final BoolSetting fixedSize;
	public final NumberSetting padding;

	protected TextElement(String id, String name, String icon, Set<Tab> tabs, boolean on, double defX, double defY) {
		super(id, name, icon, tabs, on, defX, defY);
		textColor = add(new ColorSetting("text-color", "Text colour", 0xFFFFFFFF, false).group(STYLE));
		shadow = add(new BoolSetting("shadow", "Text shadow", true).group(STYLE));
		brackets = add(new BoolSetting("brackets", "Brackets", false).group(STYLE));
		background = add(new BoolSetting("background", "Background", true).group(STYLE));
		backgroundColor = add(new ColorSetting("background-color", "Background colour", 0x6F000000, true).group(STYLE).showIf(this::backgroundOn));
		corners = add(new ChoiceSetting("corners", "Corners", "rounded", List.of("square", "rounded"), List.of("Square", "Rounded")).group(STYLE)
				.showIf(this::backgroundOn));
		fixedSize = add(new BoolSetting("fixed-size", "Same size boxes", true).group(STYLE).showIf(this::backgroundOn));
		padding = add(new NumberSetting("padding", "Padding", 4, 0, 10, 1).group(STYLE).showIf(this::backgroundOn));
		border = add(new BoolSetting("border", "Border", false).group(STYLE));
		borderColor = add(new ColorSetting("border-color", "Border colour", 0xFF9B7BFF, false).group(STYLE).showIf(border::on));
	}

	private boolean backgroundOn() {
		return background != null && background.on();
	}

	/** The text now; null hides the element ({@code preview}: always something, for the editor). */
	@Nullable
	protected abstract String text(boolean preview);

	protected String shown(boolean preview) {
		String t = text(preview);
		if (t == null) t = "";
		return brackets.on() ? "[" + t + "]" : t;
	}

	@Override
	public boolean hasContent() {
		try {
			return text(false) != null;
		} catch (RuntimeException e) {
			return false;
		}
	}

	private int pad() {
		return background.on() ? padding.i() : 1;
	}

	@Override
	public int width(boolean preview) {
		int w = font().width(shown(preview)) + pad() * 2;
		return background.on() && fixedSize.on() ? Math.max(56, w) : w;
	}

	@Override
	public int height(boolean preview) {
		int h = 8 + pad() * 2;
		return background.on() && fixedSize.on() ? Math.max(18, h) : h;
	}

	@Override
	public void render(GuiGraphics g, boolean preview) {
		String t = shown(preview);
		int w = width(preview), h = height(preview);
		box(g, w, h);
		int tw = font().width(t);
		g.drawString(font(), t, (w - tw) / 2, (h - 8) / 2, textColor.argb(), shadow.on());
	}

	/** The background and border every text module draws. */
	protected void box(GuiGraphics g, int w, int h) {
		boolean round = corners.is("rounded");
		if (background.on()) {
			if (round) Draw.round(g, 0, 0, w, h, 3, backgroundColor.argb());
			else g.fill(0, 0, w, h, backgroundColor.argb());
		}
		if (border.on()) {
			int c = borderColor.argb(0.3f);
			if (round && background.on()) Draw.roundOutline(g, 0, 0, w, h, 3, c);
			else g.renderOutline(0, 0, w, h, c);
		}
	}
}
