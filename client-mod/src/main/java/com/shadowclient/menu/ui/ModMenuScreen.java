package com.shadowclient.menu.ui;

import com.shadowclient.menu.ShadowMenu;
import com.shadowclient.menu.feature.Features;
import com.shadowclient.menu.hud.HudElement;
import com.shadowclient.menu.hud.HudManager;
import com.shadowclient.menu.module.AutoText;
import com.shadowclient.menu.module.Combat;
import com.shadowclient.menu.module.GameModules;
import com.shadowclient.menu.module.MenuConfig;
import com.shadowclient.menu.module.Module;
import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.module.OverlayPacks;
import com.shadowclient.menu.module.QuickKeys;
import com.shadowclient.menu.module.ShadowHud;
import com.shadowclient.menu.module.Waypoints;
import com.shadowclient.menu.setting.ActionSetting;
import com.shadowclient.menu.setting.KeySetting;
import com.shadowclient.menu.setting.Setting;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.options.OptionsScreen;
import net.minecraft.client.gui.screens.options.VideoSettingsScreen;
import net.minecraft.client.input.CharacterEvent;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.network.chat.Component;
import net.minecraft.sounds.SoundEvents;
import org.jetbrains.annotations.Nullable;
import org.lwjgl.glfw.GLFW;

/**
 * Shadow Client's mod menu (Right Shift): categories on the left, search and the HUD editor at the
 * top, and a grid of module cards (icon, favourite, settings, an animated Enabled/Disabled bar or
 * a value to click through). A card's gear opens its settings page with sliders, switches, colour
 * pickers and keys. Look and animations follow the theme (Shadow Client settings, bottom left).
 */
public final class ModMenuScreen extends Screen {
	private record Category(String name, String icon, @Nullable Tab tab) {
	}

	private static final Category ALL = new Category("All Mods", "grid", null);
	private static final Category FAVS = new Category("Favourites", "heart", null);
	private static final Category NEW = new Category("New", "boost", null);
	private static final Category MODS = new Category("Mods", "pack", null);
	private static final List<Category> CATEGORIES = List.of(ALL, FAVS, NEW, new Category("HUD", "hotbar", Tab.HUD), new Category("PvP", "attack", Tab.PVP),
			new Category("Hypixel", "server", Tab.HYPIXEL), new Category("Visual", "glint", Tab.VISUAL), new Category("Performance", "fps", Tab.PERFORMANCE),
			new Category("Game", "gear", Tab.GAME), MODS);

	private static Category category = ALL;
	private static String query = "";

	private final List<Module> modules = new ArrayList<>();
	private final Anim anim = new Anim();
	private final ModsPage modsPage = new ModsPage(this, anim);
	@Nullable private final Screen parent;
	@Nullable private SettingsPage page;
	/** Opened straight on a page (from the HUD editor): going back closes the menu. */
	private boolean directPage;
	@Nullable private Module pendingPage;
	private boolean pendingClient;
	private boolean searchFocused;
	private double scroll, scrollTarget;
	private long openedAt;
	private boolean closing;

	// layout (computed every frame)
	private int px, py, pw, ph, sideW, cx, cy, cw, ch, cols, cardW, cardH;
	private static final int HEADER = 32, GAP = 6;

	private final List<Hit> hits = new ArrayList<>();
	private final List<Hit> cardHits = new ArrayList<>();

	private record Hit(int x, int y, int w, int h, Runnable left, @Nullable Runnable right) {
	}

	public ModMenuScreen() {
		this(null);
	}

	public ModMenuScreen(@Nullable Screen parent) {
		super(Component.literal("Mod Menu"));
		this.parent = parent;
	}

	/** The menu on the HUD category. */
	public static ModMenuScreen hudTab(@Nullable Screen parent) {
		category = CATEGORIES.get(3);
		query = "";
		return new ModMenuScreen(parent);
	}

	/** The menu on the list of installed mods. */
	public static ModMenuScreen mods(@Nullable Screen parent) {
		category = MODS;
		query = "";
		return new ModMenuScreen(parent);
	}

