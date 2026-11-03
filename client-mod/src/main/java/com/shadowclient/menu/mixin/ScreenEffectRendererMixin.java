package com.shadowclient.menu.mixin;

import com.mojang.blaze3d.vertex.PoseStack;
import com.shadowclient.menu.feature.Visuals;
import net.minecraft.client.renderer.MultiBufferSource;
import net.minecraft.client.renderer.ScreenEffectRenderer;
import net.minecraft.client.renderer.texture.TextureAtlasSprite;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Low Fire: the burning overlay drawn lower (or not at all). */
@Mixin(ScreenEffectRenderer.class)
public abstract class ScreenEffectRendererMixin {
	@Inject(method = "renderFire", at = @At("HEAD"), cancellable = true, require = 0)
	private static void shadowmenu$lowFireStart(PoseStack pose, MultiBufferSource buffers, TextureAtlasSprite sprite, CallbackInfo ci) {
		if (Visuals.LowFire.hidden()) {
			ci.cancel();
			return;
		}
		pose.pushPose();
		pose.translate(0, -Visuals.LowFire.offset(), 0);
	}

	@Inject(method = "renderFire", at = @At("RETURN"), require = 0)
	private static void shadowmenu$lowFireEnd(PoseStack pose, MultiBufferSource buffers, TextureAtlasSprite sprite, CallbackInfo ci) {
		pose.popPose();
	}
}
