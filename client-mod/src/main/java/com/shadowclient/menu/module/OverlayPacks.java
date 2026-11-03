package com.shadowclient.menu.module;

import com.shadowclient.menu.module.Module.Tab;
import java.util.EnumSet;
import java.util.List;
import java.util.Set;
import net.fabricmc.fabric.api.resource.v1.ResourceLoader;
import net.fabricmc.fabric.api.resource.v1.pack.PackActivationType;
import net.fabricmc.loader.api.ModContainer;
import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.server.packs.repository.PackRepository;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.Items;
import org.jetbrains.annotations.Nullable;

/**
 * Modules that are small resource packs inside the mod (resourcepacks/&lt;id&gt;/): switching one
 * selects or deselects the pack and reloads resources, like the Resource Packs screen does.
 * Textures are Shadow Client's own (tools/PackGen.java), not Mojang's.
 */
public final class OverlayPacks {
	private OverlayPacks() {
	}

	private record Pack(String id, String name, String icon, Set<Tab> tabs, @Nullable Item item, boolean onByDefault) {
	}

	private static final List<Pack> PACKS = List.of(
			new Pack("shadow_background", "Shadow Background", "pack", EnumSet.of(Tab.VISUAL), Items.ENDER_EYE, true),
			new Pack("no_pumpkin", "No Pumpkin Overlay", "fullbright", EnumSet.of(Tab.PVP, Tab.VISUAL), Items.CARVED_PUMPKIN, false),
			new Pack("no_underwater", "No Underwater Overlay", "rain", EnumSet.of(Tab.PVP, Tab.VISUAL), Items.WATER_BUCKET, false),
			new Pack("no_powder_snow", "No Powder Snow Overlay", "rain", EnumSet.of(Tab.PVP, Tab.VISUAL), Items.POWDER_SNOW_BUCKET, false),
			new Pack("clear_glass", "Clear Glass", "glass", EnumSet.of(Tab.PVP, Tab.VISUAL), Items.GLASS, false),
			new Pack("clear_water", "Clear Water", "rain", EnumSet.of(Tab.PVP, Tab.VISUAL), Items.HEART_OF_THE_SEA, false),
			new Pack("shadow_hotbar", "Shadow Hotbar", "hotbar", EnumSet.of(Tab.HUD, Tab.VISUAL), Items.AMETHYST_SHARD, false));

	private static String packId(Pack p) {
		return "shadowmenu:" + p.id();
	}

	public static void register(ModContainer mod) {
		for (Pack p : PACKS) {
			ResourceLoader.registerBuiltinPack(Identifier.fromNamespaceAndPath("shadowmenu", p.id()), mod, Component.literal("Shadow: " + p.name()),
					p.onByDefault() ? PackActivationType.DEFAULT_ENABLED : PackActivationType.NORMAL);
		}
	}

	private static boolean isOn(Pack p) {
		return Minecraft.getInstance().getResourcePackRepository().getSelectedIds().contains(packId(p));
	}

	private static void set(Pack p, boolean on) {
		Minecraft mc = Minecraft.getInstance();
		PackRepository repo = mc.getResourcePackRepository();
		boolean changed = on ? repo.addPack(packId(p)) : repo.removePack(packId(p));
		if (changed) mc.options.updateResourcePacks(repo); // saves options.txt and reloads resources
	}

	public static void addTo(List<Module> out) {
		for (Pack p : PACKS) {
			Module m = new Module("pack:" + p.id(), p.name(), p.icon(), p.tabs(), () -> isOn(p), v -> set(p, v), null, "Reloads textures");
			out.add(p.item() != null ? m.item(p.item()) : m);
		}
	}
}
