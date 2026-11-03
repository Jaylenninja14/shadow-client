package com.shadowclient.menu.module;

import com.google.gson.JsonObject;
import com.shadowclient.menu.module.Module.Tab;
import java.util.EnumSet;
import java.util.List;
import java.util.Set;
import java.util.function.Function;
import java.util.function.IntFunction;
import net.minecraft.client.AttackIndicatorStatus;
import net.minecraft.client.CloudStatus;
import net.minecraft.client.GraphicsPreset;
import net.minecraft.client.InactivityFpsLimit;
import net.minecraft.client.Minecraft;
import net.minecraft.client.OptionInstance;
import net.minecraft.client.Options;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.components.debug.DebugScreenEntries;
import net.minecraft.client.gui.components.debug.DebugScreenEntryStatus;
import net.minecraft.client.gui.screens.options.AccessibilityOptionsScreen;
import net.minecraft.client.gui.screens.options.ChatOptionsScreen;
import net.minecraft.client.gui.screens.options.MouseSettingsScreen;
import net.minecraft.client.gui.screens.options.SkinCustomizationScreen;
import net.minecraft.client.gui.screens.options.SoundOptionsScreen;
import net.minecraft.client.gui.screens.options.VideoSettingsScreen;
import net.minecraft.client.gui.screens.options.controls.KeyBindsScreen;
import net.minecraft.network.chat.Component;
import net.minecraft.server.level.ParticleStatus;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.HumanoidArm;
import net.minecraft.world.entity.player.ChatVisiblity;
import net.minecraft.world.entity.player.PlayerModelPart;
import net.minecraft.world.item.Items;
import org.jetbrains.annotations.Nullable;

/**
 * Shadow Client's own modules built on Minecraft's options: they work with or without other mods.
 * Cards with a value step through it on click (left: next, right: previous).
 */
public final class GameModules {
	private GameModules() {
	}

	private static Minecraft mc() {
		return Minecraft.getInstance();
	}

	private static Options o() {
		return mc().options;
	}

