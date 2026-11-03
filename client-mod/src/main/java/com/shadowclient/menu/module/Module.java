package com.shadowclient.menu.module;

import com.shadowclient.menu.setting.Setting;
import com.shadowclient.menu.setting.Settings;
import java.util.EnumSet;
import java.util.List;
import java.util.Set;
import java.util.function.BooleanSupplier;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.IntConsumer;
import java.util.function.Supplier;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import org.jetbrains.annotations.Nullable;

/**
 * One card in the mod menu: an on/off switch (Enabled/Disabled), a value you click through
 * (e.g. "Particles: Minimal"), or just a settings shortcut. Shadow's own modules also carry their
 * settings ({@link #options}), shown on the module's page in the menu.
 */
public final class Module {
	public enum Tab { HUD, PVP, HYPIXEL, VISUAL, PERFORMANCE, GAME }

	public final String id;
	public final String name;
	/** Icon file name under assets/shadowmenu/textures/gui/icon/ (without .png). */
	public final String icon;
	public final Set<Tab> tabs;
	private final BooleanSupplier enabled;
	@Nullable private final Consumer<Boolean> setEnabled;
	@Nullable private final Function<Screen, Screen> settings;
	/** Shown instead of Enabled/Disabled for modules without an on/off switch (e.g. "Hold C"). */
	@Nullable public final String hint;
	@Nullable private Supplier<String> value;
	@Nullable private IntConsumer step;
	private boolean fresh;
	@Nullable private ItemStack item;
	@Nullable private Settings options;
	@Nullable private String description;

	public Module(String id, String name, String icon, Set<Tab> tabs, BooleanSupplier enabled, @Nullable Consumer<Boolean> setEnabled,
			@Nullable Function<Screen, Screen> settings, @Nullable String hint) {
		this.id = id;
		this.name = name;
		this.icon = icon;
		this.tabs = tabs.isEmpty() ? EnumSet.noneOf(Tab.class) : EnumSet.copyOf(tabs);
		this.enabled = enabled;
		this.setEnabled = setEnabled;
		this.settings = settings;
		this.hint = hint;
	}

	/** A card whose bar shows a value; clicking steps through the values (+1 forward, -1 back). */
	public static Module cycle(String id, String name, String icon, Set<Tab> tabs, Supplier<String> value, IntConsumer step,
			@Nullable Function<Screen, Screen> settings) {
		Module m = new Module(id, name, icon, tabs, () -> true, null, settings, null);
		m.value = value;
		m.step = step;
		return m;
	}

	/** Marks the module as new in this version (listed under the "New" tab). */
	public Module fresh() {
		fresh = true;
		return this;
	}

	/** Draw this item on the card instead of the white icon (item counters, cooldowns, …). */
	public Module item(Item item) {
		this.item = new ItemStack(item);
		return this;
	}

	/** The module's own settings page (sliders, colours, keys, …). */
	public Module options(Settings options) {
		this.options = options;
		return this;
	}

	public Module describe(@Nullable String text) {
		this.description = text;
		return this;
	}

	@Nullable
	public ItemStack item() {
		return item;
	}

	public boolean isNew() {
		return fresh;
	}

	public boolean toggleable() {
		return setEnabled != null;
	}

	public boolean cycles() {
		return step != null;
	}

	public boolean enabled() {
		try {
			return enabled.getAsBoolean();
		} catch (RuntimeException e) {
			return false;
		}
	}

	public void setEnabled(boolean on) {
		if (setEnabled != null) setEnabled.accept(on);
	}

	public String value() {
		try {
			return value == null ? "" : value.get();
		} catch (RuntimeException e) {
			return "?";
		}
	}

	public void step(int direction) {
		if (step != null) step.accept(direction);
	}

	/** Settings shown on the module's page in the menu (empty for modules that open a Minecraft screen). */
	public List<Setting<?>> options() {
		return options == null ? List.of() : options.all();
	}

	@Nullable
	public Settings settingsGroup() {
		return options;
	}

	@Nullable
	public String description() {
		return description;
	}

	/** Has settings beyond its on/off switch (which the page shows in its header). */
	public boolean hasPage() {
		return options != null && options.all().stream().anyMatch(s -> !s.id.equals("enabled"));
	}

	public boolean hasSettings() {
		return settings != null || hasPage();
	}

	/** A Minecraft (or other) screen for this module, for modules without their own page. */
	@Nullable
	public Screen settings(Screen parent) {
		return settings == null ? null : settings.apply(parent);
	}

	public boolean opensScreen() {
		return settings != null;
	}
}
