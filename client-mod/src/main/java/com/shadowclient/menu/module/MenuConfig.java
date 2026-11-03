package com.shadowclient.menu.module;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import org.jetbrains.annotations.Nullable;
import net.fabricmc.loader.api.FabricLoader;

/**
 * The menu's own settings in config/shadowmenu.json: hearted modules, the settings FPS Boost
 * replaced (so switching it off puts them back) and every module's settings ("modules" → id, HUD
 * positions included). The file from the old name (sunnymenu.json) is read once if there's no new
 * one. Module settings are written a moment after they change ({@link #flush}).
 */
public final class MenuConfig {
	private static final Path DIR = FabricLoader.getInstance().getConfigDir();
	private static final Path FILE = DIR.resolve("shadowmenu.json");
	private static final Path LEGACY = DIR.resolve("sunnymenu.json");
	private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();
	private static JsonObject data;
	private static boolean dirty;
	private static long dirtySince;

	private MenuConfig() {
	}

	private static JsonObject load() {
		if (data != null) return data;
		data = new JsonObject();
		for (Path p : new Path[] {FILE, LEGACY}) {
			try {
				if (Files.isRegularFile(p)) {
					JsonObject o = GSON.fromJson(Files.readString(p, StandardCharsets.UTF_8), JsonObject.class);
					if (o != null) data = o;
					break;
				}
			} catch (IOException | RuntimeException ignored) {
				// a broken file just means defaults
			}
		}
		if (!data.has("favorites") || !data.get("favorites").isJsonArray()) data.add("favorites", new JsonArray());
		return data;
	}

	private static void save() {
		dirty = false;
		try {
			Files.createDirectories(DIR);
			Files.writeString(FILE, GSON.toJson(load()), StandardCharsets.UTF_8);
		} catch (IOException ignored) {
			// kept for this session
		}
	}

	public static boolean isFavorite(String id) {
		for (JsonElement e : load().getAsJsonArray("favorites")) {
			if (e.isJsonPrimitive() && e.getAsString().equals(id)) return true;
		}
		return false;
	}

	public static void toggleFavorite(String id) {
		JsonArray old = load().getAsJsonArray("favorites");
		JsonArray next = new JsonArray();
		boolean removed = false;
		for (JsonElement e : old) {
			if (e.isJsonPrimitive() && e.getAsString().equals(id)) removed = true;
			else next.add(e);
		}
		if (!removed) next.add(id);
		data.add("favorites", next);
		save();
	}

	@Nullable
	public static JsonObject object(String key) {
		JsonElement e = load().get(key);
		return e != null && e.isJsonObject() ? e.getAsJsonObject() : null;
	}

	public static void putObject(String key, @Nullable JsonObject value) {
		if (value == null) load().remove(key);
		else load().add(key, value);
		save();
	}

	/** One module's stored settings ("modules" → id), or null. */
	@Nullable
	public static JsonObject module(String id) {
		JsonObject all = object("modules");
		JsonElement e = all == null ? null : all.get(id);
		return e != null && e.isJsonObject() ? e.getAsJsonObject() : null;
	}

	public static void putModule(String id, JsonObject value) {
		JsonObject all = object("modules");
		if (all == null) {
			all = new JsonObject();
			load().add("modules", all);
		}
		all.add(id, value);
		if (!dirty) dirtySince = System.currentTimeMillis();
		dirty = true;
	}

	/** Writes pending changes once they're half a second old ({@code force}: now). */
	public static void flush(boolean force) {
		if (dirty && (force || System.currentTimeMillis() - dirtySince > 500)) save();
	}
}
