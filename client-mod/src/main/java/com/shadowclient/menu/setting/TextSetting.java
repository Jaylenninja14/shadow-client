package com.shadowclient.menu.setting;

import com.google.gson.JsonElement;
import com.google.gson.JsonPrimitive;

/** A short line of text (a nickname, a chat message, a time format). */
public final class TextSetting extends Setting<String> {
	public final int maxLength;

	public TextSetting(String id, String name, String def, int maxLength) {
		super(id, name, def);
		this.maxLength = maxLength;
	}

	@Override
	public void set(String v) {
		if (v == null) return;
		super.set(v.length() > maxLength ? v.substring(0, maxLength) : v);
	}

	@Override
	public JsonElement toJson() {
		return new JsonPrimitive(value);
	}

	@Override
	public void fromJson(JsonElement e) {
		if (e.isJsonPrimitive()) {
			String s = e.getAsString();
			value = s.length() > maxLength ? s.substring(0, maxLength) : s;
		}
	}
}
