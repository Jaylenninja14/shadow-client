package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Freelook;
import net.minecraft.client.Camera;
import net.minecraft.world.entity.Entity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/** Freelook: the camera faces where you look with the freelook key held, not where the player faces. */
@Mixin(Camera.class)
public abstract class CameraMixin {
	@Redirect(method = "setup", at = @At(value = "INVOKE", target = "Lnet/minecraft/world/entity/Entity;getViewYRot(F)F"), require = 0)
	private float shadowmenu$yaw(Entity entity, float partial) {
		return Freelook.yaw(entity.getViewYRot(partial));
	}

	@Redirect(method = "setup", at = @At(value = "INVOKE", target = "Lnet/minecraft/world/entity/Entity;getViewXRot(F)F"), require = 0)
	private float shadowmenu$pitch(Entity entity, float partial) {
		return Freelook.pitch(entity.getViewXRot(partial));
	}
}