	/** The menu on the Shadow Client settings page (theme, accent, animations, keys). */
	public static ModMenuScreen clientSettings(@Nullable Screen parent) {
		ModMenuScreen s = new ModMenuScreen(parent);
		s.pendingClient = true;
		return s;
	}

	/** The menu straight on one module's page; back returns to {@code parent}. */
	public static ModMenuScreen page(@Nullable Screen parent, Module module) {
		ModMenuScreen s = new ModMenuScreen(parent);
		s.pendingPage = module;
		s.directPage = true;
		return s;
	}

	@Override
	public void onClose() {
		if (page != null) page.close();
		MenuConfig.flush(true);
		minecraft.setScreen(parent);
	}

	@Override
	public boolean isPauseScreen() {
		return true;
	}

	@Override
	protected void init() {
		modules.clear();
		Features.addCards(modules);
		HudManager.addCards(modules);
		GameModules.addTo(modules);
		GameModules.addMore(modules);
		GameModules.addEvenMore(modules);
		ShadowHud.addTo(modules);
		OverlayPacks.addTo(modules);
		Waypoints.addTo(modules);
		AutoText.addTo(modules);
		Combat.addTo(modules);
		QuickKeys.addTo(modules);
		modules.sort((a, b) -> a.name.compareToIgnoreCase(b.name));
		if (openedAt == 0) openedAt = System.currentTimeMillis();
		if (pendingPage != null) {
			openPage(pendingPage);
			pendingPage = null;
		}
		if (pendingClient) {
			openClientPage();
			pendingClient = false;
		}
	}

	/** Every module, for the startup log. */
	public static List<Module> allModules() {
		List<Module> out = new ArrayList<>();
		Features.addCards(out);
		HudManager.addCards(out);
		GameModules.addTo(out);
		GameModules.addMore(out);
		GameModules.addEvenMore(out);
		ShadowHud.addTo(out);
		OverlayPacks.addTo(out);
		Waypoints.addTo(out);
		AutoText.addTo(out);
		Combat.addTo(out);
		QuickKeys.addTo(out);
		return out;
	}

	void click() {
		if (Theme.SOUNDS.on()) minecraft.getSoundManager().play(SimpleSoundInstance.forUI(SoundEvents.UI_BUTTON_CLICK, 1.0F));
	}

	private void openPage(Module m) {
		page = new SettingsPage(this, m.name, m.icon, m.item(), m.description(), m.options(), m, () -> {
			if (m.settingsGroup() != null) m.settingsGroup().resetAll();
		}, anim);
		anim.set("page", 0);
	}

	private void openClientPage() {
		List<Setting<?>> list = new ArrayList<>(Theme.SETTINGS.all());
		list.add(new KeySetting("menu-key", "Menu key", ShadowMenu::openKey).group("Controls"));
		list.add(new ActionSetting("hud-editor", "HUD layout", "Edit HUD layout", HudEditorScreen::open).group("Controls"));
		for (Setting<?> s : HudManager.SETTINGS.all()) list.add(s.group == null ? s.group("HUD") : s);
		list.add(new ActionSetting("minecraft-options", "Minecraft options", "Open", s -> minecraft.setScreen(new OptionsScreen(s, minecraft.options))).group("Controls"));
		list.add(new ActionSetting("video", "Video settings", "Open", s -> minecraft.setScreen(new VideoSettingsScreen(s, minecraft, minecraft.options))).group("Controls"));
		page = new SettingsPage(this, "Shadow Client", "gear", null, "How the menu looks and moves, its key, and HUD editor options. Version "
				+ ShadowMenu.version() + ".", list, null, () -> {
			Theme.SETTINGS.resetAll();
			HudManager.SETTINGS.resetAll();
		}, anim);
		anim.set("page", 0);
	}

	void closePage() {
		if (page != null) page.close();
		click();
		if (directPage) {
			onClose();
			return;
		}
		page = null;
	}

	private static boolean matches(Module m, Category c) {
		if (c == ALL) return true;
		if (c == FAVS) return MenuConfig.isFavorite(m.id);
		if (c == NEW) return m.isNew();
		if (c == MODS) return false;
		return m.tabs.contains(c.tab());
	}

