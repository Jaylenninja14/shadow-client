package com.shadowclient.menu.module;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.shadowclient.menu.module.Module.Tab;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import java.util.function.Function;
import java.util.function.Predicate;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.core.BlockPos;
import net.minecraft.core.component.DataComponents;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EquipmentSlot;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.player.Inventory;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.BowItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.LightLayer;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.HitResult;
import org.jetbrains.annotations.Nullable;

/**
 * Shadow Client's own HUD: one stack of lines in a screen corner (item counters, cooldowns, target,
 * health, coordinates helpers, …) plus PvP warnings. Each line is a card in the mod menu; which are
 * on, the corner, scale and colours are kept in config/shadowmenu.json ("hud").
 */
public final class ShadowHud {
	private ShadowHud() {
	}

	/** One HUD line. {@code value} returns null to hide the line for now (e.g. no target). */
	private record Line(String id, String name, Set<Tab> tabs, @Nullable Item item, String icon, Function<Minecraft, String> value) {
	}

	private static final Set<Tab> PVP = EnumSet.of(Tab.HUD, Tab.PVP);
	private static final Set<Tab> INFO = EnumSet.of(Tab.HUD);
	private static final List<Line> LINES = new ArrayList<>();
	private static final String[] COLOURS = {"orange", "purple", "aqua", "green", "white"};
	private static final int[] COLOUR_VALUES = {0xFFFF8F1F, 0xFF9B7BFF, 0xFF55FFFF, 0xFF7AC943, 0xFFFFFFFF};
	private static final DateTimeFormatter DATE = DateTimeFormatter.ofPattern("EEE d MMM", Locale.ENGLISH);

