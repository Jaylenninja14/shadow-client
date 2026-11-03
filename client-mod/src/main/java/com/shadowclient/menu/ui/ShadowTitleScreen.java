package com.shadowclient.menu.ui;

import com.mojang.realmsclient.RealmsMainScreen;
import com.shadowclient.menu.ShadowMenu;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.SharedConstants;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.CreditsAndAttributionScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.multiplayer.JoinMultiplayerScreen;
import net.minecraft.client.gui.screens.multiplayer.SafetyScreen;
import net.minecraft.client.gui.screens.options.AccessibilityOptionsScreen;
import net.minecraft.client.gui.screens.options.LanguageSelectScreen;
import net.minecraft.client.gui.screens.options.OptionsScreen;
import net.minecraft.client.gui.screens.worldselection.SelectWorldScreen;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvents;
import org.jetbrains.annotations.Nullable;

/**
 * Shadow Client's title screen, always shown in place of Minecraft's (see MinecraftMixin): the
 * Shadow panorama, the logo, and big themed buttons that slide in (Singleplayer, Multiplayer,
 * Shadow Menu, Options, Quit), with Realms, the mod list, language and accessibility as small chips.
 */
public final class ShadowTitleScreen extends Screen {
	private static final Identifier LOGO = Identifier.fromNamespaceAndPath("shadowmenu", "icon.png");
	private static final Component COPYRIGHT = Component.translatable("title.credits");
	private static boolean firstShow = true;
	private final Anim anim = new Anim();
	private final List<Entry> buttons = new ArrayList<>();
	private final List<Entry> chips = new ArrayList<>();
	private long openedAt;
	private final boolean fadeIn;

	private record Entry(String label, String icon, @Nullable Component disabledReason, Runnable action) {
	}

	public ShadowTitleScreen() {
		super(Component.translatable("narrator.screen.title"));
		fadeIn = firstShow;
		firstShow = false;
	}

	@Override
	public boolean shouldCloseOnEsc() {
		return false;
	}

	@Override
	public boolean isPauseScreen() {
		return false;
	}

	@Override
	protected void init() {
		if (openedAt == 0) openedAt = System.currentTimeMillis();
		buttons.clear();
		chips.clear();
		buttons.add(new Entry(Component.translatable("menu.singleplayer").getString(), "player", null, () -> minecraft.setScreen(new SelectWorldScreen(this))));
		Component noMultiplayer = minecraft.allowsMultiplayer() ? null : Component.translatable("title.multiplayer.disabled");
		buttons.add(new Entry(Component.translatable("menu.multiplayer").getString(), "server", noMultiplayer,
				() -> minecraft.setScreen(minecraft.options.skipMultiplayerWarning ? new JoinMultiplayerScreen(this) : new SafetyScreen(this))));
		buttons.add(new Entry("Shadow Menu", "grid", null, () -> minecraft.setScreen(new ModMenuScreen(this))));
		buttons.add(new Entry(Component.translatable("menu.options").getString().replace("...", "").replace("…", ""), "gear", null,
				() -> minecraft.setScreen(new OptionsScreen(this, minecraft.options))));
		buttons.add(new Entry(Component.translatable("menu.quit").getString(), "exit", null, () -> minecraft.stop()));

		chips.add(new Entry(Component.translatable("menu.online").getString(), "cloud", noMultiplayer, () -> minecraft.setScreen(new RealmsMainScreen(this))));
		chips.add(new Entry("Mods", "pack", null, () -> minecraft.setScreen(ModMenuScreen.mods(this))));
		chips.add(new Entry("Language", "chat", null, () -> minecraft.setScreen(new LanguageSelectScreen(this, minecraft.options, minecraft.getLanguageManager()))));
		chips.add(new Entry("Accessibility", "player", null, () -> minecraft.setScreen(new AccessibilityOptionsScreen(this, minecraft.options))));
	}

	private float since(long delayMs, float durationMs) {
		if (Theme.speed() <= 0) return 1;
		return Draw.ease((System.currentTimeMillis() - openedAt - delayMs / Theme.speed()) / (durationMs / Theme.speed()));
	}

