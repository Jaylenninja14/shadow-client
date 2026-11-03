package com.shadowclient.menu.ui;

import com.shadowclient.menu.hud.HudElement;
import com.shadowclient.menu.hud.HudManager;
import com.shadowclient.menu.module.MenuConfig;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.network.chat.Component;
import net.minecraft.sounds.SoundEvents;
import org.jetbrains.annotations.Nullable;
import org.lwjgl.glfw.GLFW;

/**
 * Drag HUD modules where you want them: drag to move (snaps to edges, the centre and other modules;
 * hold Shift to place freely), drag the corner or scroll to resize, right-click for the module's
 * settings, arrow keys to nudge, Delete to hide. Positions are saved as you go.
 */
public final class HudEditorScreen extends Screen {
	private static final int SNAP = 5, MARGIN = 2, HANDLE = 5;
	@Nullable private final Screen parent;
	@Nullable private HudElement selected, dragging, resizing;
	private double dragDX, dragDY;
	private final List<int[]> guides = new ArrayList<>(); // {vertical?1:0, position}
	private final Anim anim = new Anim();
	private final List<Button> buttons = new ArrayList<>();

	private record Button(String label, int x, int y, int w, Runnable action) {
	}

	public HudEditorScreen(@Nullable Screen parent) {
		super(Component.literal("Edit HUD Layout"));
		this.parent = parent;
	}

	/** Open the editor from a settings page (comes back to it when done). */
	public static void open(Screen from) {
		Minecraft.getInstance().setScreen(new HudEditorScreen(from));
	}

	@Override
	public boolean isPauseScreen() {
		return false;
	}

	@Override
	public void onClose() {
		MenuConfig.flush(true);
		minecraft.setScreen(parent);
	}

	private List<HudElement> shown() {
		List<HudElement> out = new ArrayList<>();
		for (HudElement e : HudManager.all()) if (e.enabled.on()) out.add(e);
		return out;
	}

	private void layoutButtons() {
		buttons.clear();
		String[] labels = {"Mod Menu", "Snap: " + (HudManager.SNAP.on() ? "On" : "Off"), "Grid: " + (HudManager.GRID.on() ? "On" : "Off"), "Reset All", "Done"};
		Runnable[] actions = {
				() -> minecraft.setScreen(ModMenuScreen.hudTab(this)),
				HudManager.SNAP::toggle,
				HudManager.GRID::toggle,
				this::resetAll,
				this::onClose,
		};
		int total = -4;
		int[] widths = new int[labels.length];
		for (int i = 0; i < labels.length; i++) {
			widths[i] = font.width(labels[i]) + 14;
			total += widths[i] + 4;
		}
		int x = (width - total) / 2, y = height / 2 - 9;
		for (int i = 0; i < labels.length; i++) {
			buttons.add(new Button(labels[i], x, y, widths[i], actions[i]));
			x += widths[i] + 4;
		}
	}

	private void resetAll() {
		for (HudElement e : HudManager.all()) e.resetPlace();
	}

	@Override
	public void renderBackground(GuiGraphics g, int mx, int my, float partial) {
		g.fill(0, 0, width, height, minecraft.level == null ? 0xFF101016 : 0x40000000);
	}

