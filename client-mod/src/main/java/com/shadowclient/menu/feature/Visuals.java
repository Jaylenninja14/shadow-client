package com.shadowclient.menu.feature;

import com.mojang.blaze3d.platform.NativeImage;
import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ColorSetting;
import com.shadowclient.menu.setting.KeySetting;
import com.shadowclient.menu.setting.NumberSetting;
import java.util.EnumSet;
import java.util.Locale;
import java.util.Set;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.item.Items;
import org.jetbrains.annotations.Nullable;

/**
 * The look-and-feel modules Shadow Menu does with small mixins: fullbright, time changer, block
 * outline, hit colour, no hurt cam, low fire/shield, minimal view bob, no vignette, no rain,
 * nametags, TNT timer, beacon beams and hit particles. Mixins read the static instances.
 */
public final class Visuals {
	private Visuals() {
	}

	static final Set<Tab> VISUAL = EnumSet.of(Tab.VISUAL);
	static final Set<Tab> PVP_VISUAL = EnumSet.of(Tab.PVP, Tab.VISUAL);

	public static final class Fullbright extends Feature {
		public static final Fullbright I = new Fullbright();
		static KeyMapping key;
		final KeySetting keySetting = add(new KeySetting("key", "Toggle key", () -> key));

		private Fullbright() {
			super("fullbright", "Fullbright", "fullbright", VISUAL, false);
			describe("See in the dark as if you had night vision.");
		}

		static void tick(Minecraft mc) {
			while (key != null && key.consumeClick()) {
				I.setOn(!I.on());
				if (mc.player != null) mc.player.displayClientMessage(Component.literal("Fullbright " + (I.on() ? "on" : "off")), true);
			}
		}

		/** Night vision strength for the lightmap (1 = full). */
		public static float nightVision(float vanilla) {
			return I.on() ? 1f : vanilla;
		}
	}

	public static final class TimeChanger extends Feature {
		public static final TimeChanger I = new TimeChanger();
		final NumberSetting time = add(new NumberSetting("time", "Time of day", 6000, 0, 23500, 500).format(TimeChanger::clock));

		private TimeChanger() {
			super("time-changer", "Time Changer", "time", VISUAL, false);
			describe("Choose the time of day you see (only you see it).");
		}

		private static String clock(double ticks) {
			long t = Math.round(ticks);
			return String.format(Locale.ROOT, "%02d:%02d", (t / 1000 + 6) % 24, t % 1000 * 60 / 1000);
		}

		public static long dayTime(long real) {
			if (!I.on()) return real;
			// keep the day count so the moon phase doesn't jump
			return real - Math.floorMod(real, 24000L) + I.time.i();
		}
	}

	public static final class BlockOutline extends Feature {
		public static final BlockOutline I = new BlockOutline();
		final ColorSetting color = add(new ColorSetting("color", "Colour", 0xFF9B7BFF, true));
		final NumberSetting width = add(new NumberSetting("width", "Thickness", 1.5, 0.5, 5, 0.5).suffix("x"));

		private BlockOutline() {
			super("block-outline", "Block Outline", "outline", VISUAL, false);
			describe("Colour and thickness of the outline around the block you look at.");
		}

		public static int color(int vanilla) {
			return I.on() ? I.color.argb() : vanilla;
		}

		public static float width(float vanilla) {
			return I.on() ? vanilla * I.width.f() : vanilla;
		}
	}

	/** Hit Color: the tint on players and mobs when they're hit (Minecraft's is red). */
	public static final class HitColor extends Feature {
		public static final HitColor I = new HitColor();
		final ColorSetting color = add(new ColorSetting("color", "Hit colour", 0xB3FF0000, true));
		@Nullable private static DynamicTexture texture;
		private static int applied;

		private HitColor() {
			super("hit-color", "Hit Color", "hitcolor", PVP_VISUAL, false);
			describe("Pick the colour players and mobs flash when they're hit.");
		}

		/** The game made its overlay texture; remember it so the colour can change later. */
		public static void created(DynamicTexture t) {
			texture = t;
			applied = 0;
			refresh();
		}

