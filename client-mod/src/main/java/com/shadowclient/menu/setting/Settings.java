package com.shadowclient.menu.setting;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.shadowclient.menu.module.MenuConfig;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * The settings of one module, kept in config/shadowmenu.json under "modules" → id. Loaded when
 * created; every change marks the file dirty and it's written a moment later (dragging a slider
 * doesn't write the file on every frame).
 */
public final class Settings {
	public final String id;
	private final List<Setting<?>> list = new ArrayList<>();
	private boolean loading;

	public Settings(String id) {
		this.id = id;
	}

	public <S extends Setting<?>> S add(S s) {
		s.owner = this;
		list.add(s);
		JsonObject stored = MenuConfig.module(id);
		if (stored != null && s.stored() && stored.has(s.id)) {
			loading = true;
			try {
				s.fromJson(stored.get(s.id));
			} catch (RuntimeException ignored) {
				// keeps the default
			} finally {
				loading = false;
			}
		}
		return s;
	}

	public List<Setting<?>> all() {
		return Collections.unmodifiableList(list);
	}

	void changed() {
		if (loading) return;
		JsonObject o = new JsonObject();
		for (Setting<?> s : list) {
			if (!s.stored()) continue;
			JsonElement e = s.toJson();
			if (e != null) o.add(s.id, e);
		}
		MenuConfig.putModule(id, o);
	}

	public void resetAll() {
		loading = true;
		try {
			for (Setting<?> s : list) s.reset();
		} finally {
			loading = false;
		}
		changed();
	}
}
