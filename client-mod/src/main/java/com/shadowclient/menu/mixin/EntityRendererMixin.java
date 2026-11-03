package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Hypixel;
import net.minecraft.client.renderer.entity.EntityRenderer;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Nick Hider: names above players. */
@Mixin(EntityRenderer.class)
public abstract class EntityRendererMixin {
	@Inject(method = "extractRenderState", at = @At("RETURN"), require = 0)
	private void shadowmenu$nick(Entity entity, EntityRenderState state, float partial, CallbackInfo ci) {
		if (state.nameTag != null && entity instanceof Player) state.nameTag = Hypixel.NickHider.apply(state.nameTag);
	}
}
