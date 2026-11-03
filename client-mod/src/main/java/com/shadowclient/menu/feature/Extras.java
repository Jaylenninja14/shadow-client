package com.shadowclient.menu.feature;

import com.mojang.authlib.GameProfile;
import com.mojang.blaze3d.platform.NativeImage;
import com.shadowclient.menu.ShadowMenu;
import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.ui.Draw;
import java.util.EnumSet;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.multiplayer.PlayerInfo;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.core.component.DataComponents;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.contents.objects.PlayerSprite;
import net.minecraft.resources.Identifier;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.food.FoodData;
import net.minecraft.world.food.FoodProperties;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.component.ResolvableProfile;
import org.jetbrains.annotations.Nullable;

/**
 * Shadow Menu's own versions of what the launcher used to install as extra mods: chat heads, a
 * saturation display on the hunger bar, ping numbers in the tab list and screenshots copied to the
 * clipboard.
 */
public final class Extras {
	private Extras() {
	}

	/** The sender's head in front of chat messages (Minecraft draws it as part of the text). */
	public static final class ChatHeads extends Feature {
		public static final ChatHeads I = new ChatHeads();
		final BoolSetting hat = add(new BoolSetting("hat", "Show the hat layer", true));
		final BoolSetting system = add(new BoolSetting("system", "Also on server messages that start with a player's name", true));
		@Nullable private static GameProfile pending;
		private static long pendingAt;

		private ChatHeads() {
			super("chat-heads", "Chat Heads", "chat", EnumSet.of(Tab.GAME, Tab.HYPIXEL), true);
			describe("Shows who sent a chat message with their head in front of it.");
		}

		/** A signed player chat message is about to be added; its sender comes with it. */
		public static void sender(@Nullable GameProfile profile) {
			pending = profile;
			pendingAt = System.currentTimeMillis();
		}

		public static Component decorate(Component message) {
			ChatHeads c = I;
			if (!c.on() || message == null) return message;
			GameProfile who = null;
			if (pending != null && System.currentTimeMillis() - pendingAt < 1000) who = pending;
			pending = null;
			if (who == null && c.system.on()) who = nameAtStart(message.getString());
			if (who == null) return message;
			return Component.empty().append(Component.object(new PlayerSprite(ResolvableProfile.createResolved(who), c.hat.on()))).append(" ").append(message);
		}

		/** A player in the tab list whose name opens the message ("[VIP] Steve: hi", "<Steve> hi", "Steve > hi"). */
		@Nullable
		private static GameProfile nameAtStart(String text) {
			Minecraft mc = Minecraft.getInstance();
			if (mc.getConnection() == null) return null;
			int colon = text.indexOf(':'), angle = text.indexOf('>');
			int end = colon < 0 ? angle : angle < 0 ? colon : Math.min(colon, angle);
			if (end < 0 || end > 48) return null;
			String head = text.substring(0, end);
			GameProfile best = null;
			int bestAt = Integer.MAX_VALUE;
			for (PlayerInfo info : mc.getConnection().getOnlinePlayers()) {
				String name = info.getProfile().name();
				int at = head.indexOf(name);
				// a whole word: not part of a longer name
				if (at < 0 || at > 0 && Character.isLetterOrDigit(head.charAt(at - 1)) || at + name.length() < head.length() && isNameChar(head.charAt(at + name.length()))) continue;
				if (at < bestAt) {
					best = info.getProfile();
					bestAt = at;
				}
			}
			return best;
		}

		private static boolean isNameChar(char ch) {
			return Character.isLetterOrDigit(ch) || ch == '_';
		}
	}

	/** Saturation under the hunger bar, and what the food you hold would fill. */
	public static final class Saturation extends Feature {
		public static final Saturation I = new Saturation();
		final BoolSetting bar = add(new BoolSetting("bar", "Show saturation under the hunger bar", true));
		final BoolSetting preview = add(new BoolSetting("preview", "Preview what held food restores", true));
		private static final Identifier FULL = Identifier.withDefaultNamespace("hud/food_full");
		private static final Identifier HALF = Identifier.withDefaultNamespace("hud/food_half");

		private Saturation() {
			super("saturation", "Saturation Display", "speed", EnumSet.of(Tab.HUD, Tab.PVP), true);
			describe("Shows your hidden saturation under the hunger bar and flashes how much the food you hold would restore.");
		}

