package com.shadowclient.menu.mixin;

import com.mojang.blaze3d.vertex.PoseStack;
import com.shadowclient.menu.module.Combat;
import net.minecraft.client.player.AbstractClientPlayer;
import net.minecraft.client.renderer.ItemInHandRenderer;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.ItemStack;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/**
 * Held Item Size: shrinks the item in first person where it is drawn (scaling around the eye would
 * look the same size), and Low Shield lowers a shield that isn't blocking. The method pushes and
 * pops its own pose, so nothing leaks.
 */
@Mixin(ItemInHandRenderer.class)
public abstract class ItemInHandRendererMixin {
	/** Hide Offhand Item: nothing is drawn for the off hand in first person (it still works). */
	@Inject(method = "renderArmWithItem", at = @At("HEAD"), cancellable = true, require = 0)
	private void shadowmenu$hideOffhand(AbstractClientPlayer player, float partialTick, float pitch, InteractionHand hand, float swing, ItemStack stack,
			float equip, PoseStack pose, SubmitNodeCollector collector, int light, CallbackInfo ci) {
		if (hand == InteractionHand.OFF_HAND && !stack.isEmpty() && Combat.hideOffhand()) ci.cancel();
	}

	@Inject(method = "renderArmWithItem", at = @At(value = "INVOKE",
			target = "Lnet/minecraft/client/renderer/ItemInHandRenderer;renderItem(Lnet/minecraft/world/entity/LivingEntity;Lnet/minecraft/world/item/ItemStack;Lnet/minecraft/world/item/ItemDisplayContext;Lcom/mojang/blaze3d/vertex/PoseStack;Lnet/minecraft/client/renderer/SubmitNodeCollector;I)V"),
			require = 0)
	private void shadowmenu$itemSize(AbstractClientPlayer player, float partialTick, float pitch, InteractionHand hand, float swing, ItemStack stack,
			float equip, PoseStack pose, SubmitNodeCollector collector, int light, CallbackInfo ci) {
		float s = Combat.itemScale();
		if (s < 0.999f) pose.scale(s, s, s);
		// Low Shield: a shield you're not blocking with sits lower
		float low = com.shadowclient.menu.feature.Visuals.LowShield.offset();
		if (low > 0 && stack.is(net.minecraft.world.item.Items.SHIELD) && !(player.isUsingItem() && player.getUsedItemHand() == hand)) pose.translate(0, -low, 0);
	}
}
