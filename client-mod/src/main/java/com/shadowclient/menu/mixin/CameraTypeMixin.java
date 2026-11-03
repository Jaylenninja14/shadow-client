package com.shadowclient.menu.mixin;

import com.shadowclient.menu.module.Tweaks;
import net.minecraft.client.CameraType;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Third Person Back Only: F5 switches between first person and the back view, skipping the front view. */
@Mixin(CameraType.class)
public abstract class CameraTypeMixin {
	@Inject(method = "cycle", at = @At("RETURN"), cancellable = true, require = 0)
	private void shadowmenu$skipFront(CallbackInfoReturnable<CameraType> cir) {
		if (cir.getReturnValue() == CameraType.THIRD_PERSON_FRONT && Tweaks.is("third-person-back-only")) cir.setReturnValue(CameraType.FIRST_PERSON);
	}
}
