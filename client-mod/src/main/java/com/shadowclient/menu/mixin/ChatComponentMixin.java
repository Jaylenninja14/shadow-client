package com.shadowclient.menu.mixin;

import com.shadowclient.menu.module.Tweaks;
import java.time.LocalTime;
import java.time.format.DateTimeFormatter;
import java.util.List;
import net.minecraft.ChatFormatting;
import net.minecraft.client.GuiMessage;
import net.minecraft.client.gui.components.ChatComponent;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Constant;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.ModifyConstant;
import org.spongepowered.asm.mixin.injection.ModifyVariable;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Chat Timestamps, Compact Chat (repeats stack as "×3"), Longer Chat History (1000 lines instead of 100), Nick Hider and Chat Heads. */
@Mixin(ChatComponent.class)
public abstract class ChatComponentMixin {
	private static final DateTimeFormatter SHADOWMENU_TIME = DateTimeFormatter.ofPattern("HH:mm");

	@Shadow @Final private List<GuiMessage> allMessages;
	@Shadow @Final private List<GuiMessage.Line> trimmedMessages;

	@Unique private String shadowmenu$lastText;
	@Unique private int shadowmenu$repeats = 1;
	@Unique private int shadowmenu$lastLines;
	@Unique private int shadowmenu$linesBefore;

	// every chat line goes through this overload (the one-argument version calls it)
	@ModifyVariable(method = "addMessage(Lnet/minecraft/network/chat/Component;Lnet/minecraft/network/chat/MessageSignature;Lnet/minecraft/client/GuiMessageTag;)V",
			at = @At("HEAD"), argsOnly = true, require = 0)
	private Component shadowmenu$beforeAdd(Component message) {
		message = com.shadowclient.menu.feature.Extras.ChatHeads.decorate(com.shadowclient.menu.feature.Hypixel.NickHider.apply(message));
		Component out = message;
		if (Tweaks.is("compact-chat")) {
			String text = message.getString();
			if (text.equals(shadowmenu$lastText) && !allMessages.isEmpty() && trimmedMessages.size() >= shadowmenu$lastLines) {
				// take the previous copy off the screen and show it once with a count
				shadowmenu$repeats++;
				allMessages.remove(0);
				for (int i = 0; i < shadowmenu$lastLines; i++) trimmedMessages.remove(0);
				out = message.copy().append(Component.literal(" (×" + shadowmenu$repeats + ")").withStyle(ChatFormatting.GRAY));
			} else {
				shadowmenu$repeats = 1;
			}
			shadowmenu$lastText = text;
		}
		if (Tweaks.is(Tweaks.CHAT_TIMESTAMPS)) {
			out = Component.literal("[" + LocalTime.now().format(SHADOWMENU_TIME) + "] ").withStyle(ChatFormatting.GRAY).append(out);
		}
		return out;
	}

	// how many wrapped lines the last message took, so a repeat can remove exactly those
	@Inject(method = "addMessageToDisplayQueue", at = @At("HEAD"), require = 0)
	private void shadowmenu$countBefore(GuiMessage message, CallbackInfo ci) {
		shadowmenu$linesBefore = trimmedMessages.size();
	}

	@Inject(method = "addMessageToDisplayQueue", at = @At("RETURN"), require = 0)
	private void shadowmenu$countAfter(GuiMessage message, CallbackInfo ci) {
		shadowmenu$lastLines = Math.max(0, trimmedMessages.size() - shadowmenu$linesBefore);
	}

	@ModifyConstant(method = {"addMessageToQueue", "addMessageToDisplayQueue"}, constant = @Constant(intValue = 100), require = 0)
	private int shadowmenu$history(int max) {
		return Tweaks.is(Tweaks.LONG_CHAT) ? 1000 : max;
	}
}
