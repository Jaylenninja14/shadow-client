package com.shadowclient.menu.ui;

import com.mojang.blaze3d.platform.NativeImage;
import com.shadowclient.menu.ShadowMenu;
import java.io.InputStream;
import java.lang.reflect.Method;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.function.Function;
import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.ModContainer;
import net.fabricmc.loader.api.entrypoint.EntrypointContainer;
import net.fabricmc.loader.api.metadata.ModMetadata;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.resources.Identifier;
import org.jetbrains.annotations.Nullable;

/**
 * Shadow Client's own mod list (instead of the Mod Menu mod): every installed mod with its icon,
 * version, authors and description, and a Settings button for mods that offer a settings screen
 * through Mod Menu's entrypoint (only when that mod's screen can load).
 */
final class ModsPage {
	record Entry(String id, String name, String version, String description, String authors, @Nullable Identifier icon,
			@Nullable Function<Screen, Screen> settings) {
	}

	private static List<Entry> cache;
	private static final Map<String, Identifier> ICONS = new HashMap<>();
	private final ModMenuScreen menu;
	private final Anim anim;
	private final Font font = Minecraft.getInstance().font;
	private double scroll, scrollTarget;
	private final List<int[]> buttons = new ArrayList<>(); // x, y, w, h, index
	private List<Entry> shown = List.of();

	ModsPage(ModMenuScreen menu, Anim anim) {
		this.menu = menu;
		this.anim = anim;
	}

	static List<Entry> mods() {
		if (cache != null) return cache;
		Map<String, Function<Screen, Screen>> screens = settingsScreens();
		List<Entry> out = new ArrayList<>();
		for (ModContainer mod : FabricLoader.getInstance().getAllMods()) {
			ModMetadata m = mod.getMetadata();
			// jars inside other mods (Fabric API's parts, libraries) are listed under their parent
			if (mod.getContainingMod().isPresent() || m.getId().equals("java") || hasParent(m)) continue;
			StringBuilder authors = new StringBuilder();
			m.getAuthors().forEach(p -> authors.append(authors.isEmpty() ? "" : ", ").append(p.getName()));
			out.add(new Entry(m.getId(), m.getName(), m.getVersion().getFriendlyString(), m.getDescription(), authors.toString(), icon(mod), screens.get(m.getId())));
		}
		out.sort((a, b) -> a.id().equals("shadowmenu") ? -1 : b.id().equals("shadowmenu") ? 1 : a.name().compareToIgnoreCase(b.name()));
		cache = out;
		return out;
	}

	/** Mods that list themselves under another (Fabric API's parts say so with Mod Menu's "parent"). */
	private static boolean hasParent(ModMetadata m) {
		net.fabricmc.loader.api.metadata.CustomValue v = m.getCustomValue("modmenu");
		return v != null && v.getType() == net.fabricmc.loader.api.metadata.CustomValue.CvType.OBJECT && v.getAsObject().containsKey("parent");
	}

	/** Settings screens mods register for Mod Menu ("modmenu" entrypoint), found without needing Mod Menu to build. */
	private static Map<String, Function<Screen, Screen>> settingsScreens() {
		Map<String, Function<Screen, Screen>> out = new HashMap<>();
		List<EntrypointContainer<Object>> list;
		try {
			list = FabricLoader.getInstance().getEntrypointContainers("modmenu", Object.class);
		} catch (Throwable t) {
			return out;
		}
		for (EntrypointContainer<Object> c : list) {
			String id = c.getProvider().getMetadata().getId();
			try {
				Object api = c.getEntrypoint();
				Method m = api.getClass().getMethod("getModConfigScreenFactory");
				Object factory = m.invoke(api);
				if (factory == null) continue;
				Method create = factory.getClass().getMethod("create", Screen.class);
				create.setAccessible(true);
				out.put(id, parent -> {
					try {
						return (Screen) create.invoke(factory, parent);
					} catch (ReflectiveOperationException | RuntimeException e) {
						ShadowMenu.LOG.warn("{}'s settings screen failed to open", id, e);
						return null;
					}
				});
			} catch (Throwable t) { // that mod's Mod Menu support needs Mod Menu itself
				ShadowMenu.LOG.debug("No settings screen for {}: {}", id, t.toString());
			}
		}
		return out;
	}

	/** The mod's own icon as a texture (loaded once), or null for a letter avatar. */
	@Nullable
	private static Identifier icon(ModContainer mod) {
		String id = mod.getMetadata().getId();
		if (ICONS.containsKey(id)) return ICONS.get(id);
		Identifier result = null;
		try {
			Optional<String> path = mod.getMetadata().getIconPath(64);
			Optional<Path> file = path.flatMap(mod::findPath);
			if (file.isPresent()) {
				try (InputStream in = Files.newInputStream(file.get())) {
					NativeImage img = NativeImage.read(in);
					result = Identifier.fromNamespaceAndPath("shadowmenu", "modicon/" + id.toLowerCase(Locale.ROOT).replaceAll("[^a-z0-9_.-]", "_"));
					Minecraft.getInstance().getTextureManager().register(result, new DynamicTexture(() -> "Mod icon " + id, img));
				}
			}
		} catch (Exception e) {
			result = null;
		}
		ICONS.put(id, result);
		return result;
	}

