package com.shadowclient.menu;

import com.shadowclient.menu.hud.HudElement;
import com.shadowclient.menu.hud.HudManager;
import com.shadowclient.menu.module.Module;
import com.shadowclient.menu.ui.HudEditorScreen;
import com.shadowclient.menu.ui.ModMenuScreen;
import com.shadowclient.menu.ui.ShadowTitleScreen;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.world.Difficulty;
import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.LevelSettings;
import net.minecraft.world.level.WorldDataConfiguration;
import net.minecraft.world.level.gamerules.GameRules;
import net.minecraft.world.level.levelgen.WorldOptions;
import net.minecraft.world.level.levelgen.presets.WorldPresets;

/**
 * A screenshot tour for testing builds without touching the game: only runs when the environment
 * variable SHADOWMENU_SELFTEST=1 is set (never for players). It shows the title screen, the menu,
 * two settings pages, then makes a flat creative test world, shows the HUD and the HUD editor,
 * saves a screenshot of each to screenshots/shadow-*.png, logs "self-test done" and quits.
 */
final class SelfTest {
	private static final String WORLD = "Shadow Self Test";
	private static int step;
	private static boolean resized;
	private static long at;
	private static Screen title;

	private SelfTest() {
	}

	static boolean enabled() {
		return "1".equals(System.getenv("SHADOWMENU_SELFTEST"));
	}

	private static boolean waited(long ms) {
		return System.currentTimeMillis() - at >= ms;
	}

	private static void next() {
		step++;
		at = System.currentTimeMillis();
	}

	private static void shot(Minecraft mc, String name) {
		Screenshot.grab(mc.gameDirectory, "shadow-" + name + ".png", mc.getMainRenderTarget(), 1,
				msg -> ShadowMenu.LOG.info("Self-test screenshot {}: {}", name, msg.getString()));
	}

	static void tick(Minecraft mc) {
		if (at == 0) at = System.currentTimeMillis();
		switch (step) {
			case 0 -> {
				// a bigger window than the dev default, for screenshots worth showing
				if (!resized) {
					resized = true;
					org.lwjgl.glfw.GLFW.glfwSetWindowSize(mc.getWindow().handle(), 1600, 900);
				}
				// a fresh game folder starts on the accessibility welcome screen: go past it
				if (mc.getOverlay() == null && !(mc.screen instanceof ShadowTitleScreen) && mc.level == null && waited(3000)) {
					mc.options.onboardAccessibility = false;
					mc.options.save();
					mc.setScreen(new ShadowTitleScreen());
					at = System.currentTimeMillis();
				}
				if (mc.getOverlay() == null && mc.screen instanceof ShadowTitleScreen s && waited(4500)) {
					title = s;
					shot(mc, "title");
					next();
				}
			}
			case 1 -> {
				// what joining or leaving a server does: no world, no screen; Minecraft makes a title
				// screen there (this crashed 2.0.0: Fabric found a title screen that was never set up)
				if (waited(1000)) {
					mc.setScreen(null);
					ShadowMenu.LOG.info("Self-test: setScreen(null) without a world shows {}", mc.screen == null ? "nothing" : mc.screen.getClass().getSimpleName());
					step = 12;
					at = System.currentTimeMillis();
				}
			}
			case 12 -> {
				// drawn for a second (a screen that wasn't set up would crash here), then on to the menu
				if (waited(1000) && mc.screen instanceof ShadowTitleScreen s) {
					title = s;
					mc.setScreen(new ModMenuScreen(title));
					step = 2;
					at = System.currentTimeMillis();
				}
			}
			case 2 -> {
				if (waited(1500)) {
					shot(mc, "menu");
					HudElement keys = HudManager.byId("keystrokes");
					if (keys != null) mc.setScreen(ModMenuScreen.page(title, keys.card()));
					next();
				}
			}
			case 3 -> {
				if (waited(1200)) {
					shot(mc, "settings");
					mc.setScreen(ModMenuScreen.clientSettings(title));
					next();
				}
			}
			case 4 -> {
				if (waited(1200)) {
					shot(mc, "client-settings");
					mc.setScreen(ModMenuScreen.mods(title));
					step = 41; // the mod list, then on to the world (5)
					at = System.currentTimeMillis();
				}
			}
			case 41 -> {
				if (waited(1500)) {
					shot(mc, "mods");
					openWorld(mc);
					step = 5;
					at = System.currentTimeMillis();
				}
			}
			case 5 -> {
				if (mc.level != null && mc.player != null && mc.screen == null && waited(4000)) {
					setUpHud(mc);
					next();
				}
			}
			case 6 -> {
				// the commands' replies and the tutorial pop-ups would cover the HUD: clear them, then chat once
				if (waited(3000)) {
					mc.gui.getChat().clearMessages(false);
					mc.getToastManager().clear();
					mc.player.connection.sendChat("gg! Chat heads show who's talking.");
					next();
				}
			}
			case 7 -> {
				if (waited(2000)) {
					mc.getToastManager().clear();
					shot(mc, "hud");
					mc.setScreen(new HudEditorScreen(null));
					next();
				}
			}
			case 8 -> {
				if (waited(1500)) {
					shot(mc, "hud-editor");
					mc.setScreen(ModMenuScreen.hudTab(null));
					next();
				}
			}
			case 9 -> {
				if (waited(1500)) {
					shot(mc, "menu-ingame");
					mc.setScreen(ModMenuScreen.page(null, com.shadowclient.menu.feature.Crosshair.I.card()));
					next();
				}
			}
			case 10 -> {
				if (waited(1500)) {
					shot(mc, "crosshair-settings");
					next();
				}
			}
			case 11 -> {
				if (waited(2000)) {
					List<Module> all = ModMenuScreen.allModules();
					ShadowMenu.LOG.info("Shadow Menu self-test done: {} modules, {} HUD modules", all.size(), HudManager.all().size());
					mc.stop();
					next();
				}
			}
			default -> {
			}
		}
	}

