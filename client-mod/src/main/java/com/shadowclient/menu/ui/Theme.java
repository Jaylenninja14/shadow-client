package com.shadowclient.menu.ui;

import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ChoiceSetting;
import com.shadowclient.menu.setting.ColorSetting;
import com.shadowclient.menu.setting.NumberSetting;
import com.shadowclient.menu.setting.Settings;
import java.util.List;

/**
 * The menu's look: a base theme (Dark, Midnight, Black, Light) and an accent colour. Surfaces are
 * tinted a little towards the accent, like the Shadow Client launcher, so any accent still matches.
 */
public final class Theme {
	public static final Settings SETTINGS = new Settings("client");
	public static final ChoiceSetting BASE = SETTINGS.add(new ChoiceSetting("theme", "Theme", "dark",
			List.of("dark", "midnight", "black", "light"), List.of("Dark", "Midnight", "Black", "Light")).group("Look"));
	public static final ColorSetting ACCENT = SETTINGS.add(new ColorSetting("accent", "Accent colour", 0xFF9B7BFF, false).group("Look"));
	public static final BoolSetting BLUR = SETTINGS.add(new BoolSetting("blur", "Blur the game behind the menu", true).group("Look"));
	public static final ChoiceSetting CARDS = SETTINGS.add(new ChoiceSetting("cards", "Card size", "normal",
			List.of("compact", "normal", "large"), List.of("Compact", "Normal", "Large")).group("Look"));
	public static final BoolSetting ANIMATIONS = SETTINGS.add(new BoolSetting("animations", "Animations", true).group("Motion"));
	public static final NumberSetting SPEED = SETTINGS.add(new NumberSetting("animation-speed", "Animation speed", 1, 0.5, 2, 0.1)
			.format(v -> Math.round(v * 100) + "%").group("Motion").showIf(ANIMATIONS::on));
	public static final BoolSetting SOUNDS = SETTINGS.add(new BoolSetting("sounds", "Click sounds", true).group("Motion"));

	/** Accent presets offered next to the colour picker. */
	public static final int[] PRESETS = {0xFF9B7BFF, 0xFFFF8F1F, 0xFF3FA9FF, 0xFF2ED3B7, 0xFF7AC943, 0xFFFF5C7A, 0xFFFFD24A, 0xFFE0E0E0};

	private Theme() {
	}

	private record Palette(int bg, int panel, int side, int card, int hover, int line, int chip, int text, int dim, int faint, boolean light) {
	}

	private static final Palette DARK = new Palette(0xFF07070B, 0xFF0D0D12, 0xFF09090D, 0xFF14141A, 0xFF1B1B23, 0xFF262630, 0xFF1D1D25, 0xFFF4F4F8, 0xFFB0B0BE, 0xFF70707E, false);
	private static final Palette MIDNIGHT = new Palette(0xFF060A14, 0xFF0B1220, 0xFF080E1A, 0xFF111B2E, 0xFF17233A, 0xFF22304A, 0xFF152037, 0xFFEFF3FB, 0xFFA6B2C8, 0xFF66738C, false);
	private static final Palette BLACK = new Palette(0xFF000000, 0xFF050505, 0xFF000000, 0xFF0C0C0C, 0xFF151515, 0xFF1F1F1F, 0xFF121212, 0xFFFFFFFF, 0xFFAAAAAA, 0xFF6A6A6A, false);
	private static final Palette LIGHT = new Palette(0xFFE9E9F0, 0xFFF6F6FA, 0xFFEEEEF4, 0xFFFFFFFF, 0xFFF1EFFA, 0xFFD9D7E4, 0xFFE6E4EF, 0xFF16161D, 0xFF4E4E5C, 0xFF8A8A98, true);

	private static Palette p() {
		return switch (BASE.get()) {
			case "midnight" -> MIDNIGHT;
			case "black" -> BLACK;
			case "light" -> LIGHT;
			default -> DARK;
		};
	}

	private static int tint(int surface, float amount) {
		return Draw.mix(surface, accent(), amount);
	}

	public static boolean light() {
		return p().light();
	}

	public static int accent() {
		return ACCENT.argb() | 0xFF000000;
	}

	/** Text on an accent-coloured surface. */
	public static int onAccent() {
		return Draw.luma(accent()) > 0.62f ? 0xFF15121F : 0xFFFFFFFF;
	}

	/** Accent mixed towards the text colour (readable accent text). */
	public static int accentText() {
		return Draw.mix(accent(), light() ? 0xFF000000 : 0xFFFFFFFF, 0.25f);
	}

	public static int backdrop() {
		return light() ? 0x66101018 : 0x88000000;
	}

	public static int panel() {
		return tint(p().panel(), 0.05f);
	}

	public static int sidebar() {
		return tint(p().side(), 0.05f);
	}

	public static int bg() {
		return tint(p().bg(), 0.04f);
	}

	public static int card() {
		return tint(p().card(), 0.07f);
	}

	public static int cardHover() {
		return tint(p().hover(), 0.10f);
	}

	public static int line() {
		return tint(p().line(), 0.12f);
	}

	public static int chip() {
		return tint(p().chip(), 0.08f);
	}

	public static int chipHover() {
		return tint(p().hover(), 0.16f);
	}

	public static int text() {
		return p().text();
	}

	public static int dim() {
		return p().dim();
	}

	public static int faint() {
		return p().faint();
	}

	public static int off() {
		return light() ? 0xFFD4D2DE : tint(0xFF2A2A34, 0.05f);
	}

	public static int heart() {
		return 0xFFFF5C7A;
	}

	/** Seconds-based speed factor for animations (0 = instant). */
	public static float speed() {
		return ANIMATIONS.on() ? SPEED.f() : 0;
	}
}
