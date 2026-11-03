package com.shadowclient.menu.module;

import com.mojang.blaze3d.platform.InputConstants;
import com.shadowclient.menu.module.Module.Tab;
import java.util.EnumSet;
import java.util.List;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.options.controls.KeyBindsScreen;
import net.minecraft.core.BlockPos;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.player.ChatVisiblity;
import net.minecraft.world.item.Items;

/**
 * Small hotkeys (unbound until you set them in Controls → Shadow Menu): copy your coordinates,
 * clear chat, and hide/show chat. Plus the camera and off-hand switches handled by mixins.
 */
public final class QuickKeys {
	private static KeyMapping copyCoords, clearChat, toggleChat;

	private QuickKeys() {
	}

	public static void register(KeyMapping.Category category) {
		copyCoords = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.shadowmenu.copycoords", InputConstants.UNKNOWN.getValue(), category));
		clearChat = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.shadowmenu.clearchat", InputConstants.UNKNOWN.getValue(), category));
		toggleChat = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.shadowmenu.togglechat", InputConstants.UNKNOWN.getValue(), category));
	}

	public static void tick(Minecraft mc) {
		while (copyCoords.consumeClick()) {
			if (mc.player == null) continue;
			BlockPos p = mc.player.blockPosition();
			String text = p.getX() + " " + p.getY() + " " + p.getZ();
			mc.keyboardHandler.setClipboard(text);
			mc.player.displayClientMessage(Component.literal("Copied " + text), true);
		}
		while (clearChat.consumeClick()) {
			mc.gui.getChat().clearMessages(false);
		}
		while (toggleChat.consumeClick()) {
			var opt = mc.options.chatVisibility();
			opt.set(opt.get() == ChatVisiblity.HIDDEN ? ChatVisiblity.FULL : ChatVisiblity.HIDDEN);
			mc.options.save();
			if (mc.player != null) mc.player.displayClientMessage(Component.literal(opt.get() == ChatVisiblity.HIDDEN ? "Chat hidden" : "Chat shown"), true);
		}
	}

	public static void addTo(List<Module> out) {
		Minecraft mc = Minecraft.getInstance();
		java.util.function.Function<net.minecraft.client.gui.screens.Screen, net.minecraft.client.gui.screens.Screen> keys = p -> new KeyBindsScreen(p, mc.options);
		out.add(new Module("copy-coords-key", "Copy Coordinates Key", "coords", EnumSet.of(Tab.GAME, Tab.PVP), () -> true, null, keys, "Set key in Controls").item(Items.COMPASS));
		out.add(new Module("clear-chat-key", "Clear Chat Key", "chat", EnumSet.of(Tab.GAME), () -> true, null, keys, "Set key in Controls"));
		out.add(new Module("toggle-chat-key", "Toggle Chat Key", "chat", EnumSet.of(Tab.GAME, Tab.PVP), () -> true, null, keys, "Set key in Controls"));
		out.add(new Module("third-person-back-only", "Third Person Back Only", "camera", EnumSet.of(Tab.PVP, Tab.GAME), () -> Tweaks.is("third-person-back-only"),
				v -> Tweaks.set("third-person-back-only", v), null, null).item(Items.ENDER_EYE));
		out.add(new Module("hide-offhand", "Hide Offhand Item", "hand", EnumSet.of(Tab.PVP, Tab.VISUAL), () -> Tweaks.is("hide-offhand"),
				v -> Tweaks.set("hide-offhand", v), null, null).item(Items.SHIELD));
	}
}
