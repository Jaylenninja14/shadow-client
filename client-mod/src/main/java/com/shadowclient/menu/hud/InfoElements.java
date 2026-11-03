package com.shadowclient.menu.hud;

import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ChoiceSetting;
import com.shadowclient.menu.setting.NumberSetting;
import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import net.minecraft.client.multiplayer.PlayerInfo;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.server.packs.repository.Pack;
import net.minecraft.util.Mth;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import org.jetbrains.annotations.Nullable;

/** The one-line and list HUD modules: FPS, CPS, ping, coordinates, clock, speed, reach, … */
final class InfoElements {
	private InfoElements() {
	}

	static final Set<Tab> HUD = EnumSet.of(Tab.HUD);
	static final Set<Tab> PVP = EnumSet.of(Tab.HUD, Tab.PVP);

	static void addTo(List<HudElement> out) {
		out.add(new Fps());
		out.add(new Cps());
		out.add(new Ping());
		out.add(new Coordinates());
		out.add(new Clock());
		out.add(new Speed());
		out.add(new Reach());
		out.add(new Combo());
		out.add(new SprintStatus());
		out.add(new Memory());
		out.add(new ServerAddress());
		out.add(new Tps());
		out.add(new PackDisplay());
		out.add(new ArrowCounter());
	}

	/** "144 FPS", "FPS: 144" or just "144". */
	abstract static class Labelled extends TextElement {
		final ChoiceSetting label;

		Labelled(String id, String name, String icon, Set<Tab> tabs, boolean on, double x, double y) {
			super(id, name, icon, tabs, on, x, y);
			label = add(new ChoiceSetting("label", "Label", "after", List.of("after", "before", "none"), List.of("After (144 FPS)", "Before (FPS: 144)", "None")));
		}

		String labelled(String value, String unit) {
			return switch (label.get()) {
				case "before" -> unit + ": " + value;
				case "none" -> value;
				default -> value + " " + unit;
			};
		}
	}

	static String fmt(double v, int decimals) {
		return String.format(Locale.ROOT, "%." + decimals + "f", v);
	}

	// ------------------------------------------------------------------ elements

	static final class Fps extends Labelled {
		Fps() {
			super("fps", "FPS", "fps", HUD, true, 0.004, 0.008);
			describe("Frames per second.");
			anchor(Anchor.TOP_LEFT);
		}

		@Override
		protected String text(boolean preview) {
			return labelled(String.valueOf(mc.getFps()), "FPS");
		}
	}

	static final class Cps extends Labelled {
		final BoolSetting right = add(new BoolSetting("right", "Show right clicks", true));

		Cps() {
			super("cps", "CPS", "cps", PVP, true, 0.004, 0.068);
			describe("Clicks per second (left | right).");
			anchor(Anchor.TOP_LEFT);
		}

		@Override
		protected String text(boolean preview) {
			String v = right.on() ? HudStats.leftCps() + " | " + HudStats.rightCps() : String.valueOf(HudStats.leftCps());
			return labelled(v, "CPS");
		}
	}

	static final class Ping extends Labelled {
		Ping() {
			super("ping", "Ping", "ping", PVP, true, 0.004, 0.128);
			describe("Your latency to the server (singleplayer: 0).");
			anchor(Anchor.TOP_LEFT);
		}

		@Override
		protected String text(boolean preview) {
			int ms = 0;
			if (mc.getConnection() != null && mc.player != null) {
				PlayerInfo info = mc.getConnection().getPlayerInfo(mc.player.getUUID());
				if (info != null) ms = info.getLatency();
			}
			return labelled(String.valueOf(ms), "ms");
		}
	}

	static final class Coordinates extends LinesElement {
		final ChoiceSetting layout = add(new ChoiceSetting("layout", "Layout", "lines", List.of("lines", "one-line"), List.of("One per line", "One line")));
		final NumberSetting decimals = add(new NumberSetting("decimals", "Decimals", 0, 0, 2, 1));
		final BoolSetting facing = add(new BoolSetting("facing", "Show direction", true));
		final BoolSetting biome = add(new BoolSetting("biome", "Show biome", false));

