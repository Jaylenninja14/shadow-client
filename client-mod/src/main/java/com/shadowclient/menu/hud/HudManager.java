package com.shadowclient.menu.hud;

import com.shadowclient.menu.ShadowMenu;
import com.shadowclient.menu.feature.Crosshair;
import com.shadowclient.menu.module.Module;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.Settings;
import com.shadowclient.menu.ui.HudEditorScreen;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.fabricmc.fabric.api.client.rendering.v1.hud.VanillaHudElements;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.resources.Identifier;
import org.jetbrains.annotations.Nullable;

/**
 * Shadow Menu's HUD: every HUD module, drawn in one layer above Minecraft's own HUD. Minecraft's
 * scoreboard, boss bars, effect icons and crosshair are wrapped so the matching modules can move,
 * restyle or hide them.
 */
public final class HudManager {
	public static final Settings SETTINGS = new Settings("hud");
	public static final BoolSetting HIDE_F3 = SETTINGS.add(new BoolSetting("hide-f3", "Hide the HUD with F3", true));
	public static final BoolSetting SNAP = SETTINGS.add(new BoolSetting("snap", "Snap while dragging", true));
	public static final BoolSetting GRID = SETTINGS.add(new BoolSetting("grid", "Show grid in the editor", false));
	private static final List<HudElement> ELEMENTS = new ArrayList<>();
	private static final Set<String> FAILED = new HashSet<>();
	private static PotionEffects effects;
	private static GameElements.ScoreboardElement scoreboard;
	private static GameElements.BossBar bossBar;

	private HudManager() {
	}

	public static void init() {
		List<HudElement> all = new ArrayList<>();
		InfoElements.addTo(all);
		all.add(new Keystrokes());
		all.add(new ArmorStatus());
		effects = new PotionEffects();
		all.add(effects);
		all.add(new Compass());
		GameElements.addTo(all);
		for (HudElement e : all) {
			if (e instanceof GameElements.ScoreboardElement s) scoreboard = s;
			if (e instanceof GameElements.BossBar b) bossBar = b;
		}
		ELEMENTS.addAll(all);

		HudElementRegistry.attachElementAfter(VanillaHudElements.MISC_OVERLAYS, Identifier.fromNamespaceAndPath("shadowmenu", "hud_modules"), HudManager::render);
		HudElementRegistry.replaceElement(VanillaHudElements.SCOREBOARD, old -> (g, d) -> {
			if (!scoreboard.enabled.on()) old.render(g, d);
		});
		HudElementRegistry.replaceElement(VanillaHudElements.STATUS_EFFECTS, old -> (g, d) -> {
			if (!effects.hidesVanilla()) old.render(g, d);
		});
		HudElementRegistry.replaceElement(VanillaHudElements.CROSSHAIR, old -> (g, d) -> {
			if (!Crosshair.render(g, d)) old.render(g, d);
		});
		HudElementRegistry.replaceElement(VanillaHudElements.BOSS_BAR, old -> (g, d) -> {
			if (!bossBar.enabled.on() || Minecraft.getInstance().screen instanceof HudEditorScreen) {
				if (!bossBar.enabled.on()) old.render(g, d);
				return;
			}
			// move Minecraft's bars from where it draws them to where the module is
			int w = g.guiWidth(), h = g.guiHeight();
			float s = bossBar.scale();
			g.pose().pushMatrix();
			g.pose().translate(bossBar.left(w, false), bossBar.top(h, false));
			g.pose().scale(s, s);
			g.pose().translate(-GameElements.BossBar.vanillaLeft(w), -GameElements.BossBar.VANILLA_TOP);
			old.render(g, d);
			g.pose().popMatrix();
		});
	}

	public static List<HudElement> all() {
		return Collections.unmodifiableList(ELEMENTS);
	}

	@Nullable
	public static HudElement byId(String id) {
		for (HudElement e : ELEMENTS) if (e.id.equals(id)) return e;
		return null;
	}

	public static void addCards(List<Module> out) {
		for (HudElement e : ELEMENTS) out.add(e.card());
	}

	/** Shown in game now (not hidden by F1, F3 or the HUD editor, which draws its own preview). */
	public static boolean showing(Minecraft mc) {
		if (mc.player == null || mc.level == null || mc.options.hideGui) return false;
		if (mc.screen instanceof HudEditorScreen) return false;
		return !(HIDE_F3.on() && mc.debugEntries.isOverlayVisible());
	}

	private static final int MARGIN = 2, GAP = 2;

	/**
	 * Place the modules nobody dragged yet: stacked down (or, at the bottom, up) from their corner
	 * in list order, using their real size, so the default HUD never overlaps itself.
	 */
	public static void layout(int w, int h, boolean preview) {
		int tl = MARGIN, tr = MARGIN, tc = MARGIN, br = MARGIN;
		for (HudElement e : ELEMENTS) {
			if (!e.enabled.on() || !e.stacked()) continue;
			try {
				if (!preview && !e.hasContent()) continue;
				int ew = e.scaledWidth(preview), eh = e.scaledHeight(preview);
				switch (e.anchor) {
					case TOP_LEFT -> {
						e.autoLeft = MARGIN;
						e.autoTop = tl;
						tl += eh + GAP;
					}
					case TOP_RIGHT -> {
						e.autoLeft = w - ew - MARGIN;
						e.autoTop = tr;
						tr += eh + GAP;
					}
					case TOP_CENTER -> {
						e.autoLeft = (w - ew) / 2;
						e.autoTop = tc;
						tc += eh + GAP;
					}
					case BOTTOM_RIGHT -> {
						e.autoLeft = w - ew - MARGIN;
						e.autoTop = h - br - eh;
						br += eh + GAP;
					}
					default -> {
					}
				}
			} catch (RuntimeException ignored) {
				// placed by its x/y instead
			}
		}
	}

	private static void render(GuiGraphics g, DeltaTracker delta) {
		Minecraft mc = Minecraft.getInstance();
		if (!showing(mc)) return;
		int w = g.guiWidth(), h = g.guiHeight();
		layout(w, h, false);
		for (HudElement e : ELEMENTS) {
			if (!e.enabled.on()) continue;
			try {
				if (!e.hasContent()) continue;
				e.draw(g, w, h, false);
			} catch (RuntimeException ex) {
				// one broken module never takes the HUD down; log it once
				if (FAILED.add(e.id)) ShadowMenu.LOG.warn("HUD module {} failed to draw", e.id, ex);
			}
		}
	}
}