		/** Minecraft just drew the hunger bar ending at {@code right}, at height {@code y}. */
		public static void drawn(GuiGraphics g, Player player, int y, int right) {
			Saturation s = I;
			if (!s.on()) return;
			FoodData food = player.getFoodData();
			if (s.preview.on()) {
				ItemStack held = player.getMainHandItem();
				FoodProperties fp = held.get(DataComponents.FOOD);
				if (fp == null) {
					held = player.getOffhandItem();
					fp = held.get(DataComponents.FOOD);
				}
				if (fp != null) {
					int now = food.getFoodLevel(), after = Math.min(20, now + fp.nutrition());
					float pulse = 0.35f + 0.35f * (float) Math.sin(System.currentTimeMillis() / 160.0);
					for (int i = now; i < after; i++) {
						int shank = i / 2;
						int x = right - shank * 8 - 9;
						// a half shank where the bar would end on an odd number
						boolean half = i == after - 1 && after % 2 == 1;
						if (i % 2 == 0) g.blitSprite(RenderPipelines.GUI_TEXTURED, half ? HALF : FULL, x, y, 9, 9, Draw.fade(0xFFFFFFFF, pulse));
					}
				}
			}
			if (s.bar.on()) {
				float sat = food.getSaturationLevel();
				for (int i = 0; i < 10; i++) {
					float part = Math.max(0, Math.min(2, sat - i * 2)) / 2f;
					if (part <= 0) break;
					int x = right - i * 8 - 9;
					g.fill(x + 1, y + 9, x + 1 + Math.max(1, Math.round(7 * part)), y + 10, 0xFFFFD34A);
				}
			}
		}
	}

	/** Ping as a coloured number in the tab list instead of bars. */
	public static final class TabPing extends Feature {
		public static final TabPing I = new TabPing();

		private TabPing() {
			super("tab-ping", "Ping in Tab", "ping", EnumSet.of(Tab.HUD, Tab.PVP, Tab.HYPIXEL), true);
			describe("Shows every player's ping as a number in the tab list.");
		}

		/** Extra room the ping column needs (the bars take 13 pixels). */
		public static int extraWidth() {
			return I.on() ? 14 : 0;
		}

		/** Draw "42ms" right-aligned where the bars would be; false lets Minecraft draw its bars. */
		public static boolean draw(GuiGraphics g, int width, int x, int y, PlayerInfo info) {
			if (!I.on()) return false;
			int ms = info.getLatency();
			String text = ms < 0 ? "?" : ms + "ms";
			int color = ms < 0 ? 0xFFAAAAAA : ms < 80 ? 0xFF55FF55 : ms < 150 ? 0xFFA0E04A : ms < 300 ? 0xFFFFD34A : ms < 600 ? 0xFFFF8F1F : 0xFFFF5555;
			var font = Minecraft.getInstance().font;
			Draw.scaledText(g, font, text, x + width - 1 - font.width(text) * 0.75f, y + 1.5f, 0.75f, color, true);
			return true;
		}
	}

	/** F2 screenshots are also put on the clipboard, ready to paste (Windows). */
	public static final class ScreenshotCopy extends Feature {
		public static final ScreenshotCopy I = new ScreenshotCopy();

		private ScreenshotCopy() {
			super("screenshot-copy", "Copy Screenshots", "camera", EnumSet.of(Tab.GAME), false);
			describe("Screenshots you take with F2 are also copied to the clipboard (Windows). Takes effect after a restart the first time.");
		}

		/**
		 * Minecraft runs without a desktop toolkit; the clipboard needs one. Switched back on at start
		 * (Windows only, and only when this module is on) before anything else could use it.
		 */
		public static void prepare() {
			if (I.on() && net.minecraft.util.Util.getPlatform() == net.minecraft.util.Util.OS.WINDOWS) System.setProperty("java.awt.headless", "false");
		}

		public static boolean wanted() {
			return I.on();
		}

		/** Copy the pixels now (the image is closed after saving), hand them to the clipboard on another thread. */
		public static void copy(NativeImage img) {
			int w = img.getWidth(), h = img.getHeight();
			int[] px = new int[w * h];
			for (int y = 0; y < h; y++) for (int x = 0; x < w; x++) px[y * w + x] = img.getPixel(x, y);
			Thread t = new Thread(() -> {
				try {
					java.awt.image.BufferedImage out = new java.awt.image.BufferedImage(w, h, java.awt.image.BufferedImage.TYPE_INT_RGB);
					out.setRGB(0, 0, w, h, px, 0, w);
					java.awt.Toolkit.getDefaultToolkit().getSystemClipboard().setContents(new ImageTransfer(out), null);
				} catch (Throwable e) { // headless or no clipboard: just don't copy
					ShadowMenu.LOG.warn("Couldn't copy the screenshot to the clipboard: {}", e.toString());
				}
			}, "Shadow screenshot copy");
			t.setDaemon(true);
			t.start();
		}

		private record ImageTransfer(java.awt.Image image) implements java.awt.datatransfer.Transferable {
			@Override
			public java.awt.datatransfer.DataFlavor[] getTransferDataFlavors() {
				return new java.awt.datatransfer.DataFlavor[] {java.awt.datatransfer.DataFlavor.imageFlavor};
			}

			@Override
			public boolean isDataFlavorSupported(java.awt.datatransfer.DataFlavor flavor) {
				return java.awt.datatransfer.DataFlavor.imageFlavor.equals(flavor);
			}

			@Override
			public Object getTransferData(java.awt.datatransfer.DataFlavor flavor) throws java.awt.datatransfer.UnsupportedFlavorException {
				if (!isDataFlavorSupported(flavor)) throw new java.awt.datatransfer.UnsupportedFlavorException(flavor);
				return image;
			}
		}
	}
}
