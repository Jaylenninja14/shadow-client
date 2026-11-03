package com.shadowclient.menu.setting;

import com.google.gson.JsonElement;
import com.google.gson.JsonPrimitive;
import java.util.List;

/** One of a few named values, clicked through (left: next, right: previous). */
public final class ChoiceSetting extends Setting<String> {
	public final List<String> values;
	public final List<String> labels;

	public ChoiceSetting(String id, String name, String def, List<String> values, List<String> labels) {
		super(id, name, def);
		this.values = values;
		this.labels = labels;
	}

	/** Values that are their own labels. */
	public ChoiceSetting(String id, String name, String def, String... values) {
		this(id, name, def, List.of(values), List.of(values));
	}

	public int index() {
		int i = values.indexOf(value);
		return Math.max(0, i);
	}

	public String label() {
		return labels.get(index());
	}

	public boolean is(String v) {
		return value.equals(v);
	}

	public void step(int dir) {
		set(values.get(Math.floorMod(index() + dir, values.size())));
	}

	@Override
	public JsonElement toJson() {
		return new JsonPrimitive(value);
	}

	@Override
	public void fromJson(JsonElement e) {
		if (e.isJsonPrimitive() && values.contains(e.getAsString())) value = e.getAsString();
	}
}
