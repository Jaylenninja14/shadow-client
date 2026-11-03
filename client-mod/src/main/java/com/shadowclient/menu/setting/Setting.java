package com.shadowclient.menu.setting;

import com.google.gson.JsonElement;
import java.util.function.BooleanSupplier;
import org.jetbrains.annotations.Nullable;

/**
 * One option on a module's settings page (a switch, slider, colour, key, choice, text or button).
 * Values live in memory and are written to config/shadowmenu.json by the module's {@link Settings}.
 */
public abstract class Setting<T> {
	public final String id;
	public final String name;
	@Nullable public String description;
	/** Heading the setting is listed under on the page (null: at the top, before any heading). */
	@Nullable public String group;
	protected final T def;
	protected T value;
	@Nullable Settings owner;
	private BooleanSupplier visible = () -> true;

	protected Setting(String id, String name, T def) {
		this.id = id;
		this.name = name;
		this.def = def;
		this.value = def;
	}

	public T get() {
		return value;
	}

	public void set(T v) {
		if (v == null || v.equals(value)) return;
		value = v;
		if (owner != null) owner.changed();
	}

	public T defaultValue() {
		return def;
	}

	public void reset() {
		set(def);
	}

	/** Only shown when {@code when} is true (e.g. background colour only with a background). */
	@SuppressWarnings("unchecked")
	public <S extends Setting<T>> S showIf(BooleanSupplier when) {
		this.visible = when;
		return (S) this;
	}

	@SuppressWarnings("unchecked")
	public <S extends Setting<T>> S describe(String text) {
		this.description = text;
		return (S) this;
	}

	@SuppressWarnings("unchecked")
	public <S extends Setting<T>> S group(String heading) {
		this.group = heading;
		return (S) this;
	}

	public boolean visible() {
		try {
			return visible.getAsBoolean();
		} catch (RuntimeException e) {
			return true;
		}
	}

	/** Whether the setting is written to the config (buttons and key mappings aren't). */
	public boolean stored() {
		return true;
	}

	@Nullable
	public abstract JsonElement toJson();

	/** Read a stored value; anything unreadable keeps the default. */
	public abstract void fromJson(JsonElement e);
}