	public static void addTo(List<Module> out) {
		Options o = o();
		Function<Screen, Screen> keys = p -> new KeyBindsScreen(p, o);
		Function<Screen, Screen> video = p -> new VideoSettingsScreen(p, mc(), o);
		Function<Screen, Screen> access = p -> new AccessibilityOptionsScreen(p, o);

		// ---- game & PvP
		out.add(toggle("toggle-sprint", "Toggle Sprint", "sprint", EnumSet.of(Tab.PVP, Tab.GAME), o.toggleSprint(), keys));
		out.add(toggle("toggle-sneak", "Toggle Sneak", "sprint", EnumSet.of(Tab.PVP, Tab.GAME), o.toggleCrouch(), keys));
		out.add(toggle("auto-jump", "Auto Jump", "speed", EnumSet.of(Tab.GAME), o.autoJump(), null));
		out.add(toggle("view-bobbing", "View Bobbing", "viewbob", EnumSet.of(Tab.VISUAL, Tab.GAME), o.bobView(), null));
		out.add(toggle("raw-input", "Raw Mouse Input", "cps", EnumSet.of(Tab.PVP, Tab.GAME), o.rawMouseInput(), null));
		out.add(enumCycle("attack-indicator", "Attack Indicator", "attack", EnumSet.of(Tab.PVP), o.attackIndicator(),
				List.of(AttackIndicatorStatus.values()), v -> v.caption().getString(), null));
		out.add(intCycle("fov", "Field of View", "fov", EnumSet.of(Tab.PVP, Tab.VISUAL), o.fov(), new int[] {30, 50, 60, 70, 80, 90, 100, 110},
				v -> v == 70 ? "Normal" : v == 110 ? "Quake Pro" : String.valueOf(v), null));
		out.add(doubleCycle("damage-tilt", "Damage Tilt", "tilt", EnumSet.of(Tab.PVP, Tab.VISUAL), o.damageTiltStrength(), new double[] {0, 0.25, 0.5, 1}, access));
		out.add(toggle("fullscreen", "Fullscreen", "fullscreen", EnumSet.of(Tab.GAME), o.fullscreen(), null));
		out.add(toggle("subtitles", "Subtitles", "subtitle", EnumSet.of(Tab.GAME), o.showSubtitles(), null));
		out.add(toggle("autosave-indicator", "Autosave Indicator", "save", EnumSet.of(Tab.GAME), o.showAutosaveIndicator(), null));
		out.add(toggle("hide-splash", "Hide Splash Texts", "splash", EnumSet.of(Tab.GAME, Tab.VISUAL), o.hideSplashTexts(), null));
		out.add(intCycle("gui-scale", "GUI Scale", "gui", EnumSet.of(Tab.GAME, Tab.VISUAL), o.guiScale(), new int[] {0, 1, 2, 3, 4, 5, 6},
				v -> v == 0 ? "Auto" : String.valueOf(v), null, () -> mc().resizeDisplay()));

		// ---- visual
		out.add(toggle("hide-lightning", "Hide Lightning Flash", "storm", EnumSet.of(Tab.VISUAL), o.hideLightningFlash(), access));
		out.add(doubleCycle("distortion", "Distortion Effects", "blur", EnumSet.of(Tab.VISUAL), o.screenEffectScale(), new double[] {0, 0.25, 0.5, 0.75, 1}, access));
		out.add(doubleCycle("fov-effects", "FOV Effects", "fov", EnumSet.of(Tab.VISUAL, Tab.PVP), o.fovEffectScale(), new double[] {0, 0.25, 0.5, 0.75, 1}, access));
		out.add(doubleCycle("darkness-pulse", "Darkness Pulsing", "darkness", EnumSet.of(Tab.VISUAL), o.darknessEffectScale(), new double[] {0, 0.25, 0.5, 1}, access));
		out.add(doubleCycle("glint", "Enchant Glint", "glint", EnumSet.of(Tab.VISUAL), o.glintStrength(), new double[] {0, 0.25, 0.5, 0.75, 1}, access));
		out.add(doubleCycle("brightness", "Brightness", "brightness", EnumSet.of(Tab.VISUAL), o.gamma(), new double[] {0, 0.25, 0.5, 0.75, 1}, video));
		out.add(intCycle("menu-blur", "Menu Blur", "blur", EnumSet.of(Tab.VISUAL, Tab.PERFORMANCE), o.menuBackgroundBlurriness(), new int[] {0, 2, 5, 10},
				v -> v == 0 ? "Off" : String.valueOf(v), null));

		// ---- performance
		out.add(fpsBoost());
		out.add(enumCycle("graphics", "Graphics", "graphics", EnumSet.of(Tab.PERFORMANCE), o.graphicsPreset(),
				List.of(GraphicsPreset.FAST, GraphicsPreset.FANCY, GraphicsPreset.FABULOUS), v -> Component.translatable(v.getKey()).getString(), video));
		out.add(intCycle("render-distance", "Render Distance", "distance", EnumSet.of(Tab.PERFORMANCE), o.renderDistance(), new int[] {2, 4, 6, 8, 10, 12, 16, 20, 24, 32},
				v -> v + " chunks", video));
		out.add(intCycle("simulation-distance", "Simulation Distance", "chunk", EnumSet.of(Tab.PERFORMANCE), o.simulationDistance(), new int[] {5, 6, 8, 10, 12, 16, 24, 32},
				v -> v + " chunks", video));
		out.add(intCycle("max-fps", "Max Framerate", "fps", EnumSet.of(Tab.PERFORMANCE), o.framerateLimit(), new int[] {30, 60, 100, 120, 140, 160, 240, 260},
				v -> v >= 260 ? "Unlimited" : v + " fps", video));
		out.add(toggle("vsync", "VSync", "vsync", EnumSet.of(Tab.PERFORMANCE, Tab.GAME), o.enableVsync(), video));
		out.add(enumCycle("unfocused-fps", "Inactivity FPS Limit", "darkness", EnumSet.of(Tab.PERFORMANCE), o.inactivityFpsLimit(),
				List.of(InactivityFpsLimit.values()), v -> v.caption().getString(), video));
		out.add(enumCycle("clouds", "Clouds", "cloud", EnumSet.of(Tab.PERFORMANCE, Tab.VISUAL), o.cloudStatus(),
				List.of(CloudStatus.values()), v -> v.caption().getString(), video));
		out.add(enumCycle("particles", "Particles", "particles", EnumSet.of(Tab.PERFORMANCE, Tab.VISUAL), o.particles(),
				List.of(ParticleStatus.values()), v -> v.caption().getString(), video));
		out.add(toggle("entity-shadows", "Entity Shadows", "shadow", EnumSet.of(Tab.PERFORMANCE, Tab.VISUAL), o.entityShadows(), video));
		out.add(toggle("smooth-lighting", "Smooth Lighting", "light", EnumSet.of(Tab.PERFORMANCE, Tab.VISUAL), o.ambientOcclusion(), video));
		out.add(toggle("cutout-leaves", "Cutout Leaves", "leaves", EnumSet.of(Tab.PERFORMANCE, Tab.VISUAL), o.cutoutLeaves(), video));
		out.add(toggle("improved-transparency", "Improved Transparency", "glass", EnumSet.of(Tab.PERFORMANCE, Tab.VISUAL), o.improvedTransparency(), video));
		out.add(intCycle("biome-blend", "Biome Blend", "blend", EnumSet.of(Tab.PERFORMANCE), o.biomeBlendRadius(), new int[] {0, 1, 2, 3, 5, 7},
				v -> v == 0 ? "Off" : (v * 2 + 1) + "x" + (v * 2 + 1), video));
		out.add(doubleCycle("entity-distance", "Entity Distance", "entity", EnumSet.of(Tab.PERFORMANCE), o.entityDistanceScaling(),
				new double[] {0.5, 0.75, 1, 1.25, 1.5, 2, 3, 5}, video));
	}

