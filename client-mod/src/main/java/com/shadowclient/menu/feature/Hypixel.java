package com.shadowclient.menu.feature;

import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.NumberSetting;
import com.shadowclient.menu.setting.TextSetting;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.Iterator;
import java.util.List;
import java.util.Locale;
import java.util.Optional;
import java.util.Set;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.MutableComponent;
import net.minecraft.network.chat.Style;
import net.minecraft.world.item.Items;

/**
 * Hypixel helpers: AutoGG / AutoGLHF (say gg at the end and glhf at the start of a game), AutoTip
 * and AutoBoop, plus Nick Hider (works anywhere). Messages go out with a delay, never more than one
 * per game, so they look like you typed them.
 */
public final class Hypixel {
	private Hypixel() {
	}

	static final Set<Tab> HYP = EnumSet.of(Tab.HYPIXEL);
	private record Pending(long at, String message) {
	}

	private static final List<Pending> QUEUE = new ArrayList<>();
	private static long lastGg, lastGlhf, lastTip;
	// what Hypixel prints when a game ends (Bed Wars, SkyWars, Duels, Murder Mystery, …)
	private static final String[] END = {"1st Killer - ", "1st Place - ", "Winner: ", " - Damage Dealt - ", "Winning Team -", "1st - ",
			"Winners: ", "Winner - ", "Reward Summary", "won the game", "WINNER!", "Your Overall Winstreak:"};
	private static final Pattern FRIEND_JOIN = Pattern.compile("^Friend > (\\w{2,16}) joined\\.$");

	static boolean onHypixel(Minecraft mc) {
		return mc.getCurrentServer() != null && mc.getCurrentServer().ip.toLowerCase(Locale.ROOT).contains("hypixel");
	}

	static void send(String message, double delaySeconds) {
		QUEUE.add(new Pending(System.currentTimeMillis() + Math.round(delaySeconds * 1000), message));
	}

	static void tick(Minecraft mc) {
		long now = System.currentTimeMillis();
		for (Iterator<Pending> it = QUEUE.iterator(); it.hasNext(); ) {
			Pending p = it.next();
			if (p.at() > now) continue;
			it.remove();
			if (mc.player == null) continue;
			String m = p.message().trim();
			if (m.isEmpty()) continue;
			if (m.startsWith("/")) mc.player.connection.sendCommand(m.substring(1));
			else mc.player.connection.sendChat(m);
		}
		if (AutoTip.I.on() && mc.player != null && onHypixel(mc) && now - lastTip > AutoTip.I.minutes.get() * 60_000) {
			lastTip = now;
			send("/tip all", 5);
		}
	}

	/** A chat or system line arrived. */
	public static void message(Component message) {
		Minecraft mc = Minecraft.getInstance();
		if (mc.player == null) return;
		String text = message.getString();
		long now = System.currentTimeMillis();
		boolean hypixel = onHypixel(mc);
		if (AutoGG.I.on() && (hypixel || !AutoGG.I.hypixelOnly.on()) && now - lastGg > 10_000) {
			for (String end : END) {
				if (text.contains(end)) {
					lastGg = now;
					send(AutoGG.I.message.get(), AutoGG.I.delay.get());
					if (AutoGG.I.gf.on()) send("gf", AutoGG.I.delay.get() + 1.2);
					break;
				}
			}
		}
		if (AutoGLHF.I.on() && hypixel && now - lastGlhf > 30_000 && text.contains("The game starts in 1 second!")) {
			lastGlhf = now;
			send(AutoGLHF.I.message.get(), 1.5);
		}
		if (AutoBoop.I.on() && hypixel) {
			Matcher m = FRIEND_JOIN.matcher(text.trim());
			if (m.matches()) send("/boop " + m.group(1), 1.5);
		}
	}

	public static final class AutoGG extends Feature {
		public static final AutoGG I = new AutoGG();
		final TextSetting message = add(new TextSetting("message", "Message", "gg", 64));
		final NumberSetting delay = add(new NumberSetting("delay", "Delay", 1, 0, 5, 0.5).suffix(" s"));
		final BoolSetting gf = add(new BoolSetting("gf", "Also say gf (AutoGF)", false));
		final BoolSetting hypixelOnly = add(new BoolSetting("hypixel-only", "Only on Hypixel", true));

		private AutoGG() {
			super("autogg", "AutoGG", "gg", HYP, false);
			describe("Says gg when a game ends.");
		}
	}

	public static final class AutoGLHF extends Feature {
		public static final AutoGLHF I = new AutoGLHF();
		final TextSetting message = add(new TextSetting("message", "Message", "glhf", 64));

		private AutoGLHF() {
			super("autoglhf", "AutoGLHF", "gg", HYP, false);
			describe("Says glhf when a Hypixel game starts.");
		}
	}

	public static final class AutoTip extends Feature {
		public static final AutoTip I = new AutoTip();
		final NumberSetting minutes = add(new NumberSetting("minutes", "Every", 20, 15, 60, 5).suffix(" min"));

		private AutoTip() {
			super("autotip", "AutoTip", "tip", HYP, false);
			describe("Runs /tip all on Hypixel now and then, for the network's tip rewards.");
		}
	}

	public static final class AutoBoop extends Feature {
		public static final AutoBoop I = new AutoBoop();

		private AutoBoop() {
			super("autoboop", "AutoBoop", "boop", HYP, false);
			describe("Boops your Hypixel friends when they join.");
		}
	}

	/** Nick Hider: your name (and others', if you like) replaced in chat, the tab list and nametags; skins hidden. */
	public static final class NickHider extends Feature {
		public static final NickHider I = new NickHider();
		final TextSetting nick = add(new TextSetting("nick", "Your name shows as", "You", 16));
		final BoolSetting others = add(new BoolSetting("others", "Hide other players' names", false));
		final BoolSetting ownSkin = add(new BoolSetting("own-skin", "Hide your skin", false));
		final BoolSetting otherSkins = add(new BoolSetting("other-skins", "Hide other skins", false));

		private NickHider() {
			super("nickhider", "Nick Hider", "nick", EnumSet.of(Tab.HYPIXEL, Tab.GAME), false);
			describe("Hides your name (for recordings and screenshots). Only on your screen.");
			item(Items.NAME_TAG);
		}

		public static boolean hideSkin(boolean own) {
			return I.on() && (own ? I.ownSkin.on() : I.otherSkins.on());
		}

		/** The text with your name (and others' when chosen) swapped, keeping colours and styles. */
		public static Component apply(Component text) {
			NickHider n = I;
			Minecraft mc = Minecraft.getInstance();
			if (!n.on() || mc.player == null || text == null) return text;
			String me = mc.player.getGameProfile().name();
			List<String> others = new ArrayList<>();
			if (n.others.on() && mc.getConnection() != null) {
				for (var info : mc.getConnection().getOnlinePlayers()) {
					String name = info.getProfile().name();
					if (!name.equals(me)) others.add(name);
				}
			}
			String plain = text.getString();
			boolean hit = plain.contains(me);
			for (String o : others) hit |= plain.contains(o);
			if (!hit) return text;
			MutableComponent out = Component.empty();
			text.visit((style, part) -> {
				String s = part.replace(me, n.nick.get());
				for (String o : others) s = s.replace(o, "Player");
				out.append(Component.literal(s).withStyle(style));
				return Optional.empty();
			}, Style.EMPTY);
			return out;
		}
	}
}