	@Override
	public void render(GuiGraphics g, int mx, int my, float partial) {
		anim.frame();
		HudManager.layout(width, height, true);
		layoutButtons();
		int accent = Theme.accent();
		if (HudManager.GRID.on()) {
			for (int x = 0; x < width; x += 10) g.fill(x, 0, x + 1, height, x % 50 == 0 ? 0x22FFFFFF : 0x0FFFFFFF);
			for (int y = 0; y < height; y += 10) g.fill(0, y, width, y + 1, y % 50 == 0 ? 0x22FFFFFF : 0x0FFFFFFF);
		}
		// centre lines, faint
		g.fill(width / 2, 0, width / 2 + 1, height, 0x14FFFFFF);
		g.fill(0, height / 2, width, height / 2 + 1, 0x14FFFFFF);

		HudElement hover = elementAt(mx, my);
		for (HudElement e : shown()) {
			try {
				e.draw(g, width, height, true);
			} catch (RuntimeException ignored) {
				// drawn as a box only
			}
		}
		g.nextStratum();
		for (HudElement e : shown()) {
			int l = e.left(width, true), t = e.top(height, true), w = e.scaledWidth(true), h = e.scaledHeight(true);
			boolean sel = e == selected, hov = e == hover;
			float a = anim.get("hl:" + e.id, sel ? 1 : hov ? 0.6f : 0.25f, 14);
			int c = sel ? accent : 0xFFFFFFFF;
			g.fill(l - 1, t - 1, l + w + 1, t + h + 1, Draw.fade(c, a * 0.12f));
			g.renderOutline(l - 1, t - 1, w + 2, h + 2, Draw.fade(c, 0.35f + a * 0.65f));
			if (sel || hov) {
				g.fill(l + w - HANDLE + 1, t + h - HANDLE + 1, l + w + 1, t + h + 1, Draw.fade(c, 0.9f));
				String label = e.name + (sel ? "  " + Math.round(e.scale() * 100) + "%" : "");
				int lw = font.width(label) + 6;
				int ly = t - 12 < 0 ? t + h + 3 : t - 12;
				int lx = Math.max(1, Math.min(width - lw - 1, l));
				Draw.round(g, lx, ly, lw, 11, 2, Draw.fade(sel ? accent : 0xFF202028, 0.92f));
				g.drawString(font, label, lx + 3, ly + 2, sel ? Theme.onAccent() : 0xFFFFFFFF, false);
			}
		}
		for (int[] guide : guides) {
			if (guide[0] == 1) g.fill(guide[1], 0, guide[1] + 1, height, Draw.fade(accent, 0.85f));
			else g.fill(0, guide[1], width, guide[1] + 1, Draw.fade(accent, 0.85f));
		}

		// toolbar, hidden while dragging
		float bar = anim.get("toolbar", dragging == null && resizing == null ? 1 : 0, 16);
		if (bar > 0.02f) {
			// a panel behind the buttons and the tip, so chat or modules under it don't show through
			String tip = width < 520 ? "Drag · corner/scroll: size · right-click: settings" : "Drag to move · corner or scroll to resize · right-click for settings · Shift: no snapping · Delete: hide";
			int first = buttons.get(0).x(), last = buttons.get(buttons.size() - 1).x() + buttons.get(buttons.size() - 1).w();
			int panelW = Math.max(last - first, font.width(tip)) + 16;
			g.nextStratum();
			Draw.round(g, (width - panelW) / 2f, height / 2f - 15, panelW, 44, 8, Draw.fade(Theme.bg(), bar * 0.88f));
			for (Button b : buttons) {
				boolean over = Draw.inside(mx, my, b.x(), b.y(), b.w(), 18);
				boolean primary = b.label().equals("Done");
				int bg = primary ? accent : over ? Theme.chipHover() : Theme.panel();
				Draw.round(g, b.x(), b.y(), b.w(), 18, 4, Draw.fade(bg, bar * 0.95f));
				if (!primary) Draw.roundOutline(g, b.x(), b.y(), b.w(), 18, 4, Draw.fade(Theme.line(), bar));
				g.drawCenteredString(font, b.label(), b.x() + b.w() / 2, b.y() + 5, Draw.fade(primary ? Theme.onAccent() : Theme.text(), Math.max(0.05f, bar)));
			}
			g.drawCenteredString(font, tip, width / 2, height / 2 + 14, Draw.fade(0xFFD0D0DA, Math.max(0.05f, bar * 0.8f)));
		}
	}

	@Nullable
	private HudElement elementAt(double mx, double my) {
		List<HudElement> list = shown();
		for (int i = list.size() - 1; i >= 0; i--) {
			HudElement e = list.get(i);
			if (Draw.inside(mx, my, e.left(width, true) - 1, e.top(height, true) - 1, e.scaledWidth(true) + 2, e.scaledHeight(true) + 2)) return e;
		}
		return null;
	}

	private void click() {
		if (Theme.SOUNDS.on()) minecraft.getSoundManager().play(SimpleSoundInstance.forUI(SoundEvents.UI_BUTTON_CLICK, 1.0F));
	}

