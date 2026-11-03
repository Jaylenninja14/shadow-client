package com.shadowclient.menu.mixin;

import com.mojang.blaze3d.pipeline.RenderTarget;
import com.mojang.blaze3d.platform.NativeImage;
import com.shadowclient.menu.feature.Extras;
import java.io.File;
import java.util.function.Consumer;
import net.minecraft.client.Screenshot;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.ModifyArg;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/**
 * Copy Screenshots: a screenshot taken with the screenshot key (the three-argument grab) is also
 * handed to the clipboard before Minecraft saves and closes the image.
 */
@Mixin(Screenshot.class)
public abstract class ScreenshotMixin {
	@Unique private static boolean shadowmenu$fromKey;

	@Inject(method = "grab(Ljava/io/File;Lcom/mojang/blaze3d/pipeline/RenderTarget;Ljava/util/function/Consumer;)V", at = @At("HEAD"), require = 0)
	private static void shadowmenu$keyStart(File dir, RenderTarget target, Consumer<Component> done, CallbackInfo ci) {
		shadowmenu$fromKey = true;
	}

	@Inject(method = "grab(Ljava/io/File;Lcom/mojang/blaze3d/pipeline/RenderTarget;Ljava/util/function/Consumer;)V", at = @At("RETURN"), require = 0)
	private static void shadowmenu$keyEnd(File dir, RenderTarget target, Consumer<Component> done, CallbackInfo ci) {
		shadowmenu$fromKey = false;
	}

	@ModifyArg(method = "grab(Ljava/io/File;Ljava/lang/String;Lcom/mojang/blaze3d/pipeline/RenderTarget;ILjava/util/function/Consumer;)V",
			at = @At(value = "INVOKE", target = "Lnet/minecraft/client/Screenshot;takeScreenshot(Lcom/mojang/blaze3d/pipeline/RenderTarget;ILjava/util/function/Consumer;)V"),
			index = 2, require = 0)
	private static Consumer<NativeImage> shadowmenu$copy(Consumer<NativeImage> save) {
		if (!shadowmenu$fromKey || !Extras.ScreenshotCopy.wanted()) return save;
		return image -> {
			Extras.ScreenshotCopy.copy(image);
			save.accept(image);
		};
	}
}
