package com.shadowclient.menu.mixin;

import com.shadowclient.menu.ui.ShadowTitleScreen;
import net.minecraft.SharedConstants;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.ModifyVariable;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * Window title "Shadow Client 1.21.11", and the Shadow Client title screen in place of Minecraft's
 * (or any mod's title screen built on it), wherever the game opens one. AxolotlClient also sets the
 * title at the head of createTitle; the lower priority applies this mixin first, so its callback
 * runs first.
 */
@Mixin(value = Minecraft.class, priority = 500)
public abstract class MinecraftMixin {
	@Inject(method = "createTitle", at = @At("HEAD"), cancellable = true, require = 0)
	private void shadowmenu$title(CallbackInfoReturnable<String> cir) {
		cir.setReturnValue("Shadow Client " + SharedConstants.getCurrentVersion().name());
	}

	// first thing in setScreen, so mods that look at (or keep) the new screen only ever see ours;
	// Essential, for one, keeps it to show again after its own welcome screen
	@ModifyVariable(method = "setScreen", at = @At("HEAD"), argsOnly = true, require = 0)
	private Screen shadowmenu$titleScreenFirst(Screen screen) {
		return screen instanceof TitleScreen ? new ShadowTitleScreen() : screen;
	}

	// setScreen(null) without a world (joining a server, leaving one) makes a new TitleScreen and
	// stores it in the same variable: swap it right there, so the screen that gets stored is the
	// one that gets set up. (Changing it just before the field write would be too late: the old
	// value is already loaded for the write, and Fabric then finds a screen that was never set up.)
	@ModifyVariable(method = "setScreen", at = @At("STORE"), argsOnly = true, require = 0)
	private Screen shadowmenu$titleScreen(Screen screen) {
		return screen instanceof TitleScreen ? new ShadowTitleScreen() : screen;
	}
}