	private List<Module> visible() {
		String q = query.trim().toLowerCase(Locale.ROOT);
		List<Module> out = new ArrayList<>();
		for (Module m : modules) {
			if (!q.isEmpty()) {
				String d = m.description() == null ? "" : m.description().toLowerCase(Locale.ROOT);
				if (!m.name.toLowerCase(Locale.ROOT).contains(q) && !d.contains(q)) continue;
			} else if (!matches(m, category)) {
				continue;
			}
			out.add(m);
		}
		return out;
	}

	private int count(Category c) {
		if (c == MODS) return ModsPage.count();
		int n = 0;
		for (Module m : modules) if (matches(m, c)) n++;
		return n;
	}

	private void layout() {
		pw = Math.min(width - 16, 860);
		ph = Math.min(height - 16, 470);
		px = (width - pw) / 2;
		py = (height - ph) / 2;
		sideW = pw < 520 ? 84 : 112;
		cx = px + sideW + 10;
		cy = py + HEADER + 30;
		cw = px + pw - 10 - 6 - cx;
		ch = py + ph - 10 - cy;
		int min = switch (Theme.CARDS.get()) {
			case "compact" -> 86;
			case "large" -> 128;
			default -> 104;
		};
		cols = Math.max(1, (cw + GAP) / (min + GAP));
		cardW = (cw - (cols - 1) * GAP) / cols;
		cardH = switch (Theme.CARDS.get()) {
			case "compact" -> 66;
			case "large" -> 108;
			default -> 88;
		};
	}

	private int contentHeight(int n) {
		int rows = (n + cols - 1) / cols;
		return rows == 0 ? 0 : rows * cardH + (rows - 1) * GAP;
	}

	// ------------------------------------------------------------------ rendering

	private float open() {
		if (Theme.speed() <= 0) return 1;
		return Draw.ease((System.currentTimeMillis() - openedAt) / (220f / Theme.speed()));
	}

	private int c(int color) {
		return Draw.fade(color, open());
	}

	@Override
	public void renderBackground(GuiGraphics g, int mx, int my, float partial) {
		if (minecraft.level == null) renderPanorama(g, partial);
		if (Theme.BLUR.on()) renderBlurredBackground(g);
		g.fill(0, 0, width, height, Draw.fade(Theme.backdrop(), open()));
	}

	@Override
	public void render(GuiGraphics g, int mx, int my, float partial) {
		anim.frame();
		layout();
		hits.clear();
		cardHits.clear();
		float o = open();
		// open animation: scale up from 96% around the centre
		g.pose().pushMatrix();
		float s = 0.96f + 0.04f * o;
		g.pose().translate(width / 2f, height / 2f);
		g.pose().scale(s, s);
		g.pose().translate(-width / 2f, -height / 2f);
		int amx = (int) Math.round((mx - width / 2f) / s + width / 2f), amy = (int) Math.round((my - height / 2f) / s + height / 2f);

		Draw.shadow(g, px, py, pw, ph, 10, 8, Math.round(120 * o));
		Draw.round(g, px, py, pw, ph, 10, c(Theme.panel()));
		Draw.round(g, px, py, sideW, ph, 10, c(Theme.sidebar()));
		g.fill(px + sideW - 10, py, px + sideW, py + ph, c(Theme.sidebar()));
		g.fill(px + sideW, py, px + sideW + 1, py + ph, c(Theme.line()));

		header(g, amx, amy);
		sidebar(g, amx, amy);
		if (page != null) {
			float slide = anim.get("page", 1, 14);
			int off = Math.round((1 - slide) * 40);
			g.pose().pushMatrix();
			g.pose().translate(off, 0);
			page.render(g, px + sideW + 1, py + HEADER, pw - sideW - 1, ph - HEADER, amx - off, amy, o * Math.max(0.15f, slide));
			g.pose().popMatrix();
		} else {
			grid(g, amx, amy);
		}
		g.pose().popMatrix();
	}