	@Override
	public void renderBackground(GuiGraphics g, int mx, int my, float partial) {
		renderPanorama(g, partial);
		int accent = Theme.accent();
		// darken the left side behind the buttons, a little accent glow from the bottom
		g.fillGradient(0, 0, width, height, 0x30000000, 0x90000000);
		for (int i = 0; i < 8; i++) {
			int x = Math.round(width * 0.55f * (1 - i / 8f));
			g.fill(0, 0, x, height, 0x0C000000);
		}
		g.fillGradient(0, height * 2 / 3, width, height, Draw.withAlpha(accent, 0), Draw.withAlpha(accent, 0x30));
	}

	@Override
	public void render(GuiGraphics g, int mx, int my, float partial) {
		anim.frame();
		int accent = Theme.accent();
		float logoIn = since(fadeIn ? 250 : 0, 500);
		int left = Math.max(24, width / 10);
		int logoSize = height < 300 ? 32 : 44;
		int top = Math.max(16, height / 2 - 120);
		// logo
		float ly = top + (1 - logoIn) * -10;
		g.pose().pushMatrix();
		g.pose().translate(left, ly);
		g.pose().scale(logoSize / 128f, logoSize / 128f);
		g.blit(RenderPipelines.GUI_TEXTURED, LOGO, 0, 0, 0, 0, 128, 128, 128, 128, Draw.fade(0xFFFFFFFF, logoIn));
		g.pose().popMatrix();
		float big = height < 300 ? 2.2f : 3f;
		int tx = left + logoSize + 10;
		int tyy = Math.round(ly + logoSize / 2f - 4 * big);
		Draw.scaledText(g, font, "SHADOW", tx, tyy, big, Draw.fade(0xFFFFFFFF, logoIn), true);
		Draw.scaledText(g, font, "CLIENT", tx + font.width("SHADOW ") * big, tyy, big, Draw.fade(accent, logoIn), true);
		String tagline = "Minecraft " + SharedConstants.getCurrentVersion().name() + " · built for smooth PvP";
		g.drawString(font, tagline, tx + 1, Math.round(ly + logoSize / 2f + 4 * big + 3), Draw.fade(0xFFC8C8D4, logoIn), true);

		// big buttons
		int bw = Math.min(220, width - left * 2), bh = height < 300 ? 20 : 24, gap = height < 300 ? 4 : 6;
		int by = top + logoSize + (height < 300 ? 18 : 30);
		for (int i = 0; i < buttons.size(); i++) {
			Entry e = buttons.get(i);
			float in = since((fadeIn ? 400 : 60) + i * 60L, 380);
			int x = Math.round(left + (1 - in) * -30), y = by + i * (bh + gap);
			boolean disabled = e.disabledReason() != null;
			boolean hover = !disabled && Draw.inside(mx, my, x, y, bw, bh);
			float hv = anim.get("btn:" + i, hover ? 1 : 0, 14);
			boolean quit = i == buttons.size() - 1;
			int hoverColor = quit ? 0xFFD94B4B : accent;
			Draw.round(g, x, y, bw, bh, 6, Draw.fade(Draw.mix(0xB0101016, 0xD0101016, hv), in));
			if (hv > 0.01f) Draw.round(g, x, y, Math.max(12, bw * Draw.ease(hv)), bh, 6, Draw.fade(Draw.mix(hoverColor, 0xFF000000, 0.15f), in * hv));
			Draw.roundOutline(g, x, y, bw, bh, 6, Draw.fade(Draw.mix(0x40FFFFFF, hoverColor, hv), in));
			int text = disabled ? 0xFF777780 : Draw.mix(0xFFEDEDF4, quit ? 0xFFFFFFFF : Theme.onAccent() == 0xFFFFFFFF ? 0xFFFFFFFF : 0xFF15121F, hv);
			float nudge = hv * 3;
			Draw.icon(g, e.icon(), x + 10 + nudge, y + (bh - 12) / 2f, 12, Draw.fade(text, in));
			g.drawString(font, e.label(), Math.round(x + 30 + nudge), y + (bh - 8) / 2, Draw.fade(text, in), false);
			if (disabled && Draw.inside(mx, my, x, y, bw, bh)) g.setTooltipForNextFrame(font, e.disabledReason(), mx, my);
		}

		// small chips, top right
		int cxr = width - 8;
		for (int i = chips.size() - 1; i >= 0; i--) {
			Entry e = chips.get(i);
			int w = font.width(e.label()) + 26;
			int x = cxr - w, y = 8;
			float in = since(fadeIn ? 700 : 150, 400);
			boolean hover = e.disabledReason() == null && Draw.inside(mx, my, x, y, w, 18);
			float hv = anim.get("chip:" + i, hover ? 1 : 0, 14);
			Draw.round(g, x, y, w, 18, 5, Draw.fade(Draw.mix(0xA0101016, Draw.mix(0xFF101016, accent, 0.5f), hv), in));
			Draw.icon(g, e.icon(), x + 6, y + 4, 10, Draw.fade(e.disabledReason() == null ? 0xFFE0E0EA : 0xFF777780, in));
			g.drawString(font, e.label(), x + 20, y + 5, Draw.fade(e.disabledReason() == null ? 0xFFFFFFFF : 0xFF777780, in), false);
			cxr = x - 4;
		}

		// corners: version and copyright
		float foot = since(fadeIn ? 800 : 200, 400);
		g.drawString(font, "Shadow Client " + ShadowMenu.version(), 6, height - 20, Draw.fade(accent, foot), true);
		g.drawString(font, "Minecraft " + SharedConstants.getCurrentVersion().name() + " (Fabric)", 6, height - 10, Draw.fade(0xFFB4B4C0, foot), true);
		int cw = font.width(COPYRIGHT);
		boolean credits = Draw.inside(mx, my, width - cw - 4, height - 12, cw + 4, 12);
		g.drawString(font, COPYRIGHT, width - cw - 4, height - 10, Draw.fade(credits ? 0xFFFFFFFF : 0xFFB4B4C0, foot), true);
		if (credits) g.fill(width - cw - 4, height - 1, width - 4, height, Draw.fade(0xFFFFFFFF, foot));
		String hint = "Right Shift: Shadow Menu";
		g.drawString(font, hint, width - font.width(hint) - 4, height - 20, Draw.fade(0xFF8A8A98, foot), true);

		if (fadeIn) {
			// first start: fade in from black like Minecraft's own title screen
			float f = since(0, 900);
			if (f < 1) g.fill(0, 0, width, height, Draw.fade(0xFF000000, 1 - f));
		}
	}

