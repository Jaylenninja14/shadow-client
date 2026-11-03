package com.shadowclient.menu.mixin;

import com.mojang.blaze3d.resource.CrossFrameResourcePool;
import com.mojang.blaze3d.vertex.PoseStack;
import com.shadowclient.menu.feature.MotionBlur;
import com.shadowclient.menu.feature.Visuals;
import com.shadowclient.menu.feature.Zoom;
import com.shadowclient.menu.module.Tweaks;
import net.minecraft.client.Camera;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.renderer.GameRenderer;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import org.joml.Matrix4f;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.Redirect;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * Camera and screen effects: zoom (field of view), no hurt cam, minimal view bob, hand hidden while
 * zoomed, motion blur after the world is drawn, and No Totem Animation.
 */
@Mixin(GameRenderer.class)
public abstract class GameRendererMixin {
	@Shadow @Final private CrossFrameResourcePool resourcePool;

	@Shadow
	protected abstract void bobView(PoseStack poseStack, float partial);

	/** No Totem Animation: skips the big totem popping up on screen (the sound and particles stay). */
	@Inject(method = "displayItemActivation", at = @At("HEAD"), cancellable = true, require = 0)
	private void shadowmenu$noTotemAnimation(ItemStack stack, CallbackInfo ci) {
		if (stack.is(Items.TOTEM_OF_UNDYING) && Tweaks.is(Tweaks.NO_TOTEM_ANIMATION)) ci.cancel();
	}

	// the world's field of view only (the hand is drawn with a fixed one)
	@Inject(method = "getFov", at = @At("RETURN"), cancellable = true, require = 0)
	private void shadowmenu$zoom(Camera camera, float partial, boolean useOption, CallbackInfoReturnable<Float> cir) {
		if (!useOption) return;
		float d = Zoom.divisor();
		if (d != 1f) cir.setReturnValue(cir.getReturnValue() / d);
	}

	@Inject(method = "bobHurt", at = @At("HEAD"), cancellable = true, require = 0)
	private void shadowmenu$noHurtCam(PoseStack pose, float partial, CallbackInfo ci) {
		if (Visuals.NoHurtCam.I.on()) ci.cancel();
	}

	@Redirect(method = "renderLevel", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/renderer/GameRenderer;bobView(Lcom/mojang/blaze3d/vertex/PoseStack;F)V"), require = 0)
	private void shadowmenu$worldBob(GameRenderer self, PoseStack pose, float partial) {
		if (!Visuals.MinimalBob.I.on()) bobView(pose, partial);
	}

	@Inject(method = "renderItemInHand", at = @At("HEAD"), cancellable = true, require = 0)
	private void shadowmenu$hideHand(float partial, boolean sleeping, Matrix4f projection, CallbackInfo ci) {
		if (Zoom.hidesHand()) ci.cancel();
	}

	/** Last line of defence for the built-in title screen: a Minecraft one is swapped before it's drawn. */
	@Inject(method = "render", at = @At("HEAD"), require = 0)
	private void shadowmenu$titleScreenGuard(DeltaTracker delta, boolean renderLevel, CallbackInfo ci) {
		net.minecraft.client.Minecraft mc = net.minecraft.client.Minecraft.getInstance();
		if (mc.screen instanceof net.minecraft.client.gui.screens.TitleScreen) mc.setScreen(new com.shadowclient.menu.ui.ShadowTitleScreen());
	}

	@Inject(method = "render", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/renderer/LevelRenderer;doEntityOutline()V", shift = At.Shift.AFTER), require = 0)
	private void shadowmenu$motionBlur(DeltaTracker delta, boolean renderLevel, CallbackInfo ci) {
		MotionBlur.apply(resourcePool);
	}
}