	/** Lunar-style extras on Minecraft's own options: sound mixer, mouse, chat, skin layers, debug views. */
	public static void addMore(List<Module> out) {
		Options o = o();
		Function<Screen, Screen> sounds = p -> new SoundOptionsScreen(p, o);
		Function<Screen, Screen> chat = p -> new ChatOptionsScreen(p, o);
		Function<Screen, Screen> mouse = p -> new MouseSettingsScreen(p, o);
		Function<Screen, Screen> skin = p -> new SkinCustomizationScreen(p, o);
		Function<Screen, Screen> access = p -> new AccessibilityOptionsScreen(p, o);
		double[] volume = {0, 0.25, 0.5, 0.75, 1};

		// sound mixer: turn music, weather and ambience down to hear footsteps
		Object[][] mixer = {
				{SoundSource.MASTER, "Master Volume", EnumSet.of(Tab.GAME)},
				{SoundSource.MUSIC, "Music Volume", EnumSet.of(Tab.GAME, Tab.PVP)},
				{SoundSource.RECORDS, "Jukebox Volume", EnumSet.of(Tab.GAME)},
				{SoundSource.WEATHER, "Weather Volume", EnumSet.of(Tab.GAME, Tab.PVP)},
				{SoundSource.BLOCKS, "Block Sounds", EnumSet.of(Tab.GAME, Tab.PVP)},
				{SoundSource.HOSTILE, "Hostile Mob Sounds", EnumSet.of(Tab.GAME)},
				{SoundSource.NEUTRAL, "Friendly Mob Sounds", EnumSet.of(Tab.GAME)},
				{SoundSource.PLAYERS, "Player Sounds", EnumSet.of(Tab.GAME, Tab.PVP)},
				{SoundSource.AMBIENT, "Ambient Sounds", EnumSet.of(Tab.GAME, Tab.PVP)},
				{SoundSource.VOICE, "Voice Volume", EnumSet.of(Tab.GAME)},
				{SoundSource.UI, "Menu Sounds", EnumSet.of(Tab.GAME)},
		};
		for (Object[] row : mixer) {
			SoundSource src = (SoundSource) row[0];
			@SuppressWarnings("unchecked") Set<Tab> tabs = (Set<Tab>) row[2];
			out.add(doubleCycle("sound-" + src.getName(), (String) row[1], src == SoundSource.MUSIC || src == SoundSource.RECORDS ? "music" : "sound", tabs,
					o.getSoundSourceOptionInstance(src), volume, sounds));
		}
		out.add(toggle("directional-audio", "Directional Audio", "sound", EnumSet.of(Tab.PVP, Tab.GAME), o.directionalAudio(), sounds));

		// mouse (vanilla shows sensitivity as value × 200%)
		out.add(doubleCycle("sensitivity", "Mouse Sensitivity", "mouse", EnumSet.of(Tab.PVP, Tab.GAME), o.sensitivity(), new double[] {0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0},
				v -> Math.round(v * 200) + "%", mouse));
		out.add(doubleCycle("scroll-sensitivity", "Scroll Sensitivity", "mouse", EnumSet.of(Tab.GAME), o.mouseWheelSensitivity(), new double[] {0.5, 1, 2, 3, 5},
				v -> String.format(java.util.Locale.ROOT, "%.1fx", v), mouse));

		// chat
		out.add(enumCycle("chat-visibility", "Chat Visibility", "chat", EnumSet.of(Tab.GAME, Tab.PVP), o.chatVisibility(), List.of(ChatVisiblity.values()),
				v -> v.caption().getString(), chat));
		out.add(doubleCycle("chat-opacity", "Chat Opacity", "chat", EnumSet.of(Tab.GAME), o.chatOpacity(), new double[] {0.25, 0.5, 0.75, 1}, chat));
		out.add(doubleCycle("chat-scale", "Chat Size", "chat", EnumSet.of(Tab.GAME), o.chatScale(), new double[] {0.5, 0.75, 1}, chat));
		out.add(doubleCycle("chat-background", "Chat Background", "chat", EnumSet.of(Tab.GAME), o.textBackgroundOpacity(), new double[] {0, 0.25, 0.5, 0.75}, chat));

		// skin layers and main hand
		Object[][] parts = {
				{PlayerModelPart.CAPE, "Show Cape"}, {PlayerModelPart.HAT, "Show Hat Layer"}, {PlayerModelPart.JACKET, "Show Jacket"},
				{PlayerModelPart.LEFT_SLEEVE, "Show Left Sleeve"}, {PlayerModelPart.RIGHT_SLEEVE, "Show Right Sleeve"},
				{PlayerModelPart.LEFT_PANTS_LEG, "Show Left Pants Leg"}, {PlayerModelPart.RIGHT_PANTS_LEG, "Show Right Pants Leg"},
		};
		for (Object[] row : parts) {
			PlayerModelPart part = (PlayerModelPart) row[0];
			out.add(new Module("skin-" + part.getId(), (String) row[1], "skin", EnumSet.of(Tab.VISUAL), () -> o.isModelPartEnabled(part), v -> {
				o.setModelPart(part, v);
				o.save();
			}, skin, null));
		}
		out.add(enumCycle("main-hand", "Main Hand", "hand", EnumSet.of(Tab.PVP, Tab.GAME), o.mainHand(), List.of(HumanoidArm.values()), v -> v.caption().getString(), skin));
		out.add(toggle("contrast-outline", "High Contrast Block Outline", "outline", EnumSet.of(Tab.PVP, Tab.VISUAL), o.highContrastBlockOutline(), access));

		// debug views (F3 + B / F3 + G / F3 + H / F3 + P)
		out.add(debugEntry("hitboxes", "Hitboxes", "hitbox", EnumSet.of(Tab.PVP, Tab.VISUAL), DebugScreenEntries.ENTITY_HITBOXES));
		out.add(debugEntry("chunk-borders", "Chunk Borders", "chunkborder", EnumSet.of(Tab.VISUAL, Tab.GAME), DebugScreenEntries.CHUNK_BORDERS));
		out.add(new Module("advanced-tooltips", "Advanced Tooltips", "tooltip", EnumSet.of(Tab.PVP, Tab.GAME), () -> o.advancedItemTooltips, v -> {
			o.advancedItemTooltips = v;
			o.save();
		}, null, null));
		out.add(new Module("pause-on-focus", "Pause When Unfocused", "pause", EnumSet.of(Tab.GAME), () -> o.pauseOnLostFocus, v -> {
			o.pauseOnLostFocus = v;
			o.save();
		}, null, null));

		// tweaks done by the menu's own mixins
		out.add(tweak(Tweaks.CHAT_TIMESTAMPS, "Chat Timestamps", "clock", EnumSet.of(Tab.GAME), chat));
		out.add(tweak(Tweaks.LONG_CHAT, "Longer Chat History", "chat", EnumSet.of(Tab.GAME), chat));
		out.add(tweak(Tweaks.NO_TOTEM_ANIMATION, "No Totem Animation", "player", EnumSet.of(Tab.PVP, Tab.VISUAL), null).item(Items.TOTEM_OF_UNDYING));
	}

