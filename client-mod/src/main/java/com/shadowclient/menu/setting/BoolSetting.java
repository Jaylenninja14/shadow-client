package com.shadowclient.menu.setting;

import com.google.gson.JsonElement;
import com.google.gson.JsonPrimitive;

/** An on/off switch. */
public final class BoolSetting extends Setting<Boolean> {
	public BoolSetting(String id, String name, boolean def) {
		super(id, name, def);
	}

	public boolean on() {
		return value;
	}

	public void toggle() {
		set(!value);
	}

	@Override
	public JsonElement toJson() {
		return new JsonPrimitive(value);
	}

	@Override
	public void fromJson(JsonElement e) {
		if (e.isJsonPrimitive() && e.getAsJsonPrimitive().isBoolean()) value = e.getAsBoolean();
	}
}
