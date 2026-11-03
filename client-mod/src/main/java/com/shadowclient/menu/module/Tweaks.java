package com.shadowclient.menu.module;

import com.google.gson.JsonObject;

/** Switches and small values read by the menu's mixins and modules, kept in config/shadowmenu.json ("tweaks"). */
public final class Tweaks {
	public static final String CHAT_TIMESTAMPS = "chat-timestamps";
	public static final String LONG_CHAT = "long-chat";
	public static final String NO_TOTEM_ANIMATION = "no-totem-animation";

	private Tweaks() {
	}

	public static boolean is(String id) {
		JsonObject o = MenuConfig.object("tweaks");
		return o != null && o.has(id) && o.get(id).isJsonPrimitive() && o.get(id).getAsJsonPrimitive().isBoolean() && o.get(id).getAsBoolean();
	}

	public static void set(String id, boolean on) {
		JsonObject next = copy();
		next.addProperty(id, on);
		MenuConfig.putObject("tweaks", next);
	}

	public static String str(String id, String def) {
		JsonObject o = MenuConfig.object("tweaks");
		return o != null && o.has(id) && o.get(id).isJsonPrimitive() ? o.get(id).getAsString() : def;
	}

	public static void setStr(String id, String value) {
		JsonObject next = copy();
		next.addProperty(id, value);
		MenuConfig.putObject("tweaks", next);
	}

	private static JsonObject copy() {
		JsonObject o = MenuConfig.object("tweaks");
		return o == null ? new JsonObject() : o.deepCopy();
	}
}
