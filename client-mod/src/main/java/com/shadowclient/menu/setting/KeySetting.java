package com.shadowclient.menu.setting;

import com.google.gson.JsonElement;
import com.mojang.blaze3d.platform.InputConstants;
import java.util.function.Supplier;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import org.jetbrains.annotations.Nullable;

/**
 * A key for a module (zoom, freelook, …). It edits the module's real key mapping, so the same key
 * shows in Controls → Shadow Menu and is saved in options.txt like any other key.
 */
public final class KeySetting extends Setting<String> {
	private final Supplier<KeyMapping> mapping;

	public KeySetting(String id, String name, Supplier<KeyMapping> mapping) {
		super(id, name, "");
		this.mapping = mapping;
	}

	@Nullable
	public KeyMapping mapping() {
		return mapping.get();
	}

	public String keyName() {
		KeyMapping k = mapping();
		if (k == null) return "?";
		return k.isUnbound() ? "None" : k.getTranslatedKeyMessage().getString();
	}

	public void bind(InputConstants.Key key) {
		KeyMapping k = mapping();
		if (k == null) return;
		k.setKey(key);
		KeyMapping.resetMapping();
		Minecraft.getInstance().options.save();
	}

	@Override
	public void reset() {
		KeyMapping k = mapping();
		if (k != null) bind(k.getDefaultKey());
	}

	@Override
	public boolean stored() {
		return false;
	}

	@Override
	public JsonElement toJson() {
		return null;
	}

	@Override
	public void fromJson(JsonElement e) {
	}
}
