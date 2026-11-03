package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Visuals;
import net.minecraft.client.renderer.LightTexture;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyArg;

/** Fullbright: the lightmap gets full night vision (the fourth value written to its uniform block). */
@Mixin(LightTexture.class)
public abstract class LightTextureMixin {
	@ModifyArg(method = "updateLightTexture", at = @At(value = "INVOKE", target = "Lcom/mojang/blaze3d/buffers/Std140Builder;putFloat(F)Lcom/mojang/blaze3d/buffers/Std140Builder;", ordinal = 3), require = 0)
	private float shadowmenu$fullbright(float nightVision) {
		return Visuals.Fullbright.nightVision(nightVision);
	}
}