	@Override
	public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) {
		double mx = event.x(), my = event.y();
		if (event.button() == 0) {
			for (Button b : buttons) {
				if (Draw.inside(mx, my, b.x(), b.y(), b.w(), 18)) {
					click();
					b.action().run();
					return true;
				}
			}
		}
		HudElement e = elementAt(mx, my);
		if (event.button() == 1) {
			if (e != null) {
				click();
				minecraft.setScreen(ModMenuScreen.page(this, e.card()));
			}
			return true;
		}
		if (event.button() != 0) return false;
		selected = e;
		if (e == null) return true;
		int l = e.left(width, true), t = e.top(height, true);
		if (mx >= l + e.scaledWidth(true) - HANDLE && my >= t + e.scaledHeight(true) - HANDLE) {
			resizing = e;
		} else {
			dragging = e;
			dragDX = mx - l;
			dragDY = my - t;
		}
		return true;
	}

	@Override
	public boolean mouseDragged(MouseButtonEvent event, double dx, double dy) {
		double mx = event.x(), my = event.y();
		boolean free = event.hasShiftDown() || !HudManager.SNAP.on();
		if (resizing != null) {
			HudElement e = resizing;
			int l = e.left(width, true), t = e.top(height, true);
			double sx = (mx - l) / Math.max(1, e.width(true)), sy = (my - t) / Math.max(1, e.height(true));
			e.scale.set(Math.max(sx, sy));
			e.moveTo(l, t, width, height, true);
			return true;
		}
		if (dragging == null) return false;
		HudElement e = dragging;
		int w = e.scaledWidth(true), h = e.scaledHeight(true);
		int l = (int) Math.round(mx - dragDX), t = (int) Math.round(my - dragDY);
		guides.clear();
		if (!free) {
			l = snap(l, w, width, e, true);
			t = snap(t, h, height, e, false);
		}
		l = Math.max(0, Math.min(width - w, l));
		t = Math.max(0, Math.min(height - h, t));
		e.moveTo(l, t, width, height, true);
		return true;
	}

	/** Snap one axis: the module's start, centre or end to screen edges, the centre or another module. */
	private int snap(int pos, int size, int screen, HudElement moving, boolean vertical) {
		List<Integer> targets = new ArrayList<>(List.of(MARGIN, screen / 2, screen - MARGIN));
		for (HudElement o : shown()) {
			if (o == moving) continue;
			int a = vertical ? o.left(width, true) : o.top(height, true);
			int s = vertical ? o.scaledWidth(true) : o.scaledHeight(true);
			targets.add(a);
			targets.add(a + s / 2);
			targets.add(a + s);
			targets.add(a - 2); // a small gap before or after the other module
			targets.add(a + s + 2);
		}
		int best = Integer.MAX_VALUE, bestPos = pos, line = -1;
		int[] anchors = {0, size / 2, size};
		for (int anchor : anchors) {
			for (int target : targets) {
				int d = Math.abs(pos + anchor - target);
				if (d < best && d <= SNAP) {
					best = d;
					bestPos = target - anchor;
					line = target;
				}
			}
		}
		if (line >= 0) guides.add(new int[] {vertical ? 1 : 0, line});
		return bestPos;
	}

	@Override
	public boolean mouseReleased(MouseButtonEvent event) {
		dragging = null;
		resizing = null;
		guides.clear();
		return true;
	}

	@Override
	public boolean mouseScrolled(double mx, double my, double scrollX, double scrollY) {
		HudElement e = elementAt(mx, my);
		if (e == null) e = selected;
		if (e == null) return false;
		int l = e.left(width, true), t = e.top(height, true);
		e.scale.set(e.scale.get() + Math.signum(scrollY) * 0.05);
		e.moveTo(l, t, width, height, true);
		return true;
	}

	@Override
	public boolean keyPressed(KeyEvent event) {
		if (selected != null) {
			int step = event.hasShiftDown() ? 10 : 1;
			int l = selected.left(width, true), t = selected.top(height, true);
			switch (event.key()) {
				case GLFW.GLFW_KEY_LEFT -> l -= step;
				case GLFW.GLFW_KEY_RIGHT -> l += step;
				case GLFW.GLFW_KEY_UP -> t -= step;
				case GLFW.GLFW_KEY_DOWN -> t += step;
				case GLFW.GLFW_KEY_DELETE, GLFW.GLFW_KEY_BACKSPACE -> {
					selected.enabled.set(false);
					selected = null;
					return true;
				}
				case GLFW.GLFW_KEY_R -> {
					selected.resetPlace();
					return true;
				}
				default -> {
					return super.keyPressed(event);
				}
			}
			selected.moveTo(Math.max(0, l), Math.max(0, t), width, height, true);
			return true;
		}
		return super.keyPressed(event);
	}
}