	private void header(GuiGraphics g, int mx, int my) {
		int x = px + 12, y = py + 9;
		Draw.icon(g, "shadow", x, y - 1, 14, c(Theme.accent()));
		Draw.scaledText(g, font, "SHADOW", x + 18, y + 2, 1.1f, c(Theme.text()), false);
		Draw.scaledText(g, font, "CLIENT", x + 18 + font.width("SHADOW") * 1.1f + 4, y + 2, 1.1f, c(Theme.accent()), false);
		g.fill(px + sideW, py + HEADER, px + pw, py + HEADER + 1, c(Theme.line()));

		// close
		int bx = px + pw - 10 - 20;
		boolean closeHover = Draw.inside(mx, my, bx, y - 3, 20, 20);
		Draw.round(g, bx, y - 3, 20, 20, 5, c(closeHover ? 0xFFD94B4B : Theme.chip()));
		cross(g, bx + 6, y + 3, 8, c(closeHover ? 0xFFFFFFFF : Theme.dim()));
		hits.add(new Hit(bx, y - 3, 20, 20, () -> {
			click();
			onClose();
		}, null));
		// edit HUD
		String hud = "Edit HUD";
		int hw = font.width(hud) + 26;
		int hx = bx - 6 - hw;
		boolean hudHover = Draw.inside(mx, my, hx, y - 3, hw, 20);
		Draw.round(g, hx, y - 3, hw, 20, 5, c(hudHover ? Draw.mix(Theme.accent(), 0xFFFFFFFF, 0.15f) : Theme.accent()));
		Draw.icon(g, "hotbar", hx + 6, y + 2, 10, c(Theme.onAccent()));
		g.drawString(font, hud, hx + 20, y + 3, c(Theme.onAccent()), false);
		hits.add(new Hit(hx, y - 3, hw, 20, () -> {
			click();
			minecraft.setScreen(new HudEditorScreen(this));
		}, null));
		// search
		// whatever is left between the logo and the buttons, within 70-190 pixels
		int logoEnd = px + 12 + 18 + Math.round(font.width("SHADOW CLIENT") * 1.1f) + 16;
		int sw = Math.max(70, Math.min(190, hx - 8 - logoEnd));
		int sx = hx - 8 - sw;
		boolean active = searchFocused || !query.isEmpty();
		float f = anim.get("search", active ? 1 : 0, 14);
		Draw.round(g, sx, y - 3, sw, 20, 5, c(Theme.bg()));
		Draw.roundOutline(g, sx, y - 3, sw, 20, 5, c(Draw.mix(Theme.line(), Theme.accent(), f)));
		Draw.icon(g, "zoom", sx + 6, y + 2, 10, c(Theme.faint()));
		String shown = query.isEmpty() && !searchFocused ? font.plainSubstrByWidth(sw > 150 ? "Search mods…  (just type)" : "Search mods…", sw - 26)
				: font.plainSubstrByWidth(query, sw - 30, true);
		g.drawString(font, shown, sx + 20, y + 3, c(query.isEmpty() && !searchFocused ? Theme.faint() : Theme.text()), false);
		if (searchFocused && (System.currentTimeMillis() / 500) % 2 == 0) {
			int caret = sx + 20 + font.width(shown);
			g.fill(caret, y + 1, caret + 1, y + 12, c(Theme.accent()));
		}
		hits.add(new Hit(sx, y - 3, sw, 20, () -> searchFocused = true, () -> {
			query = "";
			scrollTarget = 0;
		}));
	}