	/** More of Minecraft's own options as cards: camera, mouse, chat, music, menus, privacy. */
	public static void addEvenMore(List<Module> out) {
		Options o = o();
		Function<Screen, Screen> chat = p -> new ChatOptionsScreen(p, o);
		Function<Screen, Screen> mouse = p -> new MouseSettingsScreen(p, o);
		Function<Screen, Screen> access = p -> new AccessibilityOptionsScreen(p, o);
		Function<Screen, Screen> sounds = p -> new SoundOptionsScreen(p, o);
		Function<Screen, Screen> video = p -> new VideoSettingsScreen(p, mc(), o);
		Set<Tab> game = EnumSet.of(Tab.GAME), pvpGame = EnumSet.of(Tab.PVP, Tab.GAME), visual = EnumSet.of(Tab.VISUAL);
		double[] pct = {0, 0.25, 0.5, 0.75, 1};

		// camera and mouse
		out.add(new Module("cinematic-camera", "Cinematic Camera", "camera", EnumSet.of(Tab.VISUAL, Tab.GAME), () -> o.smoothCamera, v -> o.smoothCamera = v, null, null));
		out.add(toggle("invert-mouse-x", "Invert Mouse X", "mouse", game, o.invertMouseX(), mouse));
		out.add(toggle("invert-mouse-y", "Invert Mouse Y", "mouse", game, o.invertMouseY(), mouse));
		out.add(toggle("discrete-scroll", "Discrete Scrolling", "mouse", pvpGame, o.discreteMouseScroll(), mouse));
		out.add(toggle("cursor-changes", "Cursor Changes", "mouse", game, o.allowCursorChanges(), mouse));
		out.add(toggle("rotate-minecart", "Rotate with Minecart", "compass", game, o.rotateWithMinecart(), null));
		out.add(doubleCycle("glint-speed", "Enchant Glint Speed", "glint", visual, o.glintSpeed(), pct, access));

		// chat
		out.add(toggle("chat-colours", "Chat Colours", "chat", game, o.chatColors(), chat));
		out.add(toggle("chat-links", "Web Links in Chat", "chat", game, o.chatLinks(), chat));
		out.add(toggle("chat-link-prompt", "Prompt on Links", "chat", game, o.chatLinksPrompt(), chat));
		out.add(toggle("command-suggestions", "Command Suggestions", "chat", game, o.autoSuggestions(), chat));
		out.add(toggle("hide-matched-names", "Hide Matched Names", "chat", game, o.hideMatchedNames(), chat));
		out.add(toggle("secure-chat-only", "Only Show Secure Chat", "chat", game, o.onlyShowSecureChat(), chat));
		out.add(toggle("chat-background-only", "Text Background: Chat Only", "chat", game, o.backgroundForChatOnly(), access));
		out.add(doubleCycle("chat-line-spacing", "Chat Line Spacing", "chat", game, o.chatLineSpacing(), new double[] {0, 0.25, 0.5, 1}, chat));
		out.add(doubleCycle("chat-width", "Chat Width", "chat", game, o.chatWidth(), new double[] {0.25, 0.5, 0.75, 1}, chat));
		out.add(doubleCycle("chat-height", "Chat Height (open)", "chat", game, o.chatHeightFocused(), new double[] {0.25, 0.5, 0.75, 1}, chat));
		out.add(doubleCycle("chat-height-closed", "Chat Height (closed)", "chat", game, o.chatHeightUnfocused(), new double[] {0.1, 0.25, 0.44, 0.75, 1}, chat));
		out.add(doubleCycle("chat-delay", "Chat Delay", "time", game, o.chatDelay(), new double[] {0, 0.5, 1, 2, 3}, v -> v <= 0.001 ? "None" : one(v) + " s", chat));

		// music, menus, notifications
		out.add(enumCycle("music-frequency", "Music Frequency", "music", game, o.musicFrequency(),
				List.of(net.minecraft.client.sounds.MusicManager.MusicFrequency.values()), v -> pretty(v.name()), sounds));
		out.add(enumCycle("music-toast", "Now Playing Toast", "music", game, o.musicToast(),
				List.of(net.minecraft.client.MusicToastDisplayState.values()), v -> pretty(v.name()), sounds));
		out.add(enumCycle("narrator", "Narrator", "sound", game, o.narrator(), List.of(net.minecraft.client.NarratorStatus.values()), v -> pretty(v.name()), access));
		out.add(toggle("dark-loading-screen", "Dark Loading Screen", "darkness", visual, o.darkMojangStudiosBackground(), access));
		out.add(doubleCycle("panorama-speed", "Menu Panorama Speed", "compass", visual, o.panoramaSpeed(), new double[] {0.1, 0.25, 0.5, 1}, access));
		out.add(doubleCycle("notification-time", "Notification Time", "time", game, o.notificationDisplayTime(), new double[] {0.5, 1, 2, 4, 8},
				v -> one(v) + "x", access));
		out.add(toggle("operator-tab", "Operator Items Tab", "pack", game, o.operatorItemsTab(), null));
		out.add(enumCycle("chunk-updates", "Chunk Builder", "chunk", EnumSet.of(Tab.PERFORMANCE), o.prioritizeChunkUpdates(),
				List.of(net.minecraft.client.PrioritizeChunkUpdates.values()), v -> pretty(v.name()), video));
		out.add(toggle("realms-notifications", "Realms Notifications", "server", game, o.realmsNotifications(), null));
		out.add(toggle("server-listing", "Allow Server Listing", "server", game, o.allowServerListing(), null));
	}