	static {
		// item counters (whole inventory, offhand and armor included)
		counter("totems", "Totem Counter", Items.TOTEM_OF_UNDYING, s -> s.is(Items.TOTEM_OF_UNDYING));
		counter("gapples", "Golden Apple Counter", Items.GOLDEN_APPLE, s -> s.is(Items.GOLDEN_APPLE) || s.is(Items.ENCHANTED_GOLDEN_APPLE));
		counter("pearls", "Pearl Counter", Items.ENDER_PEARL, s -> s.is(Items.ENDER_PEARL));
		counter("potions", "Splash Potion Counter", Items.SPLASH_POTION, s -> s.is(Items.SPLASH_POTION));
		counter("blocks", "Block Counter", Items.COBBLESTONE, s -> s.getItem() instanceof BlockItem);
		counter("food", "Food Counter", Items.COOKED_BEEF, s -> s.has(DataComponents.FOOD) && !s.is(Items.GOLDEN_APPLE) && !s.is(Items.ENCHANTED_GOLDEN_APPLE));
		counter("xp-bottles", "XP Bottle Counter", Items.EXPERIENCE_BOTTLE, s -> s.is(Items.EXPERIENCE_BOTTLE));
		counter("crystals", "Crystal Counter", Items.END_CRYSTAL, s -> s.is(Items.END_CRYSTAL));
		counter("obsidian", "Obsidian Counter", Items.OBSIDIAN, s -> s.is(Items.OBSIDIAN));
		counter("cobwebs", "Cobweb Counter", Items.COBWEB, s -> s.is(Items.COBWEB));
		counter("fireworks", "Firework Counter", Items.FIREWORK_ROCKET, s -> s.is(Items.FIREWORK_ROCKET));
		counter("wind-charges", "Wind Charge Counter", Items.WIND_CHARGE, s -> s.is(Items.WIND_CHARGE));
		counter("anchors", "Respawn Anchor Counter", Items.RESPAWN_ANCHOR, s -> s.is(Items.RESPAWN_ANCHOR));
		counter("glowstone", "Glowstone Counter", Items.GLOWSTONE, s -> s.is(Items.GLOWSTONE));
		counter("tnt-carts", "TNT Minecart Counter", Items.TNT_MINECART, s -> s.is(Items.TNT_MINECART));

		// cooldowns
		cooldown("pearl-cooldown", "Pearl Cooldown", Items.ENDER_PEARL);
		cooldown("shield-cooldown", "Shield Cooldown", Items.SHIELD);
		cooldown("wind-cooldown", "Wind Charge Cooldown", Items.WIND_CHARGE);
		cooldown("chorus-cooldown", "Chorus Fruit Cooldown", Items.CHORUS_FRUIT);

		// combat
		line("target", "Target HUD", PVP, Items.PLAYER_HEAD, "player", ShadowHud::target);
		line("attack-charge", "Attack Charge", PVP, Items.DIAMOND_SWORD, "attack", mc -> Math.round(mc.player.getAttackStrengthScale(0f) * 100) + "%");
		line("totem-pops", "Totem Pop Counter", PVP, Items.TOTEM_OF_UNDYING, "player", mc -> PvpTracker.lastPop());
		line("health", "Health", PVP, Items.RED_DYE, "heart", mc -> one(mc.player.getHealth()) + (mc.player.getAbsorptionAmount() > 0 ? " +" + one(mc.player.getAbsorptionAmount()) : "") + " ❤");
		line("hunger", "Hunger", PVP, Items.COOKED_BEEF, "speed", mc -> mc.player.getFoodData().getFoodLevel() + " / 20");
		line("saturation", "Saturation", PVP, Items.GOLDEN_CARROT, "speed", mc -> one(mc.player.getFoodData().getSaturationLevel()));
		line("armor-durability", "Armor Durability", PVP, Items.IRON_CHESTPLATE, "armor", ShadowHud::lowestArmor);
		line("armor-points", "Armor Points", PVP, Items.DIAMOND_CHESTPLATE, "armor",
				mc -> mc.player.getArmorValue() + " (+" + Math.round(mc.player.getAttributeValue(Attributes.ARMOR_TOUGHNESS)) + " toughness)");
		line("held-durability", "Held Item Durability", PVP, Items.NETHERITE_SWORD, "attack", mc -> durability(mc.player.getMainHandItem()));
		line("offhand-durability", "Offhand Durability", PVP, Items.SHIELD, "shield", mc -> durability(mc.player.getOffhandItem()));
		line("elytra", "Elytra Durability", PVP, Items.ELYTRA, "speed", mc -> {
			ItemStack chest = mc.player.getItemBySlot(EquipmentSlot.CHEST);
			return chest.is(Items.ELYTRA) ? durability(chest) : null;
		});
		line("held-count", "Held Item Count", PVP, Items.CHEST, "pack", mc -> {
			ItemStack held = mc.player.getMainHandItem();
			return held.isEmpty() ? null : held.getHoverName().getString() + " ×" + count(mc, s -> s.is(held.getItem()));
		});
		line("bow-charge", "Bow Charge", PVP, Items.BOW, "arrow", mc -> mc.player.isUsingItem() && mc.player.getUseItem().is(Items.BOW)
				? Math.round(BowItem.getPowerForTime(mc.player.getTicksUsingItem()) * 100) + "%" : null);
		line("fall-distance", "Fall Distance", PVP, Items.FEATHER, "speed", mc -> mc.player.fallDistance > 0.5 ? one((float) mc.player.fallDistance) + " m" : null);
		line("deaths", "Death Counter", PVP, Items.SKELETON_SKULL, "player", mc -> String.valueOf(PvpTracker.deaths()));
		line("last-death", "Last Death Location", INFO, Items.WITHER_SKELETON_SKULL, "coords", mc -> PvpTracker.lastDeath());

		// info
		line("facing", "Facing", INFO, Items.COMPASS, "compass", mc -> pretty(mc.player.getDirection().getName()));
		line("rotation", "Yaw & Pitch", INFO, Items.RECOVERY_COMPASS, "compass",
				mc -> one(net.minecraft.util.Mth.wrapDegrees(mc.player.getYRot())) + " / " + one(mc.player.getXRot()));
		line("biome", "Biome", INFO, Items.GRASS_BLOCK, "pack", mc -> mc.level.getBiome(mc.player.blockPosition()).unwrapKey().map(k -> pretty(k.identifier().getPath())).orElse("?"));
		line("light", "Light Level", INFO, Items.TORCH, "light", mc -> {
			BlockPos p = mc.player.blockPosition();
			return mc.level.getMaxLocalRawBrightness(p) + " (block " + mc.level.getBrightness(LightLayer.BLOCK, p) + ")";
		});
		line("day", "Day Counter", INFO, Items.CLOCK, "time", mc -> "Day " + (mc.level.getDayTime() / 24000L + 1));
		line("world-clock", "World Clock", INFO, Items.CLOCK, "clock", mc -> {
			long t = mc.level.getDayTime() % 24000L;
			return String.format(Locale.ROOT, "%02d:%02d", (t / 1000 + 6) % 24, t % 1000 * 60 / 1000);
		});
		line("weather", "Weather", INFO, Items.WATER_BUCKET, "storm", mc -> mc.level.isThundering() ? "Thunder" : mc.level.isRaining() ? "Rain" : "Clear");
		line("dimension", "Dimension", INFO, Items.ENDER_EYE, "pack", mc -> pretty(mc.level.dimension().identifier().getPath()));
		line("nether-coords", "Nether Coordinates", PVP, Items.NETHERRACK, "coords", ShadowHud::netherCoords);
		line("chunk", "Chunk Position", INFO, Items.MAP, "chunk", mc -> {
			BlockPos p = mc.player.blockPosition();
			return mc.player.chunkPosition().x + ", " + mc.player.chunkPosition().z + " (" + (p.getX() & 15) + ", " + (p.getZ() & 15) + ")";
		});
		line("players-online", "Players Online", INFO, Items.PLAYER_HEAD, "server", mc -> mc.getConnection() == null ? null : String.valueOf(mc.getConnection().getOnlinePlayers().size()));
		line("session", "Session Time", INFO, Items.CLOCK, "time", mc -> duration(PvpTracker.sessionMillis()));
		line("date", "Date", INFO, Items.PAPER, "clock", mc -> LocalDate.now().format(DATE));
		line("gamemode", "Game Mode", INFO, Items.GRASS_BLOCK, "player", mc -> mc.gameMode == null ? null : pretty(mc.gameMode.getPlayerMode().getName()));
		line("block-info", "Block Info", INFO, Items.STONE, "outline", mc -> mc.hitResult instanceof BlockHitResult b && b.getType() == HitResult.Type.BLOCK
				? mc.level.getBlockState(b.getBlockPos()).getBlock().getName().getString() : null);
		line("entities", "Mobs Nearby", PVP, Items.ZOMBIE_HEAD, "entity", mc -> {
			int n = 0;
			for (Entity e : mc.level.entitiesForRendering()) {
				if (e instanceof LivingEntity && e != mc.player && e.distanceTo(mc.player) <= 32) n++;
			}
			return String.valueOf(n);
		});

		// more counters
		counter("water-buckets", "Water Bucket Counter", Items.WATER_BUCKET, s -> s.is(Items.WATER_BUCKET));
		counter("lava-buckets", "Lava Bucket Counter", Items.LAVA_BUCKET, s -> s.is(Items.LAVA_BUCKET));
		counter("arrows", "Arrow Counter", Items.ARROW, s -> s.is(Items.ARROW) || s.is(Items.SPECTRAL_ARROW) || s.is(Items.TIPPED_ARROW));
		counter("snowballs", "Snowball Counter", Items.SNOWBALL, s -> s.is(Items.SNOWBALL));
		counter("eggs", "Egg Counter", Items.EGG, s -> s.is(Items.EGG));
		counter("tnt", "TNT Counter", Items.TNT, s -> s.is(Items.TNT));
		counter("flint-steel", "Flint and Steel Counter", Items.FLINT_AND_STEEL, s -> s.is(Items.FLINT_AND_STEEL));
		counter("golden-carrots", "Golden Carrot Counter", Items.GOLDEN_CARROT, s -> s.is(Items.GOLDEN_CARROT));
		counter("steak", "Steak Counter", Items.COOKED_BEEF, s -> s.is(Items.COOKED_BEEF));
		counter("ender-chests", "Ender Chest Counter", Items.ENDER_CHEST, s -> s.is(Items.ENDER_CHEST));
		counter("ladders", "Ladder Counter", Items.LADDER, s -> s.is(Items.LADDER));
		counter("scaffolding", "Scaffolding Counter", Items.SCAFFOLDING, s -> s.is(Items.SCAFFOLDING));
		counter("shulkers", "Shulker Box Counter", Items.SHULKER_BOX, s -> s.getItem() instanceof BlockItem b && b.getBlock() instanceof net.minecraft.world.level.block.ShulkerBoxBlock);
		counter("tridents", "Trident Counter", Items.TRIDENT, s -> s.is(Items.TRIDENT));
		cooldown("goat-horn-cooldown", "Goat Horn Cooldown", Items.GOAT_HORN);

		// armor, piece by piece
		armorPiece("helmet-durability", "Helmet Durability", Items.DIAMOND_HELMET, EquipmentSlot.HEAD);
		armorPiece("chestplate-durability", "Chestplate Durability", Items.DIAMOND_CHESTPLATE, EquipmentSlot.CHEST);
		armorPiece("leggings-durability", "Leggings Durability", Items.DIAMOND_LEGGINGS, EquipmentSlot.LEGS);
		armorPiece("boots-durability", "Boots Durability", Items.DIAMOND_BOOTS, EquipmentSlot.FEET);

		// player and world
		line("xp-progress", "XP Progress", INFO, Items.EXPERIENCE_BOTTLE, "speed",
				mc -> "Level " + mc.player.experienceLevel + " (" + Math.round(mc.player.experienceProgress * 100) + "%, " + Math.round((1 - mc.player.experienceProgress) * mc.player.getXpNeededForNextLevel()) + " to next)");
		line("air", "Air Supply", INFO, Items.PUFFERFISH, "rain", mc -> mc.player.getAirSupply() < mc.player.getMaxAirSupply()
				? Math.max(0, mc.player.getAirSupply() / 30) + " / " + mc.player.getMaxAirSupply() / 30 + " bubbles" : null);
		line("height", "Height Above Ground", PVP, Items.FEATHER, "speed", mc -> {
			BlockPos p = mc.player.blockPosition();
			int ground = mc.level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING, p.getX(), p.getZ());
			return one((float) (mc.player.getY() - ground)) + " m";
		});
		line("looked-at-block", "Looked-at Block Position", INFO, Items.SPYGLASS, "outline", mc -> mc.hitResult instanceof BlockHitResult b && b.getType() == HitResult.Type.BLOCK
				? b.getBlockPos().getX() + " " + b.getBlockPos().getY() + " " + b.getBlockPos().getZ() : null);
		line("render-distance", "Render Distance", INFO, Items.SPYGLASS, "distance", mc -> mc.options.getEffectiveRenderDistance() + " chunks");
		line("vertical-speed", "Vertical Speed", PVP, Items.SLIME_BALL, "speed", mc -> one((float) (mc.player.getDeltaMovement().y * 20)) + " m/s");
		line("fps-range", "FPS Range (10 s)", INFO, Items.REDSTONE, "fps", mc -> Combat.fpsRange());