	private int c(int color, float alpha) {
		return Draw.fade(color, alpha);
	}

	void render(GuiGraphics g, int x, int y, int w, int h, int mx, int my, float alpha, String query) {
		buttons.clear();
		String q = query.trim().toLowerCase(Locale.ROOT);
		List<Entry> list = new ArrayList<>();
		for (Entry e : mods()) {
			if (q.isEmpty() || e.name().toLowerCase(Locale.ROOT).contains(q) || e.id().contains(q) || e.description().toLowerCase(Locale.ROOT).contains(q)) list.add(e);
		}
		shown = list;
		int rowH = 38, gap = 4;
		int content = list.size() * (rowH + gap);
		scrollTarget = Math.max(0, Math.min(scrollTarget, Math.max(0, content - h)));
		scroll += (scrollTarget - scroll) * Math.min(1, anim.dt() * 16);
		g.enableScissor(x - 2, y, x + w + 2, y + h);
		for (int i = 0; i < list.size(); i++) {
			Entry e = list.get(i);
			int ry = y + i * (rowH + gap) - (int) Math.round(scroll);
			if (ry + rowH < y || ry > y + h) continue;
			boolean hover = Draw.inside(mx, my, x, ry, w, rowH) && my >= y && my < y + h;
			float hv = anim.get("mod:" + e.id(), hover ? 1 : 0, 16);
			Draw.round(g, x, ry, w, rowH, 6, c(Draw.mix(Theme.card(), Theme.cardHover(), hv), alpha));
			int ix = x + 7, iy = ry + 7;
			if (e.icon() != null) {
				g.blit(RenderPipelines.GUI_TEXTURED, e.icon(), ix, iy, 0, 0, 24, 24, 24, 24, c(0xFFFFFFFF, alpha));
			} else {
				Draw.round(g, ix, iy, 24, 24, 5, c(Draw.mix(Theme.card(), Theme.accent(), 0.35f), alpha));
				String letter = e.name().isEmpty() ? "?" : e.name().substring(0, 1).toUpperCase(Locale.ROOT);
				g.drawCenteredString(font, letter, ix + 12, iy + 8, c(Theme.text(), alpha));
			}
			int tx = ix + 32;
			int right = x + w - 8;
			if (e.settings() != null) {
				int bw = font.width("Settings") + 16;
				boolean bh = Draw.inside(mx, my, right - bw, ry + 11, bw, 16);
				Draw.round(g, right - bw, ry + 11, bw, 16, 4, c(bh ? Draw.mix(Theme.accent(), 0xFFFFFFFF, 0.15f) : Theme.accent(), alpha));
				g.drawCenteredString(font, "Settings", right - bw / 2, ry + 15, c(Theme.onAccent(), alpha));
				if (my >= y && my < y + h) buttons.add(new int[] {right - bw, ry + 11, bw, 16, i});
				right -= bw + 8;
			}
			String title = e.name() + "  ";
			g.drawString(font, title, tx, ry + 8, c(Theme.text(), alpha), false);
			String ver = e.version() + (e.authors().isEmpty() ? "" : "  ·  " + e.authors());
			g.drawString(font, font.plainSubstrByWidth(ver, Math.max(10, right - tx - font.width(title))), tx + font.width(title), ry + 8, c(Theme.faint(), alpha), false);
			g.drawString(font, font.plainSubstrByWidth(e.description().replace('\n', ' '), Math.max(10, right - tx)), tx, ry + 21, c(Theme.dim(), alpha), false);
		}
		g.disableScissor();
		if (list.isEmpty()) g.drawCenteredString(font, "No mods match \"" + query + "\".", x + w / 2, y + 20, c(Theme.faint(), alpha));
		if (content > h) {
			int thumb = Math.max(20, h * h / content);
			int ty = y + (int) ((h - thumb) * (scroll / (content - h)));
			Draw.round(g, x + w + 3, ty, 3, thumb, 1.5f, c(Theme.line(), alpha));
		}
	}

	boolean mouseClicked(double mx, double my) {
		for (int[] b : buttons) {
			if (!Draw.inside(mx, my, b[0], b[1], b[2], b[3])) continue;
			Entry e = shown.get(b[4]);
			menu.click();
			Screen s = e.settings().apply(menu);
			if (s != null) Minecraft.getInstance().setScreen(s);
			return true;
		}
		return false;
	}

	void mouseScrolled(double amount) {
		scrollTarget -= amount * 36;
	}

	static int count() {
		return mods().size();
	}
}