	private void sidebar(GuiGraphics g, int mx, int my) {
		int x = px + 8, y = py + HEADER + 6, w = sideW - 16;
		// rows shrink on small screens so the categories never run into Client Settings below
		int room = ph - HEADER - 6 - (8 + 20 + 6);
		int h = Math.max(12, Math.min(20, room / CATEGORIES.size() - 3));
		int selIndex = page == null && query.isEmpty() ? CATEGORIES.indexOf(category) : -1;
		float selY = anim.get("cat", selIndex < 0 ? -100 : y + selIndex * (h + 3), 16);
		if (selIndex >= 0) Draw.round(g, x, selY, w, h, 5, c(Draw.mix(Theme.sidebar(), Theme.accent(), 0.22f)));
		for (int i = 0; i < CATEGORIES.size(); i++) {
			Category cat = CATEGORIES.get(i);
			int yy = y + i * (h + 3);
			boolean sel = i == selIndex;
			boolean hover = Draw.inside(mx, my, x, yy, w, h);
			float hv = anim.get("cath:" + i, hover && !sel ? 1 : 0, 16);
			if (hv > 0.01f) Draw.round(g, x, yy, w, h, 5, c(Draw.fade(Theme.chipHover(), hv)));
			if (sel) g.fill(x, yy + 4, x + 2, yy + h - 4, c(Theme.accent()));
			Draw.icon(g, cat.icon(), x + 7, yy + (h - 10) / 2f, 10, c(sel ? Theme.accentText() : Theme.dim()));
			String name = cat.name();
			if (sideW < 100 && name.length() > 8) name = name.substring(0, 6) + ".";
			g.drawString(font, name, x + 22, yy + (h - 8) / 2, c(sel ? Theme.text() : Theme.dim()), false);
			String n = String.valueOf(count(cat));
			if (sideW >= 100) g.drawString(font, n, x + w - 5 - font.width(n), yy + (h - 8) / 2, c(Theme.faint()), false);
			hits.add(new Hit(x, yy, w, h, () -> {
				click();
				category = cat;
				query = "";
				searchFocused = false;
				scrollTarget = scroll = 0;
				if (page != null) page.close();
				page = null;
			}, null));
		}
		// Shadow Client settings, at the bottom
		h = 20;
		int by = py + ph - 8 - h;
		boolean hover = Draw.inside(mx, my, x, by, w, h);
		boolean onClient = page != null && page.module == null;
		Draw.round(g, x, by, w, h, 5, c(onClient ? Draw.mix(Theme.sidebar(), Theme.accent(), 0.22f) : hover ? Theme.chipHover() : Theme.chip()));
		Draw.icon(g, "gear", x + 7, by + 5, 10, c(onClient ? Theme.accentText() : Theme.dim()));
		g.drawString(font, sideW < 100 ? "Settings" : "Client Settings", x + 22, by + 6, c(Theme.text()), false);
		hits.add(new Hit(x, by, w, h, () -> {
			click();
			directPage = false;
			openClientPage();
		}, null));
	}

	private void grid(GuiGraphics g, int mx, int my) {
		if (category == MODS) {
			String heading = query.isEmpty() ? "Mods" : "Mods: " + query;
			Draw.scaledText(g, font, heading, cx, py + HEADER + 10, 1.2f, c(Theme.text()), false);
			String n = ModsPage.count() + " installed";
			g.drawString(font, n, cx + Math.round(font.width(heading) * 1.2f) + 8, py + HEADER + 12, c(Theme.faint()), false);
			modsPage.render(g, cx, cy, cw, ch, mx, my, open(), query);
			return;
		}
		List<Module> list = visible();
		String heading = query.isEmpty() ? category.name() : "Search: " + query;
		Draw.scaledText(g, font, heading, cx, py + HEADER + 10, 1.2f, c(Theme.text()), false);
		String count = list.size() + (list.size() == 1 ? " mod" : " mods");
		g.drawString(font, count, cx + Math.round(font.width(heading) * 1.2f) + 8, py + HEADER + 12, c(Theme.faint()), false);

		int content = contentHeight(list.size());
		scrollTarget = Math.max(0, Math.min(scrollTarget, Math.max(0, content - ch)));
		scroll += (scrollTarget - scroll) * Math.min(1, anim.dt() * 16);
		g.enableScissor(cx - 2, cy - 2, cx + cw + 2, cy + ch);
		boolean inGrid = Draw.inside(mx, my, cx, cy, cw, ch);
		for (int i = 0; i < list.size(); i++) {
			int col = i % cols, row = i / cols;
			int x0 = cx + col * (cardW + GAP);
			int y0 = cy + row * (cardH + GAP) - (int) Math.round(scroll);
			if (y0 + cardH < cy || y0 > cy + ch) continue;
			card(g, list.get(i), x0, y0, inGrid ? mx : -1000, my, i);
		}
		g.disableScissor();
		if (list.isEmpty()) {
			String msg = category == FAVS && query.isEmpty() ? "No favourites yet: click the heart on a card." : "No mods match \"" + query + "\".";
			g.drawCenteredString(font, msg, cx + cw / 2, cy + 30, c(Theme.faint()));
		}
		if (content > ch) {
			int track = cx + cw + 3;
			int thumb = Math.max(20, ch * ch / content);
			int ty = cy + (int) ((ch - thumb) * (scroll / (content - ch)));
			Draw.round(g, track, ty, 3, thumb, 1.5f, c(Theme.line()));
		}
	}

