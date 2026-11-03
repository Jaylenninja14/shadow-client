package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Extras;
import com.shadowclient.menu.feature.Hypixel;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.PlayerTabOverlay;
import net.minecraft.client.multiplayer.PlayerInfo;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Constant;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.ModifyConstant;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Nick Hider (names in the tab list) and Ping in Tab (numbers instead of bars, in a wider column). */
@Mixin(PlayerTabOverlay.class)
public abstract class PlayerTabOverlayMixin {
	@Inject(method = "getNameForDisplay", at = @At("RETURN"), cancellable = true, require = 0)
	private void shadowmenu$nick(PlayerInfo info, CallbackInfoReturnable<Component> cir) {
		cir.setReturnValue(Hypixel.NickHider.apply(cir.getReturnValue()));
	}

	// the 13 pixels each entry keeps for the ping bars
	@ModifyConstant(method = "render", constant = @Constant(intValue = 13), require = 0)
	private int shadowmenu$pingColumn(int width) {
		return width + Extras.TabPing.extraWidth();
	}

	@Inject(method = "renderPingIcon", at = @At("HEAD"), cancellable = true, require = 0)
	private void shadowmenu$pingNumber(GuiGraphics g, int width, int x, int y, PlayerInfo info, CallbackInfo ci) {
		if (Extras.TabPing.draw(g, width, x, y, info)) ci.cancel();
	}
}
