package com.shadowclient.menu.module;

import com.shadowclient.menu.module.Module.Tab;
import java.util.ArrayDeque;
import java.util.EnumSet;
import java.util.List;
import java.util.Set;
import java.util.function.Supplier;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.contents.TranslatableContents;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EquipmentSlot;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;

/**
 * PvP session stats and alerts: kills (from death messages), kill streaks, hits landed, distance
 * walked, blocks broken, hit and kill sounds, and short on-screen alerts (pearl ready, shield
 * disabled, totem popped, low durability, low food, fall, low arrows, void, armor broke).
 */
public final class Combat {
	private Combat() {
	}

	private static int kills, streak, bestStreak, hits, blocksBroken;
	private static double distance;
	private static double lastX = Double.NaN, lastZ;
	private static final ArrayDeque<Integer> FPS = new ArrayDeque<>();
	// alert state
	private static boolean pearlWasCooling;
	private static long pearlReadyUntil, totemAlertUntil, armorBrokeUntil;
	private static String armorBroke = "";
	private static final ItemStack[] lastArmor = new ItemStack[4];
	private static final EquipmentSlot[] ARMOR = {EquipmentSlot.HEAD, EquipmentSlot.CHEST, EquipmentSlot.LEGS, EquipmentSlot.FEET};
	private static final String[] ARMOR_NAMES = {"Helmet", "Chestplate", "Leggings", "Boots"};
	private static final ItemStack PEARL = new ItemStack(Items.ENDER_PEARL), SHIELD = new ItemStack(Items.SHIELD);
	// "Steve was slain by Alex", "Steve was shot by Alex using …", Hypixel's "Steve was killed by Alex. FINAL KILL!"
	private static final Pattern KILL_LINE = Pattern.compile("^(\\w{2,16}) .*?\\bby (\\w{2,16})\\b.*$");

	public static void reset() {
		kills = streak = bestStreak = hits = blocksBroken = 0;
		distance = 0;
		lastX = Double.NaN;
		FPS.clear();
	}

	// ------------------------------------------------------------------ events

	/** A death message in chat: count it as your kill when you're the killer. */
	public static void onGameMessage(Component message) {
		Minecraft mc = Minecraft.getInstance();
		if (mc.player == null) return;
		String me = mc.player.getName().getString();
		String victim = null, killer = null;
		if (message.getContents() instanceof TranslatableContents t && t.getKey().startsWith("death.") && t.getArgs().length >= 2) {
			victim = text(t.getArgs()[0]);
			killer = text(t.getArgs()[1]);
		} else {
			Matcher m = KILL_LINE.matcher(message.getString());
			if (m.matches()) {
				victim = m.group(1);
				killer = m.group(2);
			}
		}
		if (killer != null && killer.equals(me) && !me.equals(victim)) {
			kills++;
			streak++;
			bestStreak = Math.max(bestStreak, streak);
			play(Tweaks.str("kill-sound", "off"));
		}
	}

	private static String text(Object arg) {
		return arg instanceof Component c ? c.getString() : String.valueOf(arg);
	}

	public static void onAttack(Entity target) {
		hits++;
		play(Tweaks.str("hit-sound", "off"));
	}

	public static void onBlockBroken() {
		blocksBroken++;
	}

	public static void onDeath() {
		streak = 0;
	}

	public static void onOwnTotemPop() {
		totemAlertUntil = System.currentTimeMillis() + 2500;
	}

	public static void tick(Minecraft mc) {
		LocalPlayer p = mc.player;
		if (p == null) return;
		if (!Double.isNaN(lastX)) {
			double d = Math.hypot(p.getX() - lastX, p.getZ() - lastZ);
			if (d < 10) distance += d; // ignore teleports
		}
		lastX = p.getX();
		lastZ = p.getZ();
		FPS.addLast(mc.getFps());
		while (FPS.size() > 200) FPS.removeFirst();

		boolean cooling = p.getCooldowns().isOnCooldown(PEARL);
		if (pearlWasCooling && !cooling) {
			pearlReadyUntil = System.currentTimeMillis() + 1500;
			if (ShadowHud.isOn("pearl-ready")) play("ding");
		}
		pearlWasCooling = cooling;

		for (int i = 0; i < ARMOR.length; i++) {
			ItemStack now = p.getItemBySlot(ARMOR[i]);
			ItemStack before = lastArmor[i];
			// a piece that was nearly worn out and is now gone has broken
			if (before != null && !before.isEmpty() && now.isEmpty() && before.isDamageableItem()
					&& before.getDamageValue() >= before.getMaxDamage() * 0.9) {
				armorBroke = ARMOR_NAMES[i];
				armorBrokeUntil = System.currentTimeMillis() + 3000;
			}
			lastArmor[i] = now.copy();
		}
	}