		// session stats
		line("kills", "Kill Counter", PVP, Items.IRON_SWORD, "attack", mc -> Combat.kills());
		line("kd", "K/D Ratio", PVP, Items.GOLDEN_SWORD, "attack", mc -> Combat.kd());
		line("kill-streak", "Kill Streak", PVP, Items.BLAZE_POWDER, "combo", mc -> Combat.streak());
		line("best-streak", "Best Kill Streak", PVP, Items.NETHER_STAR, "combo", mc -> Combat.bestStreak());
		line("hits", "Hits Landed", PVP, Items.STONE_SWORD, "attack", mc -> Combat.hits());
		line("distance-walked", "Distance Travelled", INFO, Items.LEATHER_BOOTS, "speed", mc -> Combat.distance());
		line("blocks-broken", "Blocks Broken", INFO, Items.IRON_PICKAXE, "outline", mc -> Combat.blocksBroken());
	}

	private static void armorPiece(String id, String name, Item icon, EquipmentSlot slot) {
		line(id, name, PVP, icon, "armor", mc -> {
			ItemStack s = mc.player.getItemBySlot(slot);
			if (s.isEmpty() || !s.isDamageableItem()) return null;
			return Math.round((1f - s.getDamageValue() / (float) s.getMaxDamage()) * 100) + "%";
		});
	}

	private static void line(String id, String name, Set<Tab> tabs, @Nullable Item item, String icon, Function<Minecraft, String> value) {
		LINES.add(new Line(id, name, tabs, item, icon, value));
	}

	private static void counter(String id, String name, Item item, Predicate<ItemStack> match) {
		line(id, name, PVP, item, "pack", mc -> String.valueOf(count(mc, match)));
	}

	private static void cooldown(String id, String name, Item item) {
		ItemStack stack = new ItemStack(item);
		line(id, name, PVP, item, "time", mc -> {
			float left = mc.player.getCooldowns().getCooldownPercent(stack, 0f);
			return left <= 0 ? "Ready" : Math.round(left * 100) + "%";
		});
	}

	// ------------------------------------------------------------------ values

	private static int count(Minecraft mc, Predicate<ItemStack> match) {
		Inventory inv = mc.player.getInventory();
		int n = 0;
		for (int i = 0; i < inv.getContainerSize(); i++) {
			ItemStack s = inv.getItem(i);
			if (!s.isEmpty() && match.test(s)) n += s.getCount();
		}
		return n;
	}

	private static String one(float v) {
		return String.format(Locale.ROOT, "%.1f", v);
	}

	private static String pretty(String id) {
		StringBuilder out = new StringBuilder();
		for (String w : id.split("_")) {
			if (w.isEmpty()) continue;
			if (!out.isEmpty()) out.append(' ');
			out.append(Character.toUpperCase(w.charAt(0))).append(w.substring(1));
		}
		return out.toString();
	}

	private static String duration(long ms) {
		long s = ms / 1000;
		return s >= 3600 ? String.format(Locale.ROOT, "%d:%02d:%02d", s / 3600, s / 60 % 60, s % 60) : String.format(Locale.ROOT, "%d:%02d", s / 60, s % 60);
	}

	@Nullable
	private static String durability(ItemStack s) {
		if (s.isEmpty() || !s.isDamageableItem()) return null;
		int left = s.getMaxDamage() - s.getDamageValue();
		return left + " / " + s.getMaxDamage();
	}

	private static final EquipmentSlot[] ARMOR = {EquipmentSlot.HEAD, EquipmentSlot.CHEST, EquipmentSlot.LEGS, EquipmentSlot.FEET};
	private static final String[] ARMOR_NAMES = {"Helmet", "Chestplate", "Leggings", "Boots"};

	/** The most worn armor piece, e.g. "Boots 23%"; null without damageable armor. */
	@Nullable
	private static String lowestArmor(Minecraft mc) {
		String worst = null;
		float lowest = 2f;
		for (int i = 0; i < ARMOR.length; i++) {
			ItemStack s = mc.player.getItemBySlot(ARMOR[i]);
			if (s.isEmpty() || !s.isDamageableItem()) continue;
			float f = 1f - s.getDamageValue() / (float) s.getMaxDamage();
			if (f < lowest) {
				lowest = f;
				worst = ARMOR_NAMES[i] + " " + Math.round(f * 100) + "%";
			}
		}
		return worst;
	}

	private static float lowestArmorFraction(Minecraft mc) {
		float lowest = 1f;
		for (EquipmentSlot slot : ARMOR) {
			ItemStack s = mc.player.getItemBySlot(slot);
			if (!s.isEmpty() && s.isDamageableItem()) lowest = Math.min(lowest, 1f - s.getDamageValue() / (float) s.getMaxDamage());
		}
		return lowest;
	}

	@Nullable
	private static String target(Minecraft mc) {
		if (!(mc.crosshairPickEntity instanceof LivingEntity e)) return null;
		String s = e.getName().getString() + " " + one(e.getHealth() + e.getAbsorptionAmount()) + "❤ " + one(e.distanceTo(mc.player)) + "m";
		int pops = PvpTracker.pops(e);
		return pops > 0 ? s + " (" + pops + " pops)" : s;
	}

	@Nullable
	private static String netherCoords(Minecraft mc) {
		BlockPos p = mc.player.blockPosition();
		if (mc.level.dimension() == Level.NETHER) return "Overworld " + p.getX() * 8 + ", " + p.getZ() * 8;
		if (mc.level.dimension() == Level.OVERWORLD) return "Nether " + Math.floorDiv(p.getX(), 8) + ", " + Math.floorDiv(p.getZ(), 8);
		return null;
	}

	// ------------------------------------------------------------------ settings

	private static JsonObject settings() {
		JsonObject o = MenuConfig.object("hud");
		return o == null ? new JsonObject() : o;
	}

	private static void save(JsonObject o) {
		MenuConfig.putObject("hud", o);
	}

	private static Set<String> enabled() {
		Set<String> out = new LinkedHashSet<>();
		JsonObject o = settings();
		if (o.has("lines") && o.get("lines").isJsonArray()) {
			for (JsonElement e : o.getAsJsonArray("lines")) out.add(e.getAsString());
		}
		return out;
	}

	static boolean isOn(String id) {
		return enabled().contains(id);
	}

	private static void setOn(String id, boolean on) {
		Set<String> ids = enabled();
		if (on) ids.add(id);
		else ids.remove(id);
		JsonObject o = settings().deepCopy();
		JsonArray a = new JsonArray();
		ids.forEach(a::add);
		o.add("lines", a);
		save(o);
	}

	private static String str(String key, String def) {
		JsonObject o = settings();
		return o.has(key) ? o.get(key).getAsString() : def;
	}

	private static int num(String key, int def) {
		JsonObject o = settings();
		return o.has(key) ? o.get(key).getAsInt() : def;
	}

	private static boolean flag(String key, boolean def) {
		JsonObject o = settings();
		return o.has(key) ? o.get(key).getAsBoolean() : def;
	}

	private static void put(String key, Object value) {
		JsonObject o = settings().deepCopy();
		if (value instanceof Number n) o.addProperty(key, n);
		else if (value instanceof Boolean b) o.addProperty(key, b);
		else o.addProperty(key, String.valueOf(value));
		save(o);
	}

	private static int indexOf(String[] values, String v) {
		for (int i = 0; i < values.length; i++) if (values[i].equals(v)) return i;
		return 0;
	}

	// ------------------------------------------------------------------ menu cards

	public static void addTo(List<Module> out) {
		for (Line l : LINES) {
			Module m = new Module("shud:" + l.id(), l.name(), l.icon(), l.tabs(), () -> isOn(l.id()), v -> setOn(l.id(), v), null, null);
			out.add(l.item() != null ? m.item(l.item()) : m);
		}
		// warnings
		out.add(new Module("shud:low-health", "Low Health Warning", "heart", EnumSet.of(Tab.PVP, Tab.HUD), () -> isOn("low-health"), v -> setOn("low-health", v), null, null));
		out.add(new Module("shud:low-armor", "Low Armor Warning", "armor", EnumSet.of(Tab.PVP, Tab.HUD), () -> isOn("low-armor"), v -> setOn("low-armor", v), null, null).item(Items.CHAINMAIL_CHESTPLATE));
		String[][] alerts = {
				{"pearl-ready", "Pearl Ready Alert"}, {"shield-disabled", "Shield Disabled Alert"}, {"totem-popped", "Totem Popped Alert"},
				{"low-durability", "Low Durability Warning"}, {"low-food", "Low Food Warning"}, {"fall-warning", "Fall Warning"},
				{"low-arrows", "Low Arrows Warning"}, {"void-warning", "Void Warning"}, {"armor-break", "Armor Break Alert"},
		};
		Item[] alertIcons = {Items.ENDER_PEARL, Items.SHIELD, Items.TOTEM_OF_UNDYING, Items.ANVIL, Items.ROTTEN_FLESH, Items.FEATHER, Items.ARROW, Items.END_PORTAL_FRAME, Items.LEATHER_CHESTPLATE};
		for (int i = 0; i < alerts.length; i++) {
			String id = alerts[i][0];
			out.add(new Module("shud:" + id, alerts[i][1], "player", EnumSet.of(Tab.PVP, Tab.HUD), () -> isOn(id), v -> setOn(id, v), null, null).item(alertIcons[i]));
		}
		out.add(new Module("shud:offhand-totem", "Offhand Totem Alert", "player", EnumSet.of(Tab.PVP, Tab.HUD), () -> isOn("offhand-totem"), v -> setOn("offhand-totem", v), null, null).item(Items.TOTEM_OF_UNDYING));
		// style of the Shadow HUD stack (where it sits and its size are set in the HUD editor)
		Set<Tab> style = EnumSet.of(Tab.HUD);
		out.add(Module.cycle("shud-colour", "Shadow HUD Colour", "hitcolor", style, () -> pretty(str("label", "orange")),
				dir -> put("label", COLOURS[Math.floorMod(indexOf(COLOURS, str("label", "orange")) + dir, COLOURS.length)]), null));
		out.add(new Module("shud-background", "Shadow HUD Background", "grid", style, () -> flag("background", true), v -> put("background", v), null, null));
		out.add(new Module("shud-shadow", "Shadow HUD Text Shadow", "nametag", style, () -> flag("shadow", true), v -> put("shadow", v), null, null));
	}

	// ------------------------------------------------------------------ rendering

	/** Waypoints bar and the centre-screen warnings (the line stack is a movable HUD module). */
	public static void render(GuiGraphics g, DeltaTracker delta) {
		Minecraft mc = Minecraft.getInstance();
		LocalPlayer player = mc.player;
		if (player == null || mc.level == null || mc.options.hideGui || mc.debugEntries.isOverlayVisible()) return;
		if (mc.screen instanceof com.shadowclient.menu.ui.HudEditorScreen) return;
		Waypoints.render(g, mc);
		Set<String> on = enabled();
		if (on.isEmpty()) return;
		warnings(g, mc, on);
	}

	/** The lines switched on, with their values now ({@code preview}: samples when none show). */
	private record Shown(List<Line> lines, List<String> labels, List<String> values, int width, int height) {
	}

	private static final String[][] SAMPLE = {{"totems", "3"}, {"pearls", "16"}, {"health", "20.0 ❤"}, {"facing", "North"}};

	private static Shown measure(boolean preview) {
		Minecraft mc = Minecraft.getInstance();
		Set<String> on = enabled();
		List<Line> shown = new ArrayList<>();
		List<String> values = new ArrayList<>();
		if (mc.player != null && mc.level != null) {
			for (Line l : LINES) {
				if (!on.contains(l.id())) continue;
				String v;
				try {
					v = l.value().apply(mc);
				} catch (RuntimeException e) {
					v = null; // never take the HUD down with one line
				}
				if (v != null) {
					shown.add(l);
					values.add(v);
				}
			}
		}
		if (shown.isEmpty() && preview) {
			for (String[] s : SAMPLE) {
				for (Line l : LINES) {
					if (l.id().equals(s[0])) {
						shown.add(l);
						values.add(s[1]);
					}
				}
			}
		}
		var font = mc.font;
		int width = 0;
		List<String> labels = new ArrayList<>();
		for (int i = 0; i < shown.size(); i++) {
			Line l = shown.get(i);
			String name = l.item() != null && l.icon().equals("pack") && l.name().endsWith("Counter") ? "" : shortName(l.name()) + ": ";
			labels.add(name);
			width = Math.max(width, (l.item() != null ? 12 : 0) + font.width(name) + font.width(values.get(i)));
		}
		return new Shown(shown, labels, values, width + 6, shown.size() * 11 + 2);
	}

	public static boolean hasLines() {
		return !measure(false).lines().isEmpty();
	}

	public static int stackWidth(boolean preview) {
		return Math.max(20, measure(preview).width());
	}

	public static int stackHeight(boolean preview) {
		return Math.max(13, measure(preview).height());
	}

	/** Draw the line stack at (0, 0) (the HUD module has placed and scaled the pose). */
	public static void drawStack(GuiGraphics g, boolean preview) {
		Shown s = measure(preview);
		var font = Minecraft.getInstance().font;
		int label = COLOUR_VALUES[indexOf(COLOURS, str("label", "orange"))];
		boolean bg = flag("background", true), shadow = flag("shadow", true);
		if (bg) com.shadowclient.menu.ui.Draw.round(g, 0, 0, s.width(), s.height(), 3, 0x70000000);
		for (int i = 0; i < s.lines().size(); i++) {
			Line l = s.lines().get(i);
			int y = 2 + i * 11;
			int x = 3;
			if (l.item() != null) {
				g.pose().pushMatrix();
				g.pose().translate(x - 1, y - 1);
				g.pose().scale(0.625f, 0.625f);
				g.renderFakeItem(new ItemStack(l.item()), 0, 0);
				g.pose().popMatrix();
				x += 12;
			}
			g.drawString(font, s.labels().get(i), x, y, label, shadow);
			g.drawString(font, s.values().get(i), x + font.width(s.labels().get(i)), y, 0xFFFFFFFF, shadow);
		}
	}

	/** The corner the 1.x stack was in, as the movable module's default corner. */
	public static com.shadowclient.menu.hud.HudElement.Anchor legacyAnchor() {
		return switch (str("corner", "top-right")) {
			case "top-left" -> com.shadowclient.menu.hud.HudElement.Anchor.TOP_LEFT;
			case "bottom-right" -> com.shadowclient.menu.hud.HudElement.Anchor.BOTTOM_RIGHT;
			case "bottom-left" -> com.shadowclient.menu.hud.HudElement.Anchor.NONE;
			default -> com.shadowclient.menu.hud.HudElement.Anchor.TOP_RIGHT;
		};
	}

	/** Where the stack sat before it could be moved (1.x "Shadow HUD Position" / "Scale"). */
	public static double[] legacyPlacement() {
		String corner = str("corner", "top-right");
		return new double[] {corner.endsWith("left") ? 0.004 : 0.996, corner.startsWith("top") ? 0.008 : 0.992, num("scale", 100) / 100.0};
	}

	private static String shortName(String name) {
		return name.replace(" Counter", "").replace(" Durability", " dur.").replace("Shadow HUD ", "");
	}

	private static void warnings(GuiGraphics g, Minecraft mc, Set<String> on) {
		LocalPlayer p = mc.player;
		int w = g.guiWidth(), h = g.guiHeight();
		if (on.contains("low-health") && p.getHealth() <= p.getMaxHealth() * 0.3f && !p.isDeadOrDying()) {
			// pulsing red edges
			float pulse = 0.55f + 0.45f * (float) Math.sin(System.currentTimeMillis() / 180.0);
			for (int i = 0; i < 24; i++) {
				int a = Math.round((1 - i / 24f) * 110 * pulse);
				int c = (a << 24) | 0xD01010;
				g.fill(i, i, i + 1, h - i, c);
				g.fill(w - i - 1, i, w - i, h - i, c);
				g.fill(i + 1, i, w - i - 1, i + 1, c);
				g.fill(i + 1, h - i - 1, w - i - 1, h - i, c);
			}
		}
		int y = h / 2 + 18;
		if (on.contains("low-armor") && lowestArmorFraction(mc) < 0.15f) {
			String s = "Low armor: " + lowestArmor(mc);
			g.drawCenteredString(mc.font, s, w / 2, y, 0xFFFF5555);
			y += 11;
		}
		if (on.contains("offhand-totem") && !p.getOffhandItem().is(Items.TOTEM_OF_UNDYING) && count(mc, s -> s.is(Items.TOTEM_OF_UNDYING)) > 0) {
			g.drawCenteredString(mc.font, "Totem not in offhand", w / 2, y, 0xFFFFD24A);
			y += 11;
		}
		Combat.renderAlerts(g, mc, on, y);
	}
}
