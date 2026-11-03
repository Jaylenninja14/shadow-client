package com.shadowclient.menu.hud;

import com.shadowclient.menu.mixin.BossHealthOverlayAccessor;
import com.shadowclient.menu.module.MenuConfig;
import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.module.ShadowHud;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ColorSetting;
import com.shadowclient.menu.setting.NumberSetting;
import com.shadowclient.menu.ui.Draw;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.EnumSet;
import java.util.List;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.inventory.InventoryScreen;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.numbers.NumberFormat;
import net.minecraft.network.chat.numbers.StyledFormat;
import net.minecraft.world.scores.DisplaySlot;
import net.minecraft.world.scores.Objective;
import net.minecraft.world.scores.PlayerScoreEntry;
import net.minecraft.world.scores.PlayerTeam;
import net.minecraft.world.scores.Scoreboard;
import org.jetbrains.annotations.Nullable;

/** HUD modules that move things Minecraft or Shadow Menu already draw: scoreboard, boss bar, player, Shadow HUD. */
final class GameElements {
	private GameElements() {
	}

	static void addTo(List<HudElement> out) {
		out.add(new ShadowStack());
		out.add(new ScoreboardElement());
		out.add(new BossBar());
		out.add(new PlayerModel());
	}

	/** Shadow HUD's line stack (item counters, cooldowns, target, …); its lines are cards of their own. */
	static final class ShadowStack extends HudElement {
		ShadowStack() {
			super("shadow-hud", "Shadow HUD", "grid", EnumSet.of(Tab.HUD, Tab.PVP), true, ShadowHud.legacyPlacement()[0], ShadowHud.legacyPlacement()[1]);
			describe("The stack of Shadow HUD lines you switched on (totems, pearls, target, …). Its colour, background and shadow are cards in the HUD tab.");
			anchor(ShadowHud.legacyAnchor());
			// 1.x kept a corner and a size; carry the size over once
			if (MenuConfig.module(settings.id) == null && ShadowHud.legacyPlacement()[2] != 1) scale.set(ShadowHud.legacyPlacement()[2]);
		}

		@Override
		public boolean hasContent() {
			return ShadowHud.hasLines();
		}

		@Override
		public int width(boolean preview) {
			return ShadowHud.stackWidth(preview);
		}

		@Override
		public int height(boolean preview) {
			return ShadowHud.stackHeight(preview);
		}

		@Override
		public void render(GuiGraphics g, boolean preview) {
			ShadowHud.drawStack(g, preview);
		}
	}

	/** The sidebar scoreboard, drawn by Shadow Menu so it can move, shrink and lose its red numbers. */
	static final class ScoreboardElement extends HudElement {
		final BoolSetting numbers = add(new BoolSetting("numbers", "Show score numbers", false));
		final BoolSetting shadow = add(new BoolSetting("shadow", "Text shadow", false).group("Style"));
		final ColorSetting background = add(new ColorSetting("background", "Background colour", 0x4C000000, true).group("Style"));
		final ColorSetting titleBackground = add(new ColorSetting("title-background", "Title background", 0x66000000, true).group("Style"));
		final ColorSetting numberColor = add(new ColorSetting("number-color", "Number colour", 0xFFFF5555, false).group("Style").showIf(numbers::on));
		private static final Comparator<PlayerScoreEntry> ORDER = Comparator.comparing(PlayerScoreEntry::value).reversed()
				.thenComparing(PlayerScoreEntry::owner, String.CASE_INSENSITIVE_ORDER);

		private record Row(Component name, Component score) {
		}

		ScoreboardElement() {
			super("scoreboard", "Scoreboard", "scoreboard", EnumSet.of(Tab.HUD, Tab.HYPIXEL), false, 0.996, 0.5);
			describe("The server's sidebar scoreboard, movable and without the red numbers.");
		}

		@Nullable
		private Objective objective() {
			if (mc.level == null || mc.player == null) return null;
			Scoreboard sb = mc.level.getScoreboard();
			PlayerTeam team = sb.getPlayersTeam(mc.player.getScoreboardName());
			if (team != null) {
				DisplaySlot slot = DisplaySlot.teamColorToSlot(team.getColor());
				if (slot != null && sb.getDisplayObjective(slot) != null) return sb.getDisplayObjective(slot);
			}
			return sb.getDisplayObjective(DisplaySlot.SIDEBAR);
		}

		private Component title(boolean preview) {
			Objective o = objective();
			return o != null ? o.getDisplayName() : Component.literal("SHADOW CLIENT");
		}