	// ------------------------------------------------------------------ sounds

	public static final String[] SOUNDS = {"off", "ding", "pling", "bell", "chime", "bass", "click", "level"};
	private static final String[] SOUND_LABELS = {"Off", "Ding", "Pling", "Bell", "Chime", "Bass", "Click", "Level Up"};

	private static void play(String sound) {
		Minecraft mc = Minecraft.getInstance();
		SimpleSoundInstance s = switch (sound) {
			case "ding" -> SimpleSoundInstance.forUI(SoundEvents.EXPERIENCE_ORB_PICKUP, 1.0f, 0.6f);
			case "pling" -> SimpleSoundInstance.forUI(SoundEvents.NOTE_BLOCK_PLING, 1.5f);
			case "bell" -> SimpleSoundInstance.forUI(SoundEvents.NOTE_BLOCK_BELL, 1.2f);
			case "chime" -> SimpleSoundInstance.forUI(SoundEvents.NOTE_BLOCK_CHIME, 1.4f);
			case "bass" -> SimpleSoundInstance.forUI(SoundEvents.NOTE_BLOCK_BASS, 1.0f);
			case "click" -> SimpleSoundInstance.forUI(SoundEvents.UI_BUTTON_CLICK, 1.6f);
			case "level" -> SimpleSoundInstance.forUI(SoundEvents.PLAYER_LEVELUP, 1.4f, 0.5f);
			default -> null;
		};
		if (s != null) mc.getSoundManager().play(s);
	}

	private static Module soundCard(String id, String name, String key, Set<Tab> tabs) {
		return Module.cycle(id, name, "sound", tabs, () -> SOUND_LABELS[index(Tweaks.str(key, "off"))], dir -> {
			String next = SOUNDS[Math.floorMod(index(Tweaks.str(key, "off")) + dir, SOUNDS.length)];
			Tweaks.setStr(key, next);
			play(next); // preview
		}, null);
	}

	private static int index(String sound) {
		for (int i = 0; i < SOUNDS.length; i++) if (SOUNDS[i].equals(sound)) return i;
		return 0;
	}

	// ------------------------------------------------------------------ HUD values

	public static String kills() {
		return String.valueOf(kills);
	}

	public static String kd() {
		int deaths = PvpTracker.deaths();
		return deaths == 0 ? kills + ".00" : String.format(java.util.Locale.ROOT, "%.2f", kills / (double) deaths);
	}

	public static String streak() {
		return String.valueOf(streak);
	}

	public static String bestStreak() {
		return String.valueOf(bestStreak);
	}

	public static String hits() {
		return String.valueOf(hits);
	}

	public static String distance() {
		return distance >= 1000 ? String.format(java.util.Locale.ROOT, "%.2f km", distance / 1000) : Math.round(distance) + " m";
	}

	public static String blocksBroken() {
		return String.valueOf(blocksBroken);
	}

	public static String fpsRange() {
		if (FPS.isEmpty()) return null;
		int min = Integer.MAX_VALUE, max = 0;
		for (int f : FPS) {
			min = Math.min(min, f);
			max = Math.max(max, f);
		}
		return min + " - " + max;
	}

	// ------------------------------------------------------------------ alerts