	private void click() {
		minecraft.getSoundManager().play(SimpleSoundInstance.forUI(SoundEvents.UI_BUTTON_CLICK, 1.0F));
	}

	@Override
	public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) {
		if (event.button() != 0) return false;
		double mx = event.x(), my = event.y();
		int left = Math.max(24, width / 10);
		int logoSize = height < 300 ? 32 : 44;
		int top = Math.max(16, height / 2 - 120);
		int bw = Math.min(220, width - left * 2), bh = height < 300 ? 20 : 24, gap = height < 300 ? 4 : 6;
		int by = top + logoSize + (height < 300 ? 18 : 30);
		for (int i = 0; i < buttons.size(); i++) {
			Entry e = buttons.get(i);
			if (Draw.inside(mx, my, left, by + i * (bh + gap), bw, bh) && e.disabledReason() == null) {
				click();
				e.action().run();
				return true;
			}
		}
		int cxr = width - 8;
		for (int i = chips.size() - 1; i >= 0; i--) {
			Entry e = chips.get(i);
			int w = font.width(e.label()) + 26;
			if (Draw.inside(mx, my, cxr - w, 8, w, 18) && e.disabledReason() == null) {
				click();
				e.action().run();
				return true;
			}
			cxr -= w + 4;
		}
		int cw = font.width(COPYRIGHT);
		if (Draw.inside(mx, my, width - cw - 4, height - 12, cw + 4, 12)) {
			click();
			minecraft.setScreen(new CreditsAndAttributionScreen(this));
			return true;
		}
		return false;
	}

	@Override
	public boolean keyPressed(KeyEvent event) {
		if (ShadowMenu.openKey() != null && ShadowMenu.openKey().matches(event)) {
			minecraft.setScreen(new ModMenuScreen(this));
			return true;
		}
		return super.keyPressed(event);
	}
}