		/** Rewrite the hurt rows of the overlay texture when the colour (or the switch) changed. */
		public static void refresh() {
			if (texture == null) return;
			int want = I.on() ? I.color.argb() : 0xB3FF0000;
			if (want == applied) return;
			NativeImage img = texture.getPixels();
			if (img == null) return;
			for (int y = 0; y < 8; y++) for (int x = 0; x < 16; x++) img.setPixel(x, y, want);
			texture.upload();
			applied = want;
		}
	}

	public static final class NoHurtCam extends Feature {
		public static final NoHurtCam I = new NoHurtCam();

		private NoHurtCam() {
			super("no-hurt-cam", "No Hurt Cam", "hurtcam", EnumSet.of(Tab.PVP), false);
			describe("The camera doesn't shake when you take damage.");
		}
	}

	public static final class LowFire extends Feature {
		public static final LowFire I = new LowFire();
		final NumberSetting height = add(new NumberSetting("height", "Lower by", 0.3, 0, 0.6, 0.05).format(v -> Math.round(v * 100) + "%"));
		final BoolSetting hide = add(new BoolSetting("hide", "Hide the flames completely", false));

		private LowFire() {
			super("low-fire", "Low Fire", "fire", PVP_VISUAL, false);
			describe("Moves the fire overlay down so you can see while burning.");
		}

		public static float offset() {
			return I.on() ? I.height.f() : 0;
		}

		public static boolean hidden() {
			return I.on() && I.hide.on();
		}
	}

	public static final class LowShield extends Feature {
		public static final LowShield I = new LowShield();
		final NumberSetting height = add(new NumberSetting("height", "Lower by", 0.15, 0, 0.4, 0.05).format(v -> Math.round(v * 100) + "%"));

		private LowShield() {
			super("low-shield", "Low Shield", "shield", PVP_VISUAL, false);
			describe("Holds your shield lower when you're not blocking.");
			item(Items.SHIELD);
		}

		public static float offset() {
			return I.on() ? I.height.f() : 0;
		}
	}

	public static final class MinimalBob extends Feature {
		public static final MinimalBob I = new MinimalBob();

		private MinimalBob() {
			super("minimal-bobbing", "Minimal View Bob", "viewbob", PVP_VISUAL, false);
			describe("Only your hand bobs when you walk, not the whole world.");
		}
	}

	public static final class NoVignette extends Feature {
		public static final NoVignette I = new NoVignette();

		private NoVignette() {
			super("no-vignette", "Remove Vignette", "vignette", VISUAL, false);
			describe("No dark corners around the screen.");
		}
	}

	public static final class NoRain extends Feature {
		public static final NoRain I = new NoRain();

		private NoRain() {
			super("no-rain", "No Rain", "rain", VISUAL, false);
			describe("Hides rain and snow (and their splashes and sounds) for you.");
		}
	}

	public static final class Nametags extends Feature {
		public static final Nametags I = new Nametags();
		final BoolSetting own = add(new BoolSetting("own", "Show your own nametag in third person", true));
		final NumberSetting background = add(new NumberSetting("background", "Background", 25, 0, 100, 5).suffix("%"));

		private Nametags() {
			super("nametags", "Nametags", "nametag", PVP_VISUAL, false);
			describe("Your own nametag in third person, and how dark the nametag background is.");
		}

		public static boolean showOwn() {
			return I.on() && I.own.on();
		}

		public static float background(float vanilla) {
			return I.on() ? I.background.f() / 100f : vanilla;
		}
	}

	public static final class TntTimer extends Feature {
		public static final TntTimer I = new TntTimer();
		final NumberSetting decimals = add(new NumberSetting("decimals", "Decimals", 1, 0, 2, 1));
		final BoolSetting colors = add(new BoolSetting("colors", "Colour by time left", true));

		private TntTimer() {
			super("tnt-timer", "TNT Timer", "tnt", PVP_VISUAL, false);
			describe("Shows the seconds left above lit TNT.");
			item(Items.TNT);
		}