	private static String pretty(String name) {
		StringBuilder out = new StringBuilder();
		for (String w : name.toLowerCase(java.util.Locale.ROOT).split("_")) {
			if (w.isEmpty()) continue;
			if (!out.isEmpty()) out.append(' ');
			out.append(Character.toUpperCase(w.charAt(0))).append(w.substring(1));
		}
		return out.toString();
	}

	private static String one(double v) {
		return String.format(java.util.Locale.ROOT, v == Math.rint(v) ? "%.0f" : "%.1f", v);
	}

	/** An F3 debug view kept on (shown without F3) or off. */
	private static Module debugEntry(String id, String name, String icon, Set<Tab> tabs, net.minecraft.resources.Identifier entry) {
		return new Module(id, name, icon, tabs, () -> mc().debugEntries.getStatus(entry) == DebugScreenEntryStatus.ALWAYS_ON, v -> {
			mc().debugEntries.setStatus(entry, v ? DebugScreenEntryStatus.ALWAYS_ON : DebugScreenEntryStatus.NEVER);
			mc().debugEntries.save();
		}, null, null);
	}

	private static Module tweak(String id, String name, String icon, Set<Tab> tabs, @Nullable Function<Screen, Screen> settings) {
		return new Module(id, name, icon, tabs, () -> Tweaks.is(id), v -> Tweaks.set(id, v), settings, null);
	}

