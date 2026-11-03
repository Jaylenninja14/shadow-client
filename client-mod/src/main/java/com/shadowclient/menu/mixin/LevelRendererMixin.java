package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Visuals;
import net.minecraft.client.renderer.LevelRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyArg;

/** Block Outline: colour and thickness of the normal (not high-contrast) selection outline. */
@Mixin(LevelRenderer.class)
public abstract class LevelRendererMixin {
	private static final String SHADOWMENU_OUTLINE = "Lnet/minecraft/client/renderer/LevelRenderer;renderHitOutline(Lcom/mojang/blaze3d/vertex/PoseStack;Lcom/mojang/blaze3d/vertex/VertexConsumer;DDDLnet/minecraft/client/renderer/state/BlockOutlineRenderState;IF)V";

	@ModifyArg(method = "renderBlockOutline", at = @At(value = "INVOKE", target = SHADOWMENU_OUTLINE, ordinal = 1), index = 6, require = 0)
	private int shadowmenu$outlineColor(int color) {
		return Visuals.BlockOutline.color(color);
	}

	@ModifyArg(method = "renderBlockOutline", at = @At(value = "INVOKE", target = SHADOWMENU_OUTLINE, ordinal = 1), index = 7, require = 0)
	private float shadowmenu$outlineWidth(float width) {
		return Visuals.BlockOutline.width(width);
	}
}
