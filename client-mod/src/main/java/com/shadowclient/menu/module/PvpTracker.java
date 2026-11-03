package com.shadowclient.menu.module;

import java.util.HashMap;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.world.entity.Entity;
import org.jetbrains.annotations.Nullable;

/**
 * What the PvP HUD lines remember during a session: totem pops (by player name, from the server's
 * totem event), your deaths and where the last one was, and when you joined.
 */
public final class PvpTracker {
	private static final Map<String, Integer> POPS = new HashMap<>();
	@Nullable private static String lastPopped;
	private static long sessionStart = System.currentTimeMillis();
	private static int deaths;
	@Nullable private static String lastDeath;
	private static boolean wasDead;

	private PvpTracker() {
	}

	/** A new world or server: start counting again. */
	public static void reset() {
		POPS.clear();
		lastPopped = null;
		sessionStart = System.currentTimeMillis();
		deaths = 0;
		lastDeath = null;
		wasDead = false;
		Combat.reset();
	}

	public static void totemPopped(@Nullable Entity entity) {
		if (entity == null) return;
		String name = entity.getName().getString();
		POPS.merge(name, 1, Integer::sum);
		lastPopped = name;
		if (entity == net.minecraft.client.Minecraft.getInstance().player) Combat.onOwnTotemPop();
	}

	public static int pops(Entity entity) {
		return POPS.getOrDefault(entity.getName().getString(), 0);
	}

	@Nullable
	public static String lastPop() {
		return lastPopped == null ? null : lastPopped + " ×" + POPS.getOrDefault(lastPopped, 0);
	}

	public static void tick(Minecraft mc) {
		if (mc.player == null) return;
		boolean dead = mc.player.isDeadOrDying();
		if (dead && !wasDead) {
			deaths++;
			BlockPos p = mc.player.blockPosition();
			lastDeath = p.getX() + " " + p.getY() + " " + p.getZ();
			Combat.onDeath();
			Waypoints.died(p);
			// your own pops reset when you die, like a fresh life
			POPS.remove(mc.player.getName().getString());
		}
		wasDead = dead;
	}

	public static long sessionMillis() {
		return System.currentTimeMillis() - sessionStart;
	}

	public static int deaths() {
		return deaths;
	}

	@Nullable
	public static String lastDeath() {
		return lastDeath;
	}
}
