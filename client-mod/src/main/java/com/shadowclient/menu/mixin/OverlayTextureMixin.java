package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Visuals;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.client.renderer.texture.OverlayTexture;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Hit Color: hands the hurt-tint texture to the module so it can recolour it. */
@Mixin(OverlayTexture.class)
public abstract class OverlayTextureMixin {
	@Shadow @Final private DynamicTexture texture;

	@Inject(method = "<init>", at = @At("RETURN"), require = 0)
	private void shadowmenu$created(CallbackInfo ci) {
		Visuals.HitColor.created(texture);
	}
}
