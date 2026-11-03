package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Extras;
import com.shadowclient.menu.feature.Visuals;
import net.minecraft.client.gui.Gui;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Remove Vignette (no dark screen corners) and the Saturation Display on the hunger bar. */
@Mixin(Gui.class)
public abstract class GuiMixin {
	@Inject(method = "renderVignette", at = @At("HEAD"), cancellable = true, require = 0)
	private void shadowmenu$noVignette(GuiGraphics g, Entity entity, CallbackInfo ci) {
		if (Visuals.NoVignette.I.on()) ci.cancel();
	}

	@Inject(method = "renderFood", at = @At("RETURN"), require = 0)
	private void shadowmenu$saturation(GuiGraphics g, Player player, int y, int right, CallbackInfo ci) {
		Extras.Saturation.drawn(g, player, y, right);
	}
}