	private void card(GuiGraphics g, Module m, int x, int y, int mx, int my, int index) {
		boolean hover = Draw.inside(mx, my, x, y, cardW, cardH);
		float hv = anim.get("card:" + m.id, hover ? 1 : 0, 16);
		boolean on = m.enabled();
		float onP = anim.get("on:" + m.id, on ? 1 : 0, 12);
		boolean dim = m.toggleable() && !on;
		// cards fade in one after another when the menu opens
		float appear = Theme.speed() <= 0 ? 1 : Draw.ease((System.currentTimeMillis() - openedAt - Math.min(index, 16) * 18) / (260f / Theme.speed()));
		int lift = Math.round((1 - appear) * 6 + hv * -1);
		y += lift;
		int bg = Draw.mix(Theme.card(), Theme.cardHover(), hv);
		Draw.round(g, x, y, cardW, cardH, 7, c(Draw.fade(bg, appear)));
		Draw.roundOutline(g, x, y, cardW, cardH, 7, c(Draw.fade(Draw.mix(Theme.line(), Theme.accent(), hv * 0.8f), appear)));

		// favourite heart (shown on hover or when hearted)
		boolean fav = MenuConfig.isFavorite(m.id);
		int hx = x + cardW - 18, hy = y + 5;
		if (fav || hv > 0.02f) {
			boolean heartHover = Draw.inside(mx, my, hx, hy, 13, 12);
			Draw.icon(g, "heart", hx + 1, hy + 1, 10, c(Draw.fade(fav ? Theme.heart() : heartHover ? Theme.text() : Theme.faint(), fav ? appear : hv * appear)));
		}
		cardHits.add(new Hit(hx, hy, 13, 12, () -> {
			click();
			MenuConfig.toggleFavorite(m.id);
		}, null));
		if (m.isNew() && category != NEW) {
			int nw = font.width("NEW") + 6;
			Draw.round(g, x + 6, y + 6, nw, 10, 3, c(Draw.fade(Draw.mix(Theme.card(), Theme.accent(), 0.35f), appear)));
			Draw.scaledText(g, font, "NEW", x + 9, y + 7.5f, 0.75f, c(Draw.fade(Theme.accentText(), appear)), false);
		}

		// icon and name
		int bar = 16;
		int iconSize = Theme.CARDS.is("compact") ? 18 : Theme.CARDS.is("large") ? 34 : 26;
		int nameY = y + cardH - bar - 8 - 12;
		int iconY = y + 8 + Math.max(0, (nameY - y - 8 - iconSize) / 2) - 1;
		float pop = 1 + hv * 0.06f;
		int isz = Math.round(iconSize * pop);
		int icx = x + cardW / 2 - isz / 2, icy = iconY - Math.round((isz - iconSize) / 2f);
		if (m.item() != null) Draw.item(g, m.item(), icx, icy, isz);
		else Draw.icon(g, m.icon, icx, icy, isz, c(Draw.fade(dim ? Theme.faint() : Draw.mix(Theme.text(), Theme.accentText(), onP * 0.35f), appear)));
		Draw.fitCentered(g, font, m.name, x + cardW / 2, nameY + 2, cardW - 10, c(Draw.fade(dim ? Theme.dim() : Theme.text(), appear)));

		// bottom bar: settings gear, then the switch / value / hint
		int by = y + cardH - bar - 6;
		int bx = x + 6;
		boolean hasGear = m.hasSettings();
		if (hasGear) {
			boolean gh = Draw.inside(mx, my, bx, by, bar, bar);
			Draw.round(g, bx, by, bar, bar, 4, c(Draw.fade(gh ? Theme.chipHover() : Theme.chip(), appear)));
			Draw.icon(g, "gear", bx + 3, by + 3, 10, c(Draw.fade(gh ? Theme.text() : Theme.dim(), appear)));
			cardHits.add(new Hit(bx, by, bar, bar, () -> openSettings(m), null));
			bx += bar + 4;
		}
		int sw = x + cardW - 6 - bx;
		boolean barHover = Draw.inside(mx, my, bx, by, sw, bar);
		if (m.toggleable()) {
			Draw.round(g, bx, by, sw, bar, 4, c(Draw.fade(barHover ? Draw.mix(Theme.off(), Theme.chipHover(), 0.5f) : Theme.off(), appear)));
			float fill = Draw.ease(onP);
			if (fill > 0.01f) Draw.round(g, bx, by, Math.max(8, sw * fill), bar, 4, c(Draw.fade(barHover ? Draw.mix(Theme.accent(), 0xFFFFFFFF, 0.12f) : Theme.accent(), appear)));
			String state = on ? "ENABLED" : "DISABLED";
			int tc = Draw.mix(Theme.dim(), Theme.onAccent(), onP);
			Draw.scaledText(g, font, state, bx + sw / 2f - font.width(state) * 0.8f / 2, by + 4.5f, 0.8f, c(Draw.fade(tc, appear)), false);
			cardHits.add(new Hit(bx, by, sw, bar, () -> {
				click();
				m.setEnabled(!m.enabled());
			}, null));
		} else if (m.cycles()) {
			Draw.round(g, bx, by, sw, bar, 4, c(Draw.fade(barHover ? Draw.mix(Theme.chip(), Theme.accent(), 0.3f) : Draw.mix(Theme.chip(), Theme.accent(), 0.15f), appear)));
			g.drawString(font, "<", bx + 4, by + 4, c(Draw.fade(Theme.accentText(), appear)), false);
			g.drawString(font, ">", bx + sw - 4 - font.width(">"), by + 4, c(Draw.fade(Theme.accentText(), appear)), false);
			Draw.fitCentered(g, font, m.value(), bx + sw / 2, by + 4, sw - 20, c(Draw.fade(Theme.text(), appear)));
			int bxx = bx;
			cardHits.add(new Hit(bx, by, sw, bar, () -> {
				click();
				double mouse = minecraft.mouseHandler.getScaledXPos(minecraft.getWindow());
				m.step(mouse < bxx + sw / 3.0 ? -1 : 1);
			}, () -> {
				click();
				m.step(-1);
			}));
		} else {
			Draw.round(g, bx, by, sw, bar, 4, c(Draw.fade(Theme.chip(), appear)));
			Draw.fitCentered(g, font, m.hint == null ? "Settings" : m.hint, bx + sw / 2, by + 4, sw - 8, c(Draw.fade(Theme.faint(), appear)));
			if (hasGear) cardHits.add(new Hit(bx, by, sw, bar, () -> openSettings(m), null));
		}
		// clicking the rest of the card opens its settings (or flips it when it has none)
		cardHits.add(0, new Hit(x, y, cardW, cardH, () -> {
			if (m.hasSettings()) openSettings(m);
			else if (m.toggleable()) {
				click();
				m.setEnabled(!m.enabled());
			}
		}, m.cycles() ? () -> {
			click();
			m.step(-1);
		} : null));
	}