		Coordinates() {
			super("coordinates", "Coordinates", "coords", HUD, false, 0.004, 0.19);
			describe("Your X, Y and Z, plus the way you face.");
			anchor(Anchor.TOP_LEFT);
		}

		@Override
		protected List<String> lines(boolean preview) {
			LocalPlayer p = mc.player;
			double x = p != null ? p.getX() : 128.5, y = p != null ? p.getY() : 64, z = p != null ? p.getZ() : -310.25;
			int d = decimals.i();
			List<String> out = new ArrayList<>();
			String dir = p != null ? facing(p.getYRot()) : "N";
			if (layout.is("one-line")) {
				out.add(fmt(x, d) + ", " + fmt(y, d) + ", " + fmt(z, d) + (facing.on() ? "  " + dir : ""));
			} else {
				out.add("X: " + fmt(x, d) + (facing.on() ? "  " + axis(p, true) : ""));
				out.add("Y: " + fmt(y, d));
				out.add("Z: " + fmt(z, d) + (facing.on() ? "  " + axis(p, false) : ""));
				if (facing.on()) out.add("Facing: " + dir);
			}
			if (biome.on()) out.add("Biome: " + biomeName(preview));
			return out;
		}

		private String biomeName(boolean preview) {
			if (mc.level == null || mc.player == null) return "Plains";
			return mc.level.getBiome(mc.player.blockPosition()).unwrapKey().map(k -> pretty(k.identifier().getPath())).orElse("?");
		}

		/** "+" or "-" for the direction X (or Z) grows as you walk forward. */
		private static String axis(@Nullable LocalPlayer p, boolean xAxis) {
			if (p == null) return xAxis ? "+" : "-";
			double rad = Math.toRadians(p.getYRot());
			double v = xAxis ? -Math.sin(rad) : Math.cos(rad);
			return Math.abs(v) < 0.38 ? "" : v > 0 ? "+" : "-";
		}
	}

	static String facing(float yaw) {
		String[] names = {"S", "SW", "W", "NW", "N", "NE", "E", "SE"};
		int i = Math.floorMod(Math.round(Mth.wrapDegrees(yaw) / 45f), 8);
		return names[i];
	}

	static String pretty(String id) {
		StringBuilder out = new StringBuilder();
		for (String w : id.split("_")) {
			if (w.isEmpty()) continue;
			if (!out.isEmpty()) out.append(' ');
			out.append(Character.toUpperCase(w.charAt(0))).append(w.substring(1));
		}
		return out.toString();
	}

	static final class Clock extends TextElement {
		final BoolSetting hours24 = add(new BoolSetting("24h", "24-hour clock", true));
		final BoolSetting seconds = add(new BoolSetting("seconds", "Show seconds", false));
		final BoolSetting date = add(new BoolSetting("date", "Show date", false));

		Clock() {
			super("clock", "Clock", "clock", HUD, false, 0.996, 0.008);
			describe("Your computer's time.");
			anchor(Anchor.TOP_RIGHT);
		}

		@Override
		protected String text(boolean preview) {
			String pattern = (hours24.on() ? "HH:mm" : "h:mm") + (seconds.on() ? ":ss" : "") + (hours24.on() ? "" : " a");
			if (date.on()) pattern = "EEE d MMM  " + pattern;
			return LocalDateTime.now().format(DateTimeFormatter.ofPattern(pattern, Locale.ENGLISH));
		}
	}

	static final class Speed extends Labelled {
		final ChoiceSetting unit = add(new ChoiceSetting("unit", "Unit", "bps", List.of("bps", "kmh"), List.of("Blocks/s", "km/h")));
		final BoolSetting vertical = add(new BoolSetting("vertical", "Count up and down too", false));
		final NumberSetting decimals = add(new NumberSetting("decimals", "Decimals", 2, 0, 2, 1));
		private double smooth;

		Speed() {
			super("speed", "Speed", "speed", HUD, false, 0.004, 0.25);
			describe("How fast you're moving.");
			anchor(Anchor.TOP_LEFT);
		}

