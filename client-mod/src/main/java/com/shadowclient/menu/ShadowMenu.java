package com.shadowclient.menu;

import com.mojang.blaze3d.platform.InputConstants;
import com.shadowclient.menu.feature.Features;
import com.shadowclient.menu.feature.Hypixel;
import com.shadowclient.menu.hud.HudManager;
import com.shadowclient.menu.hud.HudStats;
import com.shadowclient.menu.module.AutoText;
import com.shadowclient.menu.module.Combat;
import com.shadowclient.menu.module.MenuConfig;
import com.shadowclient.menu.module.Module;
import com.shadowclient.menu.module.OverlayPacks;
import com.shadowclient.menu.module.PvpTracker;
import com.shadowclient.menu.module.QuickKeys;
import com.shadowclient.menu.module.ShadowHud;
import com.shadowclient.menu.module.Waypoints;
import com.shadowclient.menu.ui.ModMenuScreen;
import com.shadowclient.menu.ui.ShadowTitleScreen;
import java.util.EnumMap;
import java.util.List;
import java.util.Map;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientLifecycleEvents;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.fabricmc.fabric.api.client.message.v1.ClientReceiveMessageEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.fabricmc.fabric.api.client.rendering.v1.hud.VanillaHudElements;
import net.fabricmc.fabric.api.event.client.player.ClientPlayerBlockBreakEvents;
import net.fabricmc.fabric.api.event.player.AttackEntityCallback;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.resources.Identifier;
import net.minecraft.world.InteractionResult;
import org.lwjgl.glfw.GLFW;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Shadow Client's in-game client: the mod menu (Right Shift), its own HUD and HUD editor, the
 * modules it does natively (zoom, freelook, motion blur, …) and the Shadow Client title screen.
 */
public final class ShadowMenu implements ClientModInitializer {
	public static final Logger LOG = LoggerFactory.getLogger("ShadowMenu");
	private static final KeyMapping.Category CATEGORY = KeyMapping.Category.register(Identifier.fromNamespaceAndPath("shadowmenu", "main"));
	private static KeyMapping open;
	private static KeyMapping addWaypoint;
	private static boolean checkedConflicts;
	private static final boolean SELF_TEST = SelfTest.enabled();

	@Override
	public void onInitializeClient() {
		com.shadowclient.menu.feature.Extras.ScreenshotCopy.prepare();
		FabricLoader.getInstance().getModContainer("shadowmenu").ifPresent(OverlayPacks::register);
		HudElementRegistry.attachElementAfter(VanillaHudElements.MISC_OVERLAYS, Identifier.fromNamespaceAndPath("shadowmenu", "hud"), ShadowHud::render);
		HudManager.init();
		ClientPlayConnectionEvents.JOIN.register((handler, sender, mc) -> {
			PvpTracker.reset();
			HudStats.reset();
		});
		addWaypoint = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.shadowmenu.waypoint", GLFW.GLFW_KEY_B, CATEGORY));
		AutoText.register(CATEGORY);
		QuickKeys.register(CATEGORY);
		Features.register(CATEGORY);
		ClientReceiveMessageEvents.GAME.register((message, overlay) -> {
			if (overlay) return;
			Combat.onGameMessage(message);
			Hypixel.message(message);
		});
		ClientReceiveMessageEvents.CHAT.register((message, signed, sender, params, time) -> Hypixel.message(message));
		// runs before the message reaches the chat box, so Chat Heads knows who sent it
		ClientReceiveMessageEvents.ALLOW_CHAT.register((message, signed, sender, params, time) -> {
			com.shadowclient.menu.feature.Extras.ChatHeads.sender(sender);
			return true;
		});
		AttackEntityCallback.EVENT.register((player, level, hand, entity, hit) -> {
			if (level.isClientSide() && player == Minecraft.getInstance().player) {
				Combat.onAttack(entity);
				HudStats.attacked(entity);
				Features.attacked(entity);
			}
			return InteractionResult.PASS;
		});
		ClientPlayerBlockBreakEvents.AFTER.register((level, player, pos, state) -> Combat.onBlockBroken());
		open = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.shadowmenu.open", GLFW.GLFW_KEY_RIGHT_SHIFT, CATEGORY));
		ClientLifecycleEvents.CLIENT_STOPPING.register(mc -> MenuConfig.flush(true));
		// The Shadow Client title screen is built in: a Minecraft title screen that got onto the screen
		// another way (a mod setting it directly) is swapped before the game ticks it, which Fabric's
		// screen API would refuse for a screen that was never set up.
		ClientTickEvents.START_CLIENT_TICK.register(mc -> {
			if (mc.screen instanceof TitleScreen) mc.setScreen(new ShadowTitleScreen());
		});
		ClientTickEvents.END_CLIENT_TICK.register(mc -> {
			if (!checkedConflicts) {
				checkedConflicts = true;
				freeMenuKey(mc);
				selfCheck();
			}
			PvpTracker.tick(mc);
			Combat.tick(mc);
			HudStats.tick(mc);
			AutoText.tick(mc);
			QuickKeys.tick(mc);
			Features.tick(mc);
			MenuConfig.flush(false);
			if (SELF_TEST) SelfTest.tick(mc);
			while (addWaypoint.consumeClick()) {
				if (mc.screen == null) Waypoints.addHere();
			}
			while (open.consumeClick()) {
				if (mc.screen == null) mc.setScreen(new ModMenuScreen());
			}
		});
	}

	/** Logs what the menu will show, so a missing module is visible in latest.log without opening it. */
	private static void selfCheck() {
		List<Module> modules;
		try {
			modules = ModMenuScreen.allModules();
		} catch (Throwable t) {
			LOG.warn("Mod menu self-check failed", t);
			return;
		}
		Map<Module.Tab, Integer> perTab = new EnumMap<>(Module.Tab.class);
		for (Module m : modules) for (Module.Tab t : m.tabs) perTab.merge(t, 1, Integer::sum);
		LOG.info("Mod menu ready: {} modules ({} new), {} HUD modules, {} native features {}", modules.size(), modules.stream().filter(Module::isNew).count(),
				HudManager.all().size(), Features.all().size(), perTab);
	}

	public static KeyMapping openKey() {
		return open;
	}

	public static String version() {
		return FabricLoader.getInstance().getModContainer("shadowmenu").map(c -> c.getMetadata().getVersion().getFriendlyString()).orElse("?");
	}

	/**
	 * AxolotlClient (if it's still installed) opens its HUD editor on Right Shift too. The menu takes
	 * that key, so its binding is cleared if it collides with ours.
	 */
	private static void freeMenuKey(Minecraft mc) {
		boolean changed = false;
		for (KeyMapping k : mc.options.keyMappings) {
			if (k != open && k.getName().equals("key.openHud") && k.same(open)) {
				k.setKey(InputConstants.UNKNOWN);
				changed = true;
			}
		}
		if (changed) {
			KeyMapping.resetMapping();
			mc.options.save();
			LOG.info("Right Shift now opens the Shadow mod menu; AxolotlClient's HUD editor key was cleared.");
		}
	}
}