		@Nullable
		public static Component label(float fuseTicks) {
			if (!I.on()) return null;
			float s = Math.max(0, fuseTicks / 20f);
			String text = String.format(Locale.ROOT, "%." + I.decimals.i() + "fs", s);
			int color = !I.colors.on() ? 0xFFFFFF : s > 2.5f ? 0x55FF55 : s > 1f ? 0xFFDD55 : 0xFF5555;
			return Component.literal(text).withColor(color);
		}
	}

	public static final class BeaconBeams extends Feature {
		public static final BeaconBeams I = new BeaconBeams();

		private BeaconBeams() {
			super("hide-beacon-beams", "Hide Beacon Beams", "beacon", VISUAL, false);
			describe("Hides beacon beams.");
		}
	}

	/** More hit particles: crits and sharpness sparks on every hit, several times over. */
	public static final class HitParticles extends Feature {
		public static final HitParticles I = new HitParticles();
		final NumberSetting multiplier = add(new NumberSetting("multiplier", "Multiplier", 2, 1, 10, 1).suffix("x"));
		final BoolSetting crits = add(new BoolSetting("crits", "Always show crit particles", true));
		final BoolSetting sharpness = add(new BoolSetting("sharpness", "Always show sharpness particles", false));

		private HitParticles() {
			super("hit-particles", "Hit Particles", "particles", PVP_VISUAL, false);
			describe("More particles when you hit something.");
		}

		public static void hit(Entity target) {
			HitParticles p = I;
			if (!p.on() || !(target instanceof LivingEntity)) return;
			Minecraft mc = Minecraft.getInstance();
			for (int i = 0; i < p.multiplier.i(); i++) {
				if (p.crits.on()) mc.particleEngine.createTrackingEmitter(target, ParticleTypes.CRIT);
				if (p.sharpness.on()) mc.particleEngine.createTrackingEmitter(target, ParticleTypes.ENCHANTED_HIT);
			}
		}
	}

	public static final class ScrollTooltips extends Feature {
		public static final ScrollTooltips I = new ScrollTooltips();
		final NumberSetting speed = add(new NumberSetting("speed", "Scroll speed", 10, 2, 30, 1));
		final BoolSetting invert = add(new BoolSetting("invert", "Invert", false));
		private static int dx, dy;
		@Nullable private static String last;
		private static long shownAt;
		private static boolean tooBig;

		private ScrollTooltips() {
			super("scroll-tooltips", "Scrollable Tooltips", "tooltip", EnumSet.of(Tab.GAME), true);
			describe("Scroll tooltips that don't fit on screen up and down (hold Shift for sideways).");
		}

		/**
		 * A tooltip is being drawn; a different one starts unscrolled. {@code doesntFit}: it's bigger
		 * than the screen (only those take the mouse wheel). Returns the offset.
		 */
		public static int[] offset(String first, boolean doesntFit) {
			if (!first.equals(last)) {
				last = first;
				dx = dy = 0;
			}
			shownAt = System.currentTimeMillis();
			tooBig = doesntFit;
			return I.on() ? new int[] {dx, dy} : new int[] {0, 0};
		}

		/** The mouse wheel turned in a screen; true when a tooltip was scrolled. */
		public static boolean scrolled(double amount, boolean sideways) {
			if (!I.on() || !tooBig || System.currentTimeMillis() - shownAt > 100 || amount == 0) return false;
			int step = (int) Math.round(Math.signum(amount) * I.speed.get()) * (I.invert.on() ? -1 : 1);
			if (sideways) dx += step;
			else dy += step;
			return true;
		}
	}

	public static final class ConfirmDisconnect extends Feature {
		public static final ConfirmDisconnect I = new ConfirmDisconnect();

		private ConfirmDisconnect() {
			super("confirm-disconnect", "Confirm Disconnect", "exit", EnumSet.of(Tab.GAME), false);
			describe("Asks before Disconnect / Save and Quit, so a misclick doesn't kick you out.");
		}
	}
}