		private List<Row> rows(boolean preview) {
			List<Row> out = new ArrayList<>();
			Objective o = objective();
			if (o == null) {
				if (preview) {
					String[] sample = {"", "Kills: 12", "Deaths: 3", "", "play.example.net"};
					for (int i = 0; i < sample.length; i++) out.add(new Row(Component.literal(sample[i]), Component.literal(String.valueOf(sample.length - i))));
				}
				return out;
			}
			Scoreboard sb = o.getScoreboard();
			NumberFormat format = o.numberFormatOrDefault(StyledFormat.SIDEBAR_DEFAULT);
			sb.listPlayerScores(o).stream().filter(e -> !e.isHidden()).sorted(ORDER).limit(15).forEach(e -> {
				PlayerTeam team = sb.getPlayersTeam(e.owner());
				out.add(new Row(PlayerTeam.formatNameForTeam(team, e.ownerName()), e.formatValue(format)));
			});
			return out;
		}

		@Override
		public boolean hasContent() {
			return objective() != null;
		}

		@Override
		public int width(boolean preview) {
			int w = font().width(title(preview));
			for (Row r : rows(preview)) w = Math.max(w, font().width(r.name()) + (numbers.on() ? font().width(": ") + font().width(r.score()) : 0));
			return w + 4;
		}

		@Override
		public int height(boolean preview) {
			return (rows(preview).size() + 1) * 9 + 2;
		}

		@Override
		public void render(GuiGraphics g, boolean preview) {
			int w = width(preview);
			List<Row> rows = rows(preview);
			Component title = title(preview);
			g.fill(0, 0, w, 10, titleBackground.argb());
			g.fill(0, 10, w, height(preview), background.argb());
			g.drawString(font(), title, (w - font().width(title)) / 2, 1, 0xFFFFFFFF, shadow.on());
			int y = 11;
			for (Row r : rows) {
				g.drawString(font(), r.name(), 2, y, 0xFFFFFFFF, shadow.on());
				if (numbers.on()) g.drawString(font(), r.score(), w - 2 - font().width(r.score()), y, numberColor.argb(), shadow.on());
				y += 9;
			}
		}
	}

	/** Minecraft's boss bars, moved and scaled as a block (they're still drawn by Minecraft). */
	static final class BossBar extends HudElement {
		BossBar() {
			super("boss-bar", "Boss Bar", "bossbar", EnumSet.of(Tab.HUD), false, 0.5, 0.0);
			describe("Moves Minecraft's boss bars (Ender Dragon, Wither, server bars).");
		}

		int bars() {
			return ((BossHealthOverlayAccessor) mc.gui.getBossOverlay()).shadowmenu$events().size();
		}

		@Override
		public boolean hasContent() {
			return bars() > 0;
		}

		@Override
		public int width(boolean preview) {
			return 182;
		}

		@Override
		public int height(boolean preview) {
			return Math.max(1, Math.min(bars(), 4)) * 19 - 4;
		}

		@Override
		public void render(GuiGraphics g, boolean preview) {
			// the real bars are drawn by Minecraft (see HudManager); the editor shows where they go
			if (!preview) return;
			for (int i = 0; i < Math.max(1, Math.min(bars(), 4)); i++) {
				int y = i * 19;
				String name = "Boss Bar";
				g.drawString(font(), name, (182 - font().width(name)) / 2, y, 0xFFFFFFFF, true);
				g.fill(0, y + 9, 182, y + 14, 0xFF3A1240);
				g.fill(0, y + 9, 120, y + 14, 0xFFE83CD6);
			}
		}

		/** Where Minecraft would put the first bar's name: centred, 3 px from the top. */
		static int vanillaLeft(int screenW) {
			return screenW / 2 - 91;
		}

		static final int VANILLA_TOP = 3;
	}

	/** You, as a small 3D model that turns with you. */
	static final class PlayerModel extends HudElement {
		final NumberSetting turn = add(new NumberSetting("turn", "Turn", 15, -30, 30, 1).suffix("°"));

		PlayerModel() {
			super("player-model", "Player Model", "player", EnumSet.of(Tab.HUD), false, 0.996, 0.97);
			describe("A small model of your player.");
			anchor(Anchor.BOTTOM_RIGHT);
		}

		@Override
		public boolean hasContent() {
			return mc.player != null;
		}

		@Override
		public int width(boolean preview) {
			return 40;
		}

		@Override
		public int height(boolean preview) {
			return 64;
		}

		@Override
		public void render(GuiGraphics g, boolean preview) {
		}

		@Override
		public boolean drawsAbsolute() {
			return true;
		}

		@Override
		public void renderAbsolute(GuiGraphics g, int left, int top, float s, boolean preview) {
			if (mc.player == null) {
				if (preview) Draw.roundOutline(g, left, top, 40 * s, 64 * s, 3, 0x80FFFFFF);
				return;
			}
			int w = Math.round(40 * s), h = Math.round(64 * s);
			// the helper turns the model atan((centre - mouse) / 40) * 20 degrees towards a mouse
			// position; aim it at the point that gives the chosen turn
			float cx = left + w / 2f, cy = top + h / 2f;
			float mouseX = cx - (float) Math.tan(turn.get() / 20.0) * 40;
			InventoryScreen.renderEntityInInventoryFollowsMouse(g, left, top, left + w, top + h, Math.round(28 * s), 0.0625f, mouseX, cy - 10 * s, mc.player);
		}
	}
}
