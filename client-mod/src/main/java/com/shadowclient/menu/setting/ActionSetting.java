package com.shadowclient.menu.setting;

import com.google.gson.JsonElement;
import java.util.function.Consumer;
import net.minecraft.client.gui.screens.Screen;

/** A button on a settings page (open a Minecraft screen, edit the HUD layout, clear a list, …). */
public final class ActionSetting extends Setting<String> {
	/** Gets the menu screen, so the action can open a screen that comes back to it. */
	public final Consumer<Screen> run;

	public ActionSetting(String id, String name, String button, Consumer<Screen> run) {
		super(id, name, button);
		this.run = run;
	}

	public String button() {
		return value;
	}

	@Override
	public boolean stored() {
		return false;
	}

	@Override
	public void reset() {
	}

	@Override
	public JsonElement toJson() {
		return null;
	}

	@Override
	public void fromJson(JsonElement e) {
	}
}
