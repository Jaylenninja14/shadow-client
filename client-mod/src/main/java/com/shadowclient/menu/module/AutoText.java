package com.shadowclient.menu.module;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import com.mojang.blaze3d.platform.InputConstants;
import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.ui.AutoTextScreen;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.List;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.world.item.Items;

/**
 * Auto Text: five keys that each send a chat message or command (like Lunar's Auto Text Hotkey).
 * The messages live in config/shadowmenu.json ("autoText"); keys are unbound until you set them in
 * Controls. A one-second gap between sends keeps servers from kicking you for spam.
 */
public final class AutoText {
	public static final int SLOTS = 5;
	private static final String[] DEFAULTS = {"/gg", "gg wp", "/l", "/p warp", ""};
	private static final List<KeyMapping> KEYS = new ArrayList<>();
	private static long lastSent;

	private AutoText() {
	}

	public static void register(KeyMapping.Category category) {
		for (int i = 0; i < SLOTS; i++) {
			KEYS.add(KeyBindingHelper.registerKeyBinding(new KeyMapping("key.shadowmenu.autotext" + (i + 1), InputConstants.UNKNOWN.getValue(), category)));
		}
	}

	public static List<String> messages() {
		List<String> out = new ArrayList<>();
		JsonObject o = MenuConfig.object("autoText");
		JsonArray a = o != null && o.has("messages") ? o.getAsJsonArray("messages") : null;
		for (int i = 0; i < SLOTS; i++) out.add(a != null && i < a.size() ? a.get(i).getAsString() : DEFAULTS[i]);
		return out;
	}

	public static void save(List<String> messages) {
		JsonObject o = new JsonObject();
		JsonArray a = new JsonArray();
		messages.forEach(a::add);
		o.add("messages", a);
		MenuConfig.putObject("autoText", o);
	}

	public static void tick(Minecraft mc) {
		for (int i = 0; i < KEYS.size(); i++) {
			while (KEYS.get(i).consumeClick()) {
				if (Tweaks.is("auto-text") && mc.player != null && mc.screen == null) send(mc, messages().get(i));
			}
		}
	}

	private static void send(Minecraft mc, String message) {
		String m = message.trim();
		long now = System.currentTimeMillis();
		if (m.isEmpty() || now - lastSent < 1000) return;
		lastSent = now;
		if (m.startsWith("/")) mc.player.connection.sendCommand(m.substring(1));
		else mc.player.connection.sendChat(m);
	}

	public static void addTo(List<Module> out) {
		out.add(new Module("auto-text", "Auto Text Hotkeys", "chat", EnumSet.of(Tab.PVP, Tab.GAME, Tab.HYPIXEL), () -> Tweaks.is("auto-text"),
				v -> Tweaks.set("auto-text", v), AutoTextScreen::new, null).item(Items.WRITABLE_BOOK));
	}
}
