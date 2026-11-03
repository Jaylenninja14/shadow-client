package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Visuals;
import net.minecraft.world.level.Level;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Time Changer: the time of day your world (sky, light) shows; the server's time is untouched. */
@Mixin(Level.class)
public abstract class LevelMixin {
	@Inject(method = "getDayTime", at = @At("RETURN"), cancellable = true, require = 0)
	private void shadowmenu$timeChanger(CallbackInfoReturnable<Long> cir) {
		if (((Level) (Object) this).isClientSide()) cir.setReturnValue(Visuals.TimeChanger.dayTime(cir.getReturnValue()));
	}
}