	/** Draw the alerts that are switched on, from {@code y} down; returns the next free y. */
	public static int renderAlerts(GuiGraphics g, Minecraft mc, Set<String> on, int y) {
		LocalPlayer p = mc.player;
		int x = g.guiWidth() / 2;
		long now = System.currentTimeMillis();
		y = alert(g, mc, on, "pearl-ready", () -> now < pearlReadyUntil ? "Pearl ready" : null, 0xFF7AC943, x, y);
		y = alert(g, mc, on, "shield-disabled", () -> p.getCooldowns().isOnCooldown(SHIELD) ? "Shield disabled" : null, 0xFFFF8F1F, x, y);
		y = alert(g, mc, on, "totem-popped", () -> now < totemAlertUntil ? "Totem popped! " + countTotems(p) + " left" : null, 0xFFFF5555, x, y);
		y = alert(g, mc, on, "low-durability", () -> {
			ItemStack held = p.getMainHandItem();
			if (held.isEmpty() || !held.isDamageableItem()) return null;
			float left = 1f - held.getDamageValue() / (float) held.getMaxDamage();
			return left < 0.1f ? held.getHoverName().getString() + " at " + Math.round(left * 100) + "%" : null;
		}, 0xFFFFD24A, x, y);
		y = alert(g, mc, on, "low-food", () -> p.getFoodData().getFoodLevel() <= 6 && !p.isCreative() ? "Eat! Hunger " + p.getFoodData().getFoodLevel() : null, 0xFFFFD24A, x, y);
		y = alert(g, mc, on, "fall-warning", () -> p.fallDistance > 4 && !p.onGround() && !p.isFallFlying() && !p.isCreative()
				? "Falling " + Math.round(p.fallDistance) + " blocks" : null, 0xFFFF5555, x, y);
		y = alert(g, mc, on, "low-arrows", () -> {
			boolean bow = p.getMainHandItem().is(Items.BOW) || p.getMainHandItem().is(Items.CROSSBOW);
			int arrows = countArrows(p);
			return bow && arrows < 8 ? "Arrows: " + arrows : null;
		}, 0xFFFFD24A, x, y);
		y = alert(g, mc, on, "void-warning", () -> p.getY() < mc.level.getMinY() + 8 ? "Void below!" : null, 0xFFFF5555, x, y);
		y = alert(g, mc, on, "armor-break", () -> now < armorBrokeUntil ? armorBroke + " broke" : null, 0xFFFF5555, x, y);
		return y;
	}

	private static int alert(GuiGraphics g, Minecraft mc, Set<String> on, String id, Supplier<String> text, int colour, int x, int y) {
		if (!on.contains(id)) return y;
		String s;
		try {
			s = text.get();
		} catch (RuntimeException e) {
			return y;
		}
		if (s == null) return y;
		g.drawCenteredString(mc.font, s, x, y, colour);
		return y + 11;
	}

	private static int countTotems(LocalPlayer p) {
		int n = 0;
		for (int i = 0; i < p.getInventory().getContainerSize(); i++) {
			ItemStack s = p.getInventory().getItem(i);
			if (s.is(Items.TOTEM_OF_UNDYING)) n += s.getCount();
		}
		return n;
	}

	private static int countArrows(LocalPlayer p) {
		int n = 0;
		for (int i = 0; i < p.getInventory().getContainerSize(); i++) {
			ItemStack s = p.getInventory().getItem(i);
			if (s.is(Items.ARROW) || s.is(Items.SPECTRAL_ARROW) || s.is(Items.TIPPED_ARROW)) n += s.getCount();
		}
		return n;
	}

	// ------------------------------------------------------------------ menu cards

	public static void addTo(List<Module> out) {
		Set<Tab> pvp = EnumSet.of(Tab.PVP);
		out.add(soundCard("hit-sound", "Hit Sound", "hit-sound", pvp).item(Items.NOTE_BLOCK));
		out.add(soundCard("kill-sound", "Kill Sound", "kill-sound", pvp).item(Items.BELL));
		out.add(new Module("compact-chat", "Compact Chat", "chat", EnumSet.of(Tab.GAME, Tab.HYPIXEL), () -> Tweaks.is("compact-chat"),
				v -> Tweaks.set("compact-chat", v), null, null));
		out.add(Module.cycle("item-size", "Held Item Size", "hand", EnumSet.of(Tab.PVP, Tab.VISUAL), () -> Tweaks.str("item-size", "100") + "%", dir -> {
			String[] sizes = {"50", "60", "70", "80", "90", "100"};
			int i = java.util.Arrays.asList(sizes).indexOf(Tweaks.str("item-size", "100"));
			Tweaks.setStr("item-size", sizes[Math.floorMod((i < 0 ? sizes.length - 1 : i) + dir, sizes.length)]);
		}, null).item(Items.DIAMOND_SWORD));
	}

	public static boolean hideOffhand() {
		return Tweaks.is("hide-offhand");
	}

	/** Scale for items held in first person (Held Item Size), 1 = normal. */
	public static float itemScale() {
		try {
			return Integer.parseInt(Tweaks.str("item-size", "100")) / 100f;
		} catch (NumberFormatException e) {
			return 1f;
		}
	}
}
