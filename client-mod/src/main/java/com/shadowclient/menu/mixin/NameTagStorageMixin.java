package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Visuals;
import net.minecraft.client.Options;
import net.minecraft.client.renderer.feature.NameTagFeatureRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/** Nametags: how dark the box behind nametags is. */
@Mixin(NameTagFeatureRenderer.Storage.class)
public abstract class NameTagStorageMixin {
	@Redirect(method = "add", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/Options;getBackgroundOpacity(F)F"), require = 0)
	private float shadowmenu$background(Options options, float fallback) {
		return Visuals.Nametags.background(options.getBackgroundOpacity(fallback));
	}
}
