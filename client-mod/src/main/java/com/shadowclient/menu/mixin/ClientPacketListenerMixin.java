package com.shadowclient.menu.mixin;

import com.shadowclient.menu.hud.HudStats;
import com.shadowclient.menu.module.PvpTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientPacketListener;
import net.minecraft.network.protocol.game.ClientboundEntityEventPacket;
import net.minecraft.network.protocol.game.ClientboundSetTimePacket;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/**
 * Counts totem pops for the Totem Pop Counter and Target HUD (entity event 35 = a totem used), and
 * times the server's clock packets for the TPS module.
 */
@Mixin(ClientPacketListener.class)
public abstract class ClientPacketListenerMixin {
	private static final byte SHADOWMENU_TOTEM_EVENT = 35;

	// RETURN runs on the game thread (the packet is first handed over from the network thread)
	@Inject(method = "handleEntityEvent", at = @At("RETURN"), require = 0)
	private void shadowmenu$totemPop(ClientboundEntityEventPacket packet, CallbackInfo ci) {
		Minecraft mc = Minecraft.getInstance();
		if (packet.getEventId() == SHADOWMENU_TOTEM_EVENT && mc.level != null && mc.isSameThread()) PvpTracker.totemPopped(packet.getEntity(mc.level));
	}

	@Inject(method = "handleSetTime", at = @At("RETURN"), require = 0)
	private void shadowmenu$serverTick(ClientboundSetTimePacket packet, CallbackInfo ci) {
		if (Minecraft.getInstance().isSameThread()) HudStats.timePacket();
	}
}
