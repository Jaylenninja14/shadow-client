package com.shadowclient.menu.setting;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.util.Locale;

/** An ARGB colour, optionally "chroma" (cycles through the rainbow). */
public final class ColorSetting extends Setting<Integer> {
	public final boolean alpha;
	private final boolean chromaDefault;
	private boolean chroma;

	public ColorSetting(String id, String name, int def, boolean alpha) {
		this(id, name, def, alpha, false);
	}

	public ColorSetting(String id, String name, int def, boolean alpha, boolean chromaDefault) {
		super(id, name, alpha ? def : def | 0xFF000000);
		this.alpha = alpha;
		this.chromaDefault = chromaDefault;
		this.chroma = chromaDefault;
	}

	public boolean chroma() {
		return chroma;
	}

	public void setChroma(boolean on) {
		if (on == chroma) return;
		chroma = on;
		if (owner != null) owner.changed();
	}

	/** The colour to draw with now (chroma moves with time; {@code offset} staggers it per element). */
	public int argb(float offset) {
		if (!chroma) return value;
		float hue = ((System.currentTimeMillis() % 4000L) / 4000f + offset) % 1f;
		return (value & 0xFF000000) | (java.awt.Color.HSBtoRGB(hue, 0.75f, 1f) & 0xFFFFFF);
	}

	public int argb() {
		return argb(0);
	}

	@Override
	public void set(Integer v) {
		if (v == null) return;
		super.set(alpha ? v : v | 0xFF000000);
	}

	@Override
	public void reset() {
		chroma = chromaDefault;
		super.set(def);
		if (owner != null) owner.changed();
	}

	public String hex() {
		return alpha ? String.format(Locale.ROOT, "#%08X", value) : String.format(Locale.ROOT, "#%06X", value & 0xFFFFFF);
	}

	@Override
	public JsonElement toJson() {
		JsonObject o = new JsonObject();
		o.addProperty("argb", hex());
		if (chroma) o.addProperty("chroma", true);
		return o;
	}

	@Override
	public void fromJson(JsonElement e) {
		String hex = null;
		if (e.isJsonObject()) {
			JsonObject o = e.getAsJsonObject();
			if (o.has("argb")) hex = o.get("argb").getAsString();
			chroma = o.has("chroma") && o.get("chroma").getAsBoolean();
		} else if (e.isJsonPrimitive()) {
			hex = e.getAsString();
		}
		Integer parsed = parse(hex);
		if (parsed != null) value = alpha ? parsed : parsed | 0xFF000000;
	}

	/** "#RRGGBB" or "#AARRGGBB" (the # is optional); null when it isn't a colour. */
	public static Integer parse(String hex) {
		if (hex == null) return null;
		String h = hex.trim();
		if (h.startsWith("#")) h = h.substring(1);
		try {
			if (h.length() == 6) return 0xFF000000 | Integer.parseUnsignedInt(h, 16);
			if (h.length() == 8) return (int) Long.parseLong(h, 16);
		} catch (NumberFormatException ignored) {
			// not a colour
		}
		return null;
	}
}
