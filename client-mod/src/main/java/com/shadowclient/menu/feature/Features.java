package com.shadowclient.menu.feature;

import com.mojang.blaze3d.platform.InputConstants;
import com.shadowclient.menu.module.Module;
import java.util.List;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.fabricmc.fabric.api.client.screen.v1.ScreenMouseEvents;
import net.fabricmc.fabric.api.client.screen.v1.Screens;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.ConfirmScreen;
import net.minecraft.client.gui.screens.PauseScreen;
import net.minecraft.client.gui.screens.inventory.AbstractContainerScreen;
import net.minecraft.network.chat.CommonComponents;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.Entity;
import org.lwjgl.glfw.GLFW;

/**
 * Every module Shadow Menu does natively (what AxolotlClient used to provide): their keys, per-tick
 * work and menu cards.
 */
public final class Features {
	private Features() {
	}

	public static List<Feature> all() {
		return List.of(Zoom.I, Freelook.I, MotionBlur.I, Crosshair.I, Visuals.Fullbright.I, Visuals.TimeChanger.I, Visuals.BlockOutline.I,
				Visuals.HitColor.I, Visuals.NoHurtCam.I, Visuals.LowFire.I, Visuals.LowShield.I, Visuals.MinimalBob.I, Visuals.NoVignette.I,
				Visuals.NoRain.I, Visuals.Nametags.I, Visuals.TntTimer.I, Visuals.BeaconBeams.I, Visuals.HitParticles.I, Visuals.ScrollTooltips.I,
				Visuals.ConfirmDisconnect.I, Extras.ChatHeads.I, Extras.Saturation.I, Extras.TabPing.I, Extras.ScreenshotCopy.I, Hypixel.AutoGG.I, Hypixel.AutoGLHF.I, Hypixel.AutoTip.I, Hypixel.AutoBoop.I, Hypixel.NickHider.I);
	}

	public static void register(KeyMapping.Category category) {
		Zoom.key = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.shadowmenu.zoom", GLFW.GLFW_KEY_C, category));
		Freelook.key = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.shadowmenu.freelook", GLFW.GLFW_KEY_V, category));
		Visuals.Fullbright.key = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.shadowmenu.fullbright", InputConstants.UNKNOWN.getValue(), category));

		ScreenEvents.AFTER_INIT.register((mc, screen, w, h) -> {
			if (screen instanceof AbstractContainerScreen<?>) {
				ScreenMouseEvents.allowMouseScroll(screen).register((s, mx, my, horizontal, vertical) ->
						!Visuals.ScrollTooltips.scrolled(vertical != 0 ? vertical : horizontal, mc.hasShiftDown() || vertical == 0));
			}
			if (screen instanceof PauseScreen) confirmDisconnect(mc, screen);
		});
	}

	/** Swap the pause menu's Disconnect button for one that asks first (it then presses the real one). */
	private static void confirmDisconnect(Minecraft mc, net.minecraft.client.gui.screens.Screen screen) {
		if (!Visuals.ConfirmDisconnect.I.on()) return;
		List<AbstractWidget> buttons = Screens.getButtons(screen);
		Component label = CommonComponents.disconnectButtonLabel(mc.isLocalServer());
		for (int i = 0; i < buttons.size(); i++) {
			if (!(buttons.get(i) instanceof Button real) || !real.getMessage().getString().equals(label.getString())) continue;
			Button ask = Button.builder(label, b -> mc.setScreen(new ConfirmScreen(yes -> {
				if (yes) real.onPress(new net.minecraft.client.input.KeyEvent(GLFW.GLFW_KEY_ENTER, 0, 0));
				else mc.setScreen(screen);
			}, Component.literal("Leave this " + (mc.isLocalServer() ? "world" : "server") + "?"), Component.literal("Shadow Client is asking because Confirm Disconnect is on."))))
					.bounds(real.getX(), real.getY(), real.getWidth(), real.getHeight()).build();
			buttons.set(i, ask);
			return;
		}
	}

	public static void tick(Minecraft mc) {
		Zoom.tick(mc);
		Freelook.tick(mc);
		Visuals.Fullbright.tick(mc);
		Visuals.HitColor.refresh();
		Hypixel.tick(mc);
	}

	public static void attacked(Entity target) {
		Visuals.HitParticles.hit(target);
	}

	public static void addCards(List<Module> out) {
		for (Feature f : all()) out.add(f.card());
	}
}