	private static void openWorld(Minecraft mc) {
		try {
			if (mc.getLevelSource().levelExists(WORLD)) {
				mc.createWorldOpenFlows().openWorld(WORLD, () -> mc.setScreen(title));
				return;
			}
		} catch (RuntimeException e) {
			ShadowMenu.LOG.warn("Self-test: couldn't check for the test world", e);
		}
		LevelSettings settings = new LevelSettings(WORLD, GameType.CREATIVE, false, Difficulty.PEACEFUL, true,
				new GameRules(FeatureFlags.DEFAULT_FLAGS), WorldDataConfiguration.DEFAULT);
		mc.createWorldOpenFlows().createFreshLevel(WORLD, settings, new WorldOptions(20261101L, false, false), WorldPresets::createFlatWorldDimensions, title);
	}

	/** Switch on a typical PvP HUD and give the player armor and effects so it has something to show. */
	private static void setUpHud(Minecraft mc) {
		for (String id : new String[] {"fps", "cps", "ping", "keystrokes", "armor-status", "potion-effects", "coordinates", "clock", "direction", "speed"}) {
			HudElement e = HudManager.byId(id);
			if (e != null) e.enabled.set(true);
		}
		var c = mc.player.connection;
		mc.options.tutorialStep = net.minecraft.client.tutorial.TutorialSteps.NONE;
		c.sendCommand("time set noon");
		c.sendCommand("difficulty easy");
		c.sendCommand("gamemode survival");
		c.sendCommand("effect give @s hunger 3 250");
		c.sendCommand("item replace entity @s weapon.offhand with cooked_beef 16");
		c.sendCommand("item replace entity @s armor.head with diamond_helmet");
		c.sendCommand("item replace entity @s armor.chest with netherite_chestplate");
		c.sendCommand("item replace entity @s armor.legs with diamond_leggings");
		c.sendCommand("item replace entity @s armor.feet with iron_boots");
		c.sendCommand("item replace entity @s weapon.mainhand with diamond_sword");
		c.sendCommand("effect give @s speed 300 1");
		c.sendCommand("effect give @s strength 120 0");
		c.sendCommand("effect give @s fire_resistance 600 0");
	}
}
