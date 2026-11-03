package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Visuals;
import net.minecraft.client.renderer.entity.TntRenderer;
import net.minecraft.client.renderer.entity.state.TntRenderState;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.EntityAttachment;
import net.minecraft.world.entity.item.PrimedTnt;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** TNT Timer: lit TNT shows the seconds left as its nametag. */
@Mixin(TntRenderer.class)
public abstract class TntRendererMixin {
	@Inject(method = "extractRenderState(Lnet/minecraft/world/entity/item/PrimedTnt;Lnet/minecraft/client/renderer/entity/state/TntRenderState;F)V",
			at = @At("RETURN"), require = 0)
	private void shadowmenu$fuse(PrimedTnt tnt, TntRenderState state, float partial, CallbackInfo ci) {
		Component label = Visuals.TntTimer.label(state.fuseRemainingInTicks);
		if (label == null || state.distanceToCameraSq > 4096) return;
		state.nameTag = label;
		state.nameTagAttachment = tnt.getAttachments().getNullable(EntityAttachment.NAME_TAG, 0, tnt.getYRot(partial));
	}
}