		@Override
		protected String text(boolean preview) {
			double bps = 0;
			LocalPlayer p = mc.player;
			if (p != null) {
				double dx = p.getX() - p.xo, dy = vertical.on() ? p.getY() - p.yo : 0, dz = p.getZ() - p.zo;
				bps = Math.sqrt(dx * dx + dy * dy + dz * dz) * 20;
			}
			smooth += (bps - smooth) * 0.2;
			double v = unit.is("kmh") ? smooth * 3.6 : smooth;
			return labelled(fmt(v, decimals.i()), unit.is("kmh") ? "km/h" : "m/s");
		}
	}

	static final class Reach extends TextElement {
		final NumberSetting decimals = add(new NumberSetting("decimals", "Decimals", 2, 0, 3, 1));
		final NumberSetting timeout = add(new NumberSetting("timeout", "Hide after", 3, 1, 10, 1).suffix(" s"));
		final BoolSetting idle = add(new BoolSetting("idle", "Show when idle", true));

		Reach() {
			super("reach", "Reach Display", "reach", PVP, false, 0.004, 0.31);
			describe("How far away your last hit landed.");
			anchor(Anchor.TOP_LEFT);
		}

		@Override
		protected String text(boolean preview) {
			double r = HudStats.reach(Math.round(timeout.get() * 1000));
			if (r < 0) return preview ? "3.00 blocks" : idle.on() ? "Hasn't attacked" : null;
			return fmt(r, decimals.i()) + " blocks";
		}
	}

	static final class Combo extends TextElement {
		final BoolSetting idle = add(new BoolSetting("idle", "Show when idle", true));

		Combo() {
			super("combo", "Combo Display", "combo", PVP, false, 0.004, 0.37);
			describe("Hits in a row without getting hit back.");
			anchor(Anchor.TOP_LEFT);
		}

		@Override
		protected String text(boolean preview) {
			int c = HudStats.combo();
			if (c == 0) return preview ? "3 Combo" : idle.on() ? "No Combo" : null;
			return c + " Combo";
		}
	}

	static final class SprintStatus extends TextElement {
		final BoolSetting sneak = add(new BoolSetting("sneak", "Show sneaking", true));
		final BoolSetting fly = add(new BoolSetting("fly", "Show flying", true));

		SprintStatus() {
			super("sprint-status", "Sprint Status", "sprint", PVP, false, 0.004, 0.97);
			describe("Shows whether you're sprinting, sneaking or flying (and whether it's toggled).");
			anchor(Anchor.TOP_LEFT);
		}

		@Override
		protected String text(boolean preview) {
			LocalPlayer p = mc.player;
			if (p == null) return preview ? "Sprinting (Toggled)" : null;
			if (fly.on() && p.getAbilities().flying) return "Flying";
			if (sneak.on() && p.isShiftKeyDown()) return mc.options.toggleCrouch().get() && mc.options.keyShift.isDown() ? "Sneaking (Toggled)" : "Sneaking";
			if (mc.options.toggleSprint().get() && mc.options.keySprint.isDown()) return "Sprinting (Toggled)";
			if (p.isSprinting()) return "Sprinting (Vanilla)";
			return preview ? "Sprinting (Toggled)" : null;
		}
	}

	static final class Memory extends TextElement {
		final ChoiceSetting style = add(new ChoiceSetting("style", "Show", "both", List.of("both", "percent", "mb"), List.of("Percent and MB", "Percent", "MB")));

		Memory() {
			super("memory", "Memory", "memory", HUD, false, 0.996, 0.068);
			describe("How much of Minecraft's memory is used.");
			anchor(Anchor.TOP_RIGHT);
		}

		@Override
		protected String text(boolean preview) {
			Runtime r = Runtime.getRuntime();
			long used = (r.totalMemory() - r.freeMemory()) / 1048576, max = r.maxMemory() / 1048576;
			int pct = (int) Math.round(used * 100.0 / Math.max(1, max));
			return switch (style.get()) {
				case "percent" -> "Mem: " + pct + "%";
				case "mb" -> "Mem: " + used + "/" + max + " MB";
				default -> "Mem: " + pct + "% " + used + "/" + max + " MB";
			};
		}
	}

