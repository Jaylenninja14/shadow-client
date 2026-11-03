package com.shadowclient.menu.feature;

import com.shadowclient.menu.module.Module;
import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.Setting;
import com.shadowclient.menu.setting.Settings;
import java.util.Set;
import net.minecraft.world.item.Item;
import org.jetbrains.annotations.Nullable;

/**
 * A module Shadow Menu does itself (zoom, freelook, motion blur, …): an on/off switch plus the
 * settings on its page, kept in config/shadowmenu.json ("modules" → id).
 */
public abstract class Feature {
	public final String id;
	public final String name;
	public final String icon;
	public final Set<Tab> tabs;
	public final Settings settings;
	public final BoolSetting enabled;
	@Nullable private String description;
	@Nullable private Item item;

	protected Feature(String id, String name, String icon, Set<Tab> tabs, boolean onByDefault) {
		this.id = id;
		this.name = name;
		this.icon = icon;
		this.tabs = tabs;
		this.settings = new Settings(id);
		this.enabled = settings.add(new BoolSetting("enabled", "Enabled", onByDefault));
	}

	public boolean on() {
		return enabled.on();
	}

	protected <S extends Setting<?>> S add(S s) {
		return settings.add(s);
	}

	protected Feature describe(String text) {
		this.description = text;
		return this;
	}

	protected Feature item(Item item) {
		this.item = item;
		return this;
	}

	/** Called after the switch is flipped. */
	protected void toggled(boolean on) {
	}

	public void setOn(boolean on) {
		enabled.set(on);
		toggled(on);
	}

	public Module card() {
		Module m = new Module(id, name, icon, tabs, this::on, this::setOn, null, null).options(settings).describe(description).fresh();
		return item != null ? m.item(item) : m;
	}
}
