package com.shadowclient.menu.hud;

import com.shadowclient.menu.module.Module;
import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.ActionSetting;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.NumberSetting;
import com.shadowclient.menu.setting.Setting;
import com.shadowclient.menu.setting.Settings;
import com.shadowclient.menu.ui.HudEditorScreen;
import java.util.Set;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.world.item.Item;
import org.jetbrains.annotations.Nullable;

/**
 * One movable HUD module drawn by Shadow Menu. Its position is stored as a fraction of the free
 * space (0 = left/top edge, 1 = right/bottom edge), so it stays glued to the same edge or centre
 * when the window or GUI scale changes. Position and scale are set in the HUD editor.
 */
public abstract class HudElement {
	/** Where a module goes until it's dragged: stacked in a screen corner (NONE: its own x/y). */
	public enum Anchor { NONE, TOP_LEFT, TOP_RIGHT, BOTTOM_RIGHT, TOP_CENTER }

	public final String id;
	public final String name;
	public final String icon;
	public final Set<Tab> tabs;
	public final Settings settings;
	public final BoolSetting enabled;
	public final NumberSetting x;
	public final NumberSetting y;
	public final NumberSetting scale;
	/** Dragged in the editor (otherwise stacked by its anchor). */
	public final BoolSetting placed;
	Anchor anchor = Anchor.NONE;
	/** Stacked position, worked out by {@link HudManager#layout} every frame. */
	int autoLeft = -1, autoTop = -1;
	@Nullable private String description;
	@Nullable private Item item;
	protected final Minecraft mc = Minecraft.getInstance();

	protected HudElement(String id, String name, String icon, Set<Tab> tabs, boolean on, double defX, double defY) {
		this.id = id;
		this.name = name;
		this.icon = icon;
		this.tabs = tabs;
		this.settings = new Settings("hud." + id);
		this.enabled = settings.add(new BoolSetting("enabled", "Enabled", on));
		this.x = settings.add(new NumberSetting("x", "X", defX, 0, 1, 0).showIf(() -> false));
		this.y = settings.add(new NumberSetting("y", "Y", defY, 0, 1, 0).showIf(() -> false));
		this.scale = settings.add(new NumberSetting("scale", "Size", 1, 0.5, 3, 0.05).format(v -> Math.round(v * 100) + "%"));
		this.placed = settings.add(new BoolSetting("placed", "Placed", false).showIf(() -> false));
		settings.add(new ActionSetting("edit", "Position", "Edit HUD layout", HudEditorScreen::open));
	}

	protected <S extends Setting<?>> S add(S s) {
		return settings.add(s);
	}

	protected HudElement anchor(Anchor a) {
		this.anchor = a;
		return this;
	}

	/** Still stacked automatically (has an anchor and was never dragged). */
	public boolean stacked() {
		return anchor != Anchor.NONE && !placed.on();
	}

	/** Back to its default place and size. */
	public void resetPlace() {
		x.reset();
		y.reset();
		scale.reset();
		placed.reset();
	}

	protected HudElement describe(String text) {
		this.description = text;
		return this;
	}

	protected HudElement item(Item item) {
		this.item = item;
		return this;
	}

	protected Font font() {
		return mc.font;
	}

	/** Unscaled size of what's drawn now ({@code preview}: sample content for the HUD editor). */
	public abstract int width(boolean preview);

	public abstract int height(boolean preview);

	/** Draw at (0, 0); the caller has moved and scaled the pose. */
	public abstract void render(GuiGraphics g, boolean preview);

	/** False hides the element for now (no potion effects, no boss bar, …); the editor still shows it. */
	public boolean hasContent() {
		return true;
	}

	/**
	 * Elements drawn by Minecraft at their own place (boss bar) or as a 3D model need absolute
	 * coordinates instead of a moved pose.
	 */
	public boolean drawsAbsolute() {
		return false;
	}

	/** For {@link #drawsAbsolute()} elements: draw into the given screen rectangle. */
	public void renderAbsolute(GuiGraphics g, int left, int top, float scale, boolean preview) {
	}

	public float scale() {
		return scale.f();
	}

	public int scaledWidth(boolean preview) {
		return Math.round(width(preview) * scale());
	}

	public int scaledHeight(boolean preview) {
		return Math.round(height(preview) * scale());
	}

	/** Left edge on a screen {@code screenW} wide. */
	public int left(int screenW, boolean preview) {
		if (stacked() && autoLeft >= 0) return autoLeft;
		return (int) Math.round(x.get() * Math.max(0, screenW - scaledWidth(preview)));
	}

	public int top(int screenH, boolean preview) {
		if (stacked() && autoTop >= 0) return autoTop;
		return (int) Math.round(y.get() * Math.max(0, screenH - scaledHeight(preview)));
	}

	/** Move so the top-left corner is at (left, top). */
	public void moveTo(int left, int top, int screenW, int screenH, boolean preview) {
		int freeW = Math.max(1, screenW - scaledWidth(preview)), freeH = Math.max(1, screenH - scaledHeight(preview));
		x.set(Math.max(0, Math.min(1, left / (double) freeW)));
		y.set(Math.max(0, Math.min(1, top / (double) freeH)));
		placed.set(true);
	}

	public void draw(GuiGraphics g, int screenW, int screenH, boolean preview) {
		int l = left(screenW, preview), t = top(screenH, preview);
		float s = scale();
		if (drawsAbsolute()) {
			renderAbsolute(g, l, t, s, preview);
			return;
		}
		g.pose().pushMatrix();
		g.pose().translate(l, t);
		g.pose().scale(s, s);
		render(g, preview);
		g.pose().popMatrix();
	}

	public Module card() {
		Module m = new Module("hud:" + id, name, icon, tabs, enabled::on, enabled::set, null, null).options(settings).describe(description).fresh();
		return item != null ? m.item(item) : m;
	}
}