	// ------------------------------------------------------------------ builders

	private static void save() {
		o().save();
	}

	private static Module toggle(String id, String name, String icon, Set<Tab> tabs, OptionInstance<Boolean> opt, @Nullable Function<Screen, Screen> settings) {
		return new Module(id, name, icon, tabs, opt::get, v -> {
			opt.set(v);
			save();
		}, settings, null);
	}

	private static <T> Module enumCycle(String id, String name, String icon, Set<Tab> tabs, OptionInstance<T> opt, List<T> order, Function<T, String> label,
			@Nullable Function<Screen, Screen> settings) {
		return Module.cycle(id, name, icon, tabs, () -> label.apply(opt.get()), dir -> {
			int i = order.indexOf(opt.get());
			opt.set(order.get(i < 0 ? 0 : Math.floorMod(i + dir, order.size())));
			save();
		}, settings);
	}

	private static Module intCycle(String id, String name, String icon, Set<Tab> tabs, OptionInstance<Integer> opt, int[] steps, IntFunction<String> label,
			@Nullable Function<Screen, Screen> settings) {
		return intCycle(id, name, icon, tabs, opt, steps, label, settings, null);
	}

	private static Module intCycle(String id, String name, String icon, Set<Tab> tabs, OptionInstance<Integer> opt, int[] steps, IntFunction<String> label,
			@Nullable Function<Screen, Screen> settings, @Nullable Runnable after) {
		return Module.cycle(id, name, icon, tabs, () -> label.apply(opt.get()), dir -> {
			int cur = opt.get();
			// the next step above (or below) the current value, wrapping around; values outside
			// what this PC allows are clamped or refused by Minecraft itself, then skipped
			for (int tries = 0; tries < steps.length; tries++) {
				int next = nextStep(steps, cur, dir);
				opt.set(next);
				if (opt.get() != cur) break;
				cur = next;
			}
			save();
			if (after != null) after.run();
		}, settings);
	}