	static final class ServerAddress extends TextElement {
		final BoolSetting port = add(new BoolSetting("port", "Show port", false));

		ServerAddress() {
			super("server-address", "Server Address", "server", HUD, false, 0.5, 0.008);
			describe("The server you're on.");
			anchor(Anchor.TOP_CENTER);
		}

		@Override
		protected String text(boolean preview) {
			if (mc.getCurrentServer() == null) return mc.hasSingleplayerServer() ? "Singleplayer" : preview ? "play.example.net" : null;
			String ip = mc.getCurrentServer().ip;
			if (!port.on() && ip.contains(":") && !ip.startsWith("[")) ip = ip.substring(0, ip.lastIndexOf(':'));
			return ip;
		}
	}

	static final class Tps extends Labelled {
		Tps() {
			super("tps", "TPS", "server", HUD, false, 0.996, 0.128);
			describe("The server's ticks per second (20 = no lag), measured from its clock updates.");
			anchor(Anchor.TOP_RIGHT);
		}

		@Override
		protected String text(boolean preview) {
			double t = HudStats.tps();
			return labelled(t < 0 ? "20.0" : fmt(t, 1), "TPS");
		}
	}

	static final class PackDisplay extends LinesElement {
		final NumberSetting max = add(new NumberSetting("max", "Most packs shown", 3, 1, 10, 1));

		PackDisplay() {
			super("pack-display", "Pack Display", "pack", HUD, false, 0.996, 0.97);
			describe("The resource packs you have on.");
			anchor(Anchor.BOTTOM_RIGHT);
		}

		@Override
		protected List<String> lines(boolean preview) {
			List<String> out = new ArrayList<>();
			for (Pack p : mc.getResourcePackRepository().getSelectedPacks()) {
				String id = p.getId();
				if (p.isRequired() || id.equals("vanilla") || id.startsWith("fabric") || id.equals("shadowmenu:shadow_client") || id.equals("mod_resources")) continue;
				out.add(0, p.getTitle().getString());
			}
			if (out.isEmpty()) return preview ? List.of("Default") : null;
			return out.size() > max.i() ? out.subList(0, max.i()) : out;
		}
	}

	/** Arrows you carry, with the arrow item. */
	static final class ArrowCounter extends TextElement {
		final BoolSetting withBow = add(new BoolSetting("with-bow", "Only while holding a bow", false));
		private static final ItemStack ARROW = new ItemStack(Items.ARROW);

		ArrowCounter() {
			super("arrows", "Arrow HUD", "arrow", PVP, false, 0.996, 0.25);
			describe("How many arrows you have.");
			anchor(Anchor.BOTTOM_RIGHT);
			item(Items.ARROW);
		}

		private int count() {
			LocalPlayer p = mc.player;
			if (p == null) return 64;
			int n = 0;
			for (int i = 0; i < p.getInventory().getContainerSize(); i++) {
				ItemStack s = p.getInventory().getItem(i);
				if (s.is(Items.ARROW) || s.is(Items.SPECTRAL_ARROW) || s.is(Items.TIPPED_ARROW)) n += s.getCount();
			}
			return n;
		}

		@Override
		protected String text(boolean preview) {
			if (!preview && withBow.on() && mc.player != null) {
				ItemStack held = mc.player.getMainHandItem();
				if (!held.is(Items.BOW) && !held.is(Items.CROSSBOW)) return null;
			}
			return String.valueOf(count());
		}

		@Override
		public int width(boolean preview) {
			return super.width(preview) + 14;
		}

		@Override
		public void render(net.minecraft.client.gui.GuiGraphics g, boolean preview) {
			int w = width(preview), h = height(preview);
			box(g, w, h);
			String t = shown(preview);
			int tw = font().width(t);
			int x0 = (w - tw - 14) / 2;
			com.shadowclient.menu.ui.Draw.item(g, ARROW, x0, (h - 12) / 2f, 12);
			g.drawString(font(), t, x0 + 14, (h - 8) / 2, textColor.argb(), shadow.on());
		}
	}

}
