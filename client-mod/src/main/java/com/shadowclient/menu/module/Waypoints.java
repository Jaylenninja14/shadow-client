package com.shadowclient.menu.module;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.ui.WaypointsScreen;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.List;
import java.util.Locale;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.core.BlockPos;
import net.minecraft.util.Mth;
import net.minecraft.world.item.Items;
import org.jetbrains.annotations.Nullable;

/**
 * Waypoints per world/server: added with a key where you stand (and automatically where you die),
 * shown on a compass bar at the top of the screen with their distance. Kept in
 * config/shadowmenu.json ("waypoints" → world → list).
 */
public final class Waypoints {
	private Waypoints() {
	}

	public record Point(String name, int x, int y, int z, String dimension, int colour) {
	}

	private static final int[] COLOURS = {0xFF9B7BFF, 0xFFFF8F1F, 0xFF55FFFF, 0xFF7AC943, 0xFFFF5C5C, 0xFFFFD24A};

	/** "server address" or "world name" for the world you're in; null on the title screen. */
	@Nullable
	public static String worldKey() {
		Minecraft mc = Minecraft.getInstance();
		if (mc.level == null) return null;
		if (mc.getCurrentServer() != null) return "server:" + mc.getCurrentServer().ip;
		if (mc.getSingleplayerServer() != null) return "world:" + mc.getSingleplayerServer().getWorldData().getLevelName();
		return "other";
	}

	private static String dimension(Minecraft mc) {
		return mc.level.dimension().identifier().toString();
	}

	public static List<Point> list() {
		List<Point> out = new ArrayList<>();
		String key = worldKey();
		JsonObject all = MenuConfig.object("waypoints");
		if (key == null || all == null || !all.has(key)) return out;
		for (JsonElement e : all.getAsJsonArray(key)) {
			JsonObject o = e.getAsJsonObject();
			out.add(new Point(o.get("name").getAsString(), o.get("x").getAsInt(), o.get("y").getAsInt(), o.get("z").getAsInt(),
					o.get("dim").getAsString(), o.has("colour") ? o.get("colour").getAsInt() : COLOURS[0]));
		}
		return out;
	}

	private static void save(List<Point> points) {
		String key = worldKey();
		if (key == null) return;
		JsonObject all = MenuConfig.object("waypoints");
		JsonObject next = all == null ? new JsonObject() : all.deepCopy();
		JsonArray a = new JsonArray();
		for (Point p : points) {
			JsonObject o = new JsonObject();
			o.addProperty("name", p.name());
			o.addProperty("x", p.x());
			o.addProperty("y", p.y());
			o.addProperty("z", p.z());
			o.addProperty("dim", p.dimension());
			o.addProperty("colour", p.colour());
			a.add(o);
		}
		next.add(key, a);
		MenuConfig.putObject("waypoints", next);
	}

	/** Add a waypoint where the player stands (the add-waypoint key). */
	public static void addHere() {
		Minecraft mc = Minecraft.getInstance();
		if (mc.player == null) return;
		List<Point> points = list();
		BlockPos p = mc.player.blockPosition();
		int n = points.size() + 1;
		points.add(new Point("Waypoint " + n, p.getX(), p.getY(), p.getZ(), dimension(mc), COLOURS[(n - 1) % COLOURS.length]));
		save(points);
		mc.player.displayClientMessage(net.minecraft.network.chat.Component.literal("Waypoint " + n + " added at " + p.getX() + " " + p.getY() + " " + p.getZ()), true);
	}

	/** A single "Death" waypoint, replaced on every death. */
	public static void died(BlockPos p) {
		if (!Tweaks.is("death-waypoints")) return;
		Minecraft mc = Minecraft.getInstance();
		List<Point> points = list();
		points.removeIf(w -> w.name().equals("Death"));
		points.add(new Point("Death", p.getX(), p.getY(), p.getZ(), dimension(mc), 0xFFFF5C5C));
		save(points);
	}

	public static void remove(int index) {
		List<Point> points = list();
		if (index >= 0 && index < points.size()) points.remove(index);
		save(points);
	}

	public static void clear() {
		save(new ArrayList<>());
	}

	// ------------------------------------------------------------------ the compass bar

	public static void render(GuiGraphics g, Minecraft mc) {
		if (!Tweaks.is("waypoints")) return;
		LocalPlayer player = mc.player;
		List<Point> points = list();
		String dim = dimension(mc);
		int w = g.guiWidth(), barW = Math.min(240, w - 120), x0 = (w - barW) / 2, y0 = 4;
		boolean any = false;
		for (Point p : points) {
			if (!p.dimension().equals(dim)) continue;
			double dx = p.x() + 0.5 - player.getX(), dz = p.z() + 0.5 - player.getZ();
			// angle of the waypoint relative to where you look, -180..180
			double yawTo = Math.toDegrees(Math.atan2(-dx, dz));
			double rel = Mth.wrapDegrees(yawTo - player.getYRot());
			if (Math.abs(rel) > 90) continue;
			if (!any) {
				g.fill(x0, y0 + 6, x0 + barW, y0 + 7, 0x80FFFFFF);
				g.fill(x0 + barW / 2, y0 + 3, x0 + barW / 2 + 1, y0 + 10, 0xC0FFFFFF);
				any = true;
			}
			int px = x0 + (int) Math.round((rel + 90) / 180.0 * barW);
			int dist = (int) Math.round(Math.sqrt(dx * dx + dz * dz));
			g.fill(px - 2, y0 + 4, px + 3, y0 + 9, p.colour());
			String label = p.name() + " " + dist + "m";
			int lw = mc.font.width(label);
			g.drawString(mc.font, label, px - lw / 2, y0 + 12, p.colour(), true);
		}
	}

	// ------------------------------------------------------------------ menu cards

	public static void addTo(List<Module> out) {
		out.add(new Module("waypoints", "Waypoints", "compass", EnumSet.of(Tab.PVP, Tab.HUD), () -> Tweaks.is("waypoints"), v -> Tweaks.set("waypoints", v),
				WaypointsScreen::new, null).item(Items.LODESTONE));
		out.add(new Module("death-waypoints", "Death Waypoints", "coords", EnumSet.of(Tab.PVP, Tab.HUD), () -> Tweaks.is("death-waypoints"),
				v -> Tweaks.set("death-waypoints", v), WaypointsScreen::new, null).item(Items.RECOVERY_COMPASS));
		out.add(new Module("waypoint-list", "Waypoint List", "coords", EnumSet.of(Tab.HUD), () -> true, null, WaypointsScreen::new,
				String.format(Locale.ROOT, "Key: %s", "B")).item(Items.MAP));
	}
}