	private void openSettings(Module m) {
		click();
		if (m.hasPage()) {
			directPage = false;
			openPage(m);
			return;
		}
		Screen s = m.settings(this);
		if (s != null) minecraft.setScreen(s);
	}

	/** An "x" made of small squares (fonts differ in whether they have a nice ✕). */
	private static void cross(GuiGraphics g, int x, int y, int size, int color) {
		for (int i = 0; i < size; i++) {
			g.fill(x + i, y + i, x + i + 1, y + i + 1, color);
			g.fill(x + i, y + size - 1 - i, x + i + 1, y + size - i, color);
		}
	}

	// ------------------------------------------------------------------ input

	private double[] unscale(double mx, double my) {
		float s = 0.96f + 0.04f * open();
		return new double[] {(mx - width / 2f) / s + width / 2f, (my - height / 2f) / s + height / 2f};
	}

	@Override
	public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) {
		double[] m = unscale(event.x(), event.y());
		double mx = m[0], my = m[1];
		int button = event.button();
		boolean wasSearch = searchFocused;
		searchFocused = false;
		// the page gets its clicks first (it knows its own controls)
		if (page != null && Draw.inside(mx, my, px + sideW, py + HEADER, pw - sideW, ph - HEADER)) {
			if (page.mouseClicked(mx, my, button)) return true;
		} else if (page != null) {
			page.mouseClicked(-1, -1, -1);
		}
		if (button != 0 && button != 1) return false;
		if (page == null && category == MODS && button == 0 && Draw.inside(mx, my, cx, cy, cw, ch) && modsPage.mouseClicked(mx, my)) return true;
		// later hits are drawn on top: check them first; cards only inside the visible grid
		List<Hit> all = new ArrayList<>(hits);
		if (page == null && Draw.inside(mx, my, cx, cy, cw, ch)) all.addAll(cardHits);
		for (int i = all.size() - 1; i >= 0; i--) {
			Hit h = all.get(i);
			if (!Draw.inside(mx, my, h.x(), h.y(), h.w(), h.h())) continue;
			if (button == 0) {
				h.left().run();
				return true;
			}
			if (h.right() != null) {
				h.right().run();
				return true;
			}
		}
		return wasSearch;
	}

	@Override
	public boolean mouseDragged(MouseButtonEvent event, double dx, double dy) {
		double[] m = unscale(event.x(), event.y());
		return page != null && page.mouseDragged(m[0], m[1]);
	}

	@Override
	public boolean mouseReleased(MouseButtonEvent event) {
		if (page != null) page.mouseReleased();
		return true;
	}

	@Override
	public boolean mouseScrolled(double mx, double my, double scrollX, double scrollY) {
		if (page != null) page.mouseScrolled(scrollY);
		else if (category == MODS) modsPage.mouseScrolled(scrollY);
		else scrollTarget -= scrollY * 36;
		return true;
	}

	@Override
	public boolean keyPressed(KeyEvent event) {
		if (page != null && page.keyPressed(event)) return true;
		int key = event.key();
		if (searchFocused || !query.isEmpty()) {
			if (key == GLFW.GLFW_KEY_BACKSPACE && !query.isEmpty()) {
				query = event.hasControlDown() ? "" : query.substring(0, query.length() - 1);
				scrollTarget = 0;
				return true;
			}
			if (key == GLFW.GLFW_KEY_ESCAPE) {
				query = "";
				searchFocused = false;
				return true;
			}
			if (event.isPaste()) {
				query = (query + minecraft.keyboardHandler.getClipboard()).replace("\n", "");
				return true;
			}
		}
		if (key == GLFW.GLFW_KEY_ESCAPE && page != null) {
			closePage();
			return true;
		}
		if (!searchFocused && ShadowMenu.openKey().matches(event)) {
			onClose();
			return true;
		}
		return super.keyPressed(event);
	}

	@Override
	public boolean charTyped(CharacterEvent event) {
		if (page != null) {
			if (page.charTyped(event)) return true;
			if (page.capturingKeys()) return true;
		}
		if (!event.isAllowedChatCharacter()) return false;
		// typing anywhere searches (like Lunar)
		if (page != null) {
			page.close();
			page = null;
		}
		if (query.length() < 40) query += event.codepointAsString();
		searchFocused = true;
		scrollTarget = 0;
		return true;
	}

	/** True when a HUD module's page is open (for the editor's "back" behaviour). */
	boolean showingElement(HudElement e) {
		return page != null && page.module != null && page.module.id.equals("hud:" + e.id);
	}
}
