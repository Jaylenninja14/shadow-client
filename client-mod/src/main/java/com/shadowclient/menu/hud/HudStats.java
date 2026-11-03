package com.shadowclient.menu.hud;

import java.util.ArrayDeque;
import net.minecraft.client.Minecraft;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.phys.EntityHitResult;
import net.minecraft.world.phys.HitResult;

/**
 * What the HUD modules count while you play: clicks per second (from the mouse handler), the
 * server's ticks per second (from its once-a-second time packets), your last hit's reach and your
 * combo.
 */
public final class HudStats {
	private static final ArrayDeque<Long> LEFT = new ArrayDeque<>(), RIGHT = new ArrayDeque<>();
	private static final ArrayDeque<Double> TPS = new ArrayDeque<>();
	private static long lastTimePacket;
	private static double lastReach = -1;
	private static long lastReachAt, lastHitAt;
	private static int combo;
	private static int lastHurtTime;

	private HudStats() {
	}

	/** A mouse button went down in game (0 = left, 1 = right). */
	public static void click(int button) {
		long now = System.currentTimeMillis();
		if (button == 0) LEFT.addLast(now);
		else if (button == 1) RIGHT.addLast(now);
	}

	private static int count(ArrayDeque<Long> clicks) {
		long cut = System.currentTimeMillis() - 1000;
		while (!clicks.isEmpty() && clicks.peekFirst() < cut) clicks.removeFirst();
		return clicks.size();
	}

	public static int leftCps() {
		return count(LEFT);
	}

	public static int rightCps() {
		return count(RIGHT);
	}

	/** The server sent its time (it does so every 20 of its ticks). */
	public static void timePacket() {
		long now = System.currentTimeMillis();
		if (lastTimePacket != 0) {
			long gap = now - lastTimePacket;
			if (gap > 0 && gap < 10_000) {
				TPS.addLast(Math.min(20, 20_000.0 / gap));
				while (TPS.size() > 5) TPS.removeFirst();
			}
		}
		lastTimePacket = now;
	}

	/** Average of the last few seconds; -1 before two packets arrived. */
	public static double tps() {
		if (TPS.isEmpty()) return -1;
		double sum = 0;
		for (double t : TPS) sum += t;
		return sum / TPS.size();
	}

	public static void reset() {
		TPS.clear();
		lastTimePacket = 0;
		lastReach = -1;
		combo = 0;
	}

	/** You attacked {@code target}: remember the reach (eye to where the crosshair hit) and count the combo. */
	public static void attacked(Entity target) {
		Minecraft mc = Minecraft.getInstance();
		LocalPlayer p = mc.player;
		if (p == null) return;
		HitResult hit = mc.hitResult;
		if (hit instanceof EntityHitResult e && e.getEntity() == target) lastReach = p.getEyePosition().distanceTo(hit.getLocation());
		else lastReach = p.getEyePosition().distanceTo(target.getBoundingBox().getCenter());
		long now = System.currentTimeMillis();
		lastReachAt = now;
		if (target instanceof LivingEntity) {
			combo = now - lastHitAt > 2500 ? 1 : combo + 1;
			lastHitAt = now;
		}
	}

	public static void tick(Minecraft mc) {
		LocalPlayer p = mc.player;
		if (p == null) return;
		// getting hit ends your combo
		if (p.hurtTime > 0 && lastHurtTime == 0) combo = 0;
		lastHurtTime = p.hurtTime;
		if (System.currentTimeMillis() - lastHitAt > 2500) combo = 0;
	}

	/** Reach of the last hit, or -1 when it's older than {@code timeoutMs}. */
	public static double reach(long timeoutMs) {
		return lastReach >= 0 && System.currentTimeMillis() - lastReachAt <= timeoutMs ? lastReach : -1;
	}

	public static int combo() {
		return combo;
	}
}
