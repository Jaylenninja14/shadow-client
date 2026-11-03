package com.shadowclient.menu.hud;

import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.ChoiceSetting;
import java.util.List;
import java.util.Set;
import net.minecraft.client.gui.GuiGraphics;
import org.jetbrains.annotations.Nullable;

/** A text module with several lines (coordinates, pack list, …), aligned left, centre or right. */
public abstract class LinesElement extends TextElement {
	public final ChoiceSetting align;

	protected LinesElement(String id, String name, String icon, Set<Tab> tabs, boolean on, double defX, double defY) {
		super(id, name, icon, tabs, on, defX, defY);
		align = add(new ChoiceSetting("align", "Text alignment", "left", List.of("left", "center", "right"), List.of("Left", "Centre", "Right")).group("Style"));
	}

	/** Lines to show now; null or empty hides the module. */
	@Nullable
	protected abstract List<String> lines(boolean preview);

	@Override
	@Nullable
	protected String text(boolean preview) {
		List<String> l = lines(preview);
		return l == null || l.isEmpty() ? null : String.join("\n", l);
	}

	private List<String> shownLines(boolean preview) {
		List<String> l = lines(preview);
		if (l == null) return List.of();
		if (!brackets.on()) return l;
		return l.stream().map(s -> "[" + s + "]").toList();
	}

	private int pad() {
		return background.on() ? padding.i() : 1;
	}

	@Override
	public int width(boolean preview) {
		int w = 0;
		for (String s : shownLines(preview)) w = Math.max(w, font().width(s));
		return Math.max(10, w + pad() * 2);
	}

	@Override
	public int height(boolean preview) {
		int n = Math.max(1, shownLines(preview).size());
		return n * 10 - 2 + pad() * 2;
	}

	@Override
	public void render(GuiGraphics g, boolean preview) {
		int w = width(preview), h = height(preview);
		box(g, w, h);
		List<String> l = shownLines(preview);
		int y = pad();
		for (int i = 0; i < l.size(); i++) {
			String s = l.get(i);
			int tw = font().width(s);
			int x = switch (align.get()) {
				case "center" -> (w - tw) / 2;
				case "right" -> w - pad() - tw;
				default -> pad();
			};
			g.drawString(font(), s, x, y, textColor.argb(i * 0.08f), shadow.on());
			y += 10;
		}
	}
}