	private static int nextStep(int[] steps, int cur, int dir) {
		if (dir > 0) {
			for (int s : steps) if (s > cur) return s;
			return steps[0];
		}
		for (int i = steps.length - 1; i >= 0; i--) if (steps[i] < cur) return steps[i];
		return steps[steps.length - 1];
	}

	private static Module doubleCycle(String id, String name, String icon, Set<Tab> tabs, OptionInstance<Double> opt, double[] steps,
			@Nullable Function<Screen, Screen> settings) {
		return doubleCycle(id, name, icon, tabs, opt, steps, v -> v <= 0.001 ? "Off" : Math.round(v * 100) + "%", settings);
	}

	private static Module doubleCycle(String id, String name, String icon, Set<Tab> tabs, OptionInstance<Double> opt, double[] steps,
			java.util.function.DoubleFunction<String> label, @Nullable Function<Screen, Screen> settings) {
		return Module.cycle(id, name, icon, tabs, () -> label.apply(opt.get()), dir -> {
			double cur = opt.get(), next = dir > 0 ? steps[0] : steps[steps.length - 1];
			if (dir > 0) {
				for (double s : steps) if (s > cur + 0.001) { next = s; break; }
			} else {
				for (int i = steps.length - 1; i >= 0; i--) if (steps[i] < cur - 0.001) { next = steps[i]; break; }
			}
			opt.set(next);
			save();
		}, settings);
	}

	// ------------------------------------------------------------------ FPS Boost

	/**
	 * One switch for the settings that cost the most frames. What it changes is remembered in
	 * config/shadowmenu.json, and switching it off puts exactly those values back.
	 */
	private static Module fpsBoost() {
		return new Module("fps-boost", "FPS Boost", "boost", EnumSet.of(Tab.PERFORMANCE), () -> MenuConfig.object("fpsBoost") != null, on -> {
			Options o = o();
			if (on) {
				JsonObject before = new JsonObject();
				before.addProperty("graphics", o.graphicsPreset().get().getSerializedName());
				before.addProperty("particles", o.particles().get().name());
				before.addProperty("clouds", o.cloudStatus().get().getSerializedName());
				before.addProperty("entityShadows", o.entityShadows().get());
				before.addProperty("improvedTransparency", o.improvedTransparency().get());
				before.addProperty("biomeBlend", o.biomeBlendRadius().get());
				before.addProperty("entityDistance", o.entityDistanceScaling().get());
				before.addProperty("menuBlur", o.menuBackgroundBlurriness().get());
				MenuConfig.putObject("fpsBoost", before);
				o.graphicsPreset().set(GraphicsPreset.FAST);
				o.particles().set(ParticleStatus.MINIMAL);
				o.cloudStatus().set(CloudStatus.OFF);
				o.entityShadows().set(false);
				o.improvedTransparency().set(false);
				o.biomeBlendRadius().set(0);
				o.entityDistanceScaling().set(0.75);
				o.menuBackgroundBlurriness().set(0);
			} else {
				JsonObject b = MenuConfig.object("fpsBoost");
				if (b != null) {
					try {
						for (GraphicsPreset g : GraphicsPreset.values()) if (g.getSerializedName().equals(b.get("graphics").getAsString())) o.graphicsPreset().set(g);
						o.particles().set(ParticleStatus.valueOf(b.get("particles").getAsString()));
						for (CloudStatus c : CloudStatus.values()) if (c.getSerializedName().equals(b.get("clouds").getAsString())) o.cloudStatus().set(c);
						o.entityShadows().set(b.get("entityShadows").getAsBoolean());
						o.improvedTransparency().set(b.get("improvedTransparency").getAsBoolean());
						o.biomeBlendRadius().set(b.get("biomeBlend").getAsInt());
						o.entityDistanceScaling().set(b.get("entityDistance").getAsDouble());
						o.menuBackgroundBlurriness().set(b.get("menuBlur").getAsInt());
					} catch (RuntimeException ignored) {
						// a hand-edited file: keep whatever could be restored
					}
				}
				MenuConfig.putObject("fpsBoost", null);
			}
			o.save();
		}, p -> new VideoSettingsScreen(p, mc(), o()), null);
	}
}
