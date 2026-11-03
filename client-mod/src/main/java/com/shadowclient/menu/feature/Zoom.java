package com.shadowclient.menu.feature;

import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ChoiceSetting;
import com.shadowclient.menu.setting.KeySetting;
import com.shadowclient.menu.setting.NumberSetting;
import java.util.EnumSet;
import java.util.List;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;

/** Hold the zoom key (C) to zoom in; scroll while zooming to zoom further. Smooth by default. */
public final class Zoom extends Feature {
	public static final Zoom I = new Zoom();
	static KeyMapping key;
	final KeySetting keySetting = add(new KeySetting("key", "Zoom key", () -> key));
	final ChoiceSetting mode = add(new ChoiceSetting("mode", "Key mode", "hold", List.of("hold", "toggle"), List.of("Hold", "Toggle")));
	final NumberSetting factor = add(new NumberSetting("factor", "Zoom", 4, 1.5, 20, 0.5).format(v -> (v == Math.rint(v) ? String.valueOf((int) Math.round(v)) : String.valueOf(v)) + "x"));
	final BoolSetting scroll = add(new BoolSetting("scroll", "Scroll to zoom further", true));
	final BoolSetting smooth = add(new BoolSetting("smooth", "Smooth zoom", true));
	final BoolSetting lowerSensitivity = add(new BoolSetting("sensitivity", "Slower mouse while zoomed", true));
	final BoolSetting cinematic = add(new BoolSetting("cinematic", "Cinematic camera while zoomed", false));
	final BoolSetting hideHand = add(new BoolSetting("hide-hand", "Hide your hand while zoomed", false));
	private boolean active, toggled, madeCinematic;
	private double extra = 1;
	private float current = 1;
	private long lastNanos = System.nanoTime();

	private Zoom() {
		super("zoom", "Zoom", "zoom", EnumSet.of(Tab.PVP, Tab.VISUAL), true);
		describe("Hold C to zoom in. Scroll while zooming to zoom further.");
	}

	static void tick(Minecraft mc) {
		Zoom z = I;
		if (key == null) return;
		boolean want;
		if (z.mode.is("toggle")) {
			while (key.consumeClick()) z.toggled = !z.toggled;
			want = z.toggled;
		} else {
			while (key.consumeClick()) {
				// clicks are only for toggle mode
			}
			want = key.isDown();
		}
		want &= z.on() && mc.screen == null && mc.player != null;
		if (want != z.active) {
			z.active = want;
			if (want) {
				z.extra = 1;
				if (z.cinematic.on() && !mc.options.smoothCamera) {
					mc.options.smoothCamera = true;
					z.madeCinematic = true;
				}
			} else {
				z.toggled = false;
				if (z.madeCinematic) {
					mc.options.smoothCamera = false;
					z.madeCinematic = false;
				}
			}
		}
	}

	private float target() {
		return active ? (float) (factor.get() * extra) : 1f;
	}

	/** How much the field of view is divided by now (1 = no zoom). */
	public static float divisor() {
		Zoom z = I;
		long now = System.nanoTime();
		float dt = Math.min(0.1f, (now - z.lastNanos) / 1e9f);
		z.lastNanos = now;
		float t = z.target();
		if (!z.smooth.on()) z.current = t;
		else {
			z.current += (t - z.current) * (1 - (float) Math.exp(-dt * 14));
			if (Math.abs(t - z.current) < 0.001f) z.current = t;
		}
		return z.current;
	}

	public static boolean zooming() {
		return I.active;
	}

	/** Scrolling while zoomed zooms further; returns true when the scroll was used. */
	public static boolean scrolled(double amount) {
		Zoom z = I;
		if (!z.active || !z.scroll.on() || amount == 0) return false;
		z.extra = Math.max(0.25, Math.min(6, z.extra * (amount > 0 ? 1.25 : 0.8)));
		return true;
	}

	public static double sensitivity() {
		Zoom z = I;
		return z.active && z.lowerSensitivity.on() ? 1.0 / Math.max(1, z.current) : 1.0;
	}

	public static boolean hidesHand() {
		return I.active && I.hideHand.on();
	}
}
