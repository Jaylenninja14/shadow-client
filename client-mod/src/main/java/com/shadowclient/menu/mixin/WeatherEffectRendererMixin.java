package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Visuals;
import net.minecraft.client.Camera;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.client.renderer.WeatherEffectRenderer;
import net.minecraft.client.renderer.state.WeatherRenderState;
import net.minecraft.server.level.ParticleStatus;
import net.minecraft.world.level.Level;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** No Rain: no rain or snow drawn, and no splashes or rain sounds. */
@Mixin(WeatherEffectRenderer.class)
public abstract class WeatherEffectRendererMixin {
	@Inject(method = "extractRenderState", at = @At("HEAD"), cancellable = true, require = 0)
	private void shadowmenu$noRain(Level level, int ticks, float partial, Vec3 camera, WeatherRenderState state, CallbackInfo ci) {
		if (!Visuals.NoRain.I.on()) return;
		state.intensity = 0;
		state.rainColumns.clear();
		state.snowColumns.clear();
		ci.cancel();
	}

	@Inject(method = "tickRainParticles", at = @At("HEAD"), cancellable = true, require = 0)
	private void shadowmenu$noSplashes(ClientLevel level, Camera camera, int ticks, ParticleStatus particles, int radius, CallbackInfo ci) {
		if (Visuals.NoRain.I.on()) ci.cancel();
	}
}
