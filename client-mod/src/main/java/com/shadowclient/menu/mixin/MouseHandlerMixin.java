package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Freelook;
import com.shadowclient.menu.feature.Zoom;
import com.shadowclient.menu.hud.HudStats;
import net.minecraft.client.Minecraft;
import net.minecraft.client.MouseHandler;
import net.minecraft.client.input.MouseButtonInfo;
import net.minecraft.client.player.LocalPlayer;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.Redirect;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Clicks for the CPS counter, the scroll wheel while zooming, and mouse turning for zoom and freelook. */
@Mixin(MouseHandler.class)
public abstract class MouseHandlerMixin {
	@Shadow @Final private Minecraft minecraft;

	@Inject(method = "onButton", at = @At("HEAD"), require = 0)
	private void shadowmenu$countClick(long window, MouseButtonInfo info, int action, CallbackInfo ci) {
		if (action == 1 && window == minecraft.getWindow().handle() && minecraft.screen == null && minecraft.getOverlay() == null) HudStats.click(info.button());
	}

	@Inject(method = "onScroll", at = @At("HEAD"), cancellable = true, require = 0)
	private void shadowmenu$zoomScroll(long window, double x, double y, CallbackInfo ci) {
		if (window == minecraft.getWindow().handle() && minecraft.screen == null && Zoom.scrolled(y)) ci.cancel();
	}

	@Redirect(method = "turnPlayer", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/player/LocalPlayer;turn(DD)V"), require = 0)
	private void shadowmenu$turn(LocalPlayer player, double dx, double dy) {
		if (Freelook.active()) {
			Freelook.turn(dx, dy);
			return;
		}
		double s = Zoom.sensitivity();
		player.turn(dx * s, dy * s);
	}
}
