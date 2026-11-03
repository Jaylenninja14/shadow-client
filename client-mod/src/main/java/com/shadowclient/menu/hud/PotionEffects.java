package com.shadowclient.menu.hud;

import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ChoiceSetting;
import com.shadowclient.menu.setting.ColorSetting;
import com.shadowclient.menu.ui.Draw;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.gui.Gui;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.core.Holder;
import net.minecraft.network.chat.Component;
import net.minecraft.util.StringUtil;
import net.minecraft.world.effect.MobEffect;
import net.minecraft.world.effect.MobEffectInstance;
import net.minecraft.world.effect.MobEffects;

/** Active potion effects with icon, level and time left. Can hide Minecraft's own effect icons. */
final class PotionEffects extends HudElement {
	final BoolSetting hideVanilla = add(new BoolSetting("hide-vanilla", "Hide Minecraft's effect icons", true));
	final BoolSetting icons = add(new BoolSetting("icons", "Show icons", true));
	final BoolSetting names = add(new BoolSetting("names", "Show names", true));
	final BoolSetting blink = add(new BoolSetting("blink", "Blink when ending", true));
	final ChoiceSetting align = add(new ChoiceSetting("align", "Alignment", "right", List.of("left", "right"), List.of("Icon left", "Icon right")));
	final ColorSetting nameColor = add(new ColorSetting("name-color", "Name colour", 0xFFFFFFFF, false).group("Style"));
	final BoolSetting effectColor = add(new BoolSetting("effect-colour", "Use the effect's colour", false).group("Style"));
	final ColorSetting timeColor = add(new ColorSetting("time-color", "Time colour", 0xFFB4B4C4, false).group("Style"));
	final BoolSetting shadow = add(new BoolSetting("shadow", "Text shadow", true).group("Style"));
	final BoolSetting background = add(new BoolSetting("background", "Background", false).group("Style"));
	final ColorSetting backgroundColor = add(new ColorSetting("background-color", "Background colour", 0x6F000000, true).group("Style").showIf(background::on));

	PotionEffects() {
		super("potion-effects", "Potion Effects", "potion", InfoElements.PVP, true, 0.996, 0.36);
		describe("Your active effects and how long they last.");
		anchor(Anchor.TOP_RIGHT);
	}

	private List<MobEffectInstance> effects(boolean preview) {
		List<MobEffectInstance> out = new ArrayList<>();
		if (mc.player != null) {
			for (MobEffectInstance e : mc.player.getActiveEffects()) if (e.showIcon()) out.add(e);
			out.sort((a, b) -> Integer.compare(b.getDuration(), a.getDuration()));
		}
		if (out.isEmpty() && preview) {
			out.add(new MobEffectInstance(MobEffects.SPEED, 20 * 95, 1));
			out.add(new MobEffectInstance(MobEffects.STRENGTH, 20 * 42, 0));
			out.add(new MobEffectInstance(MobEffects.FIRE_RESISTANCE, 20 * 300, 0));
		}
		return out;
	}

	public boolean hidesVanilla() {
		return enabled.on() && hideVanilla.on();
	}

	@Override
	public boolean hasContent() {
		return !effects(false).isEmpty();
	}

	private String name(MobEffectInstance e) {
		String n = Component.translatable(e.getDescriptionId()).getString();
		int level = e.getAmplifier() + 1;
		if (level > 1) n += " " + roman(level);
		return n;
	}

	private static String roman(int n) {
		String[] r = {"", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"};
		return n < r.length ? r[n] : String.valueOf(n);
	}

	private String time(MobEffectInstance e) {
		return e.isInfiniteDuration() ? "**:**" : StringUtil.formatTickDuration(e.getDuration(), 20f);
	}

	private int textWidth(boolean preview) {
		int w = 0;
		for (MobEffectInstance e : effects(preview)) w = Math.max(w, Math.max(names.on() ? font().width(name(e)) : 0, font().width(time(e))));
		return w;
	}

	@Override
	public int width(boolean preview) {
		return (icons.on() ? 20 : 0) + textWidth(preview) + (background.on() ? 6 : 0);
	}

	@Override
	public int height(boolean preview) {
		int n = Math.max(1, effects(preview).size());
		return n * (names.on() ? 22 : 12) - 2 + (background.on() ? 4 : 0);
	}

	@Override
	public void render(GuiGraphics g, boolean preview) {
		int w = width(preview);
		if (background.on()) Draw.round(g, 0, 0, w, height(preview), 3, backgroundColor.argb());
		int pad = background.on() ? 3 : 0;
		int y = pad / 2 + (background.on() ? 1 : 0);
		boolean right = align.is("right");
		int tw = textWidth(preview);
		for (MobEffectInstance e : effects(preview)) {
			int rowH = names.on() ? 20 : 10;
			float alpha = 1f;
			if (blink.on() && !e.isInfiniteDuration() && e.getDuration() < 200) alpha = 0.35f + 0.65f * (float) Math.abs(Math.cos(e.getDuration() * Math.PI / 20));
			int textX = pad + (icons.on() && !right ? 20 : 0);
			if (icons.on()) {
				Holder<MobEffect> h = e.getEffect();
				int ix = right ? w - pad - 18 : pad;
				g.blitSprite(RenderPipelines.GUI_TEXTURED, Gui.getMobEffectSprite(h), ix, y + (rowH - 18) / 2, 18, 18, Draw.fade(0xFFFFFFFF, alpha));
			}
			int nc = effectColor.on() ? 0xFF000000 | e.getEffect().value().getColor() : nameColor.argb();
			if (names.on()) {
				String n = name(e);
				int nx = right ? textX + tw - font().width(n) : textX;
				g.drawString(font(), n, nx, y + 1, Draw.fade(nc, alpha), shadow.on());
				String t = time(e);
				g.drawString(font(), t, right ? textX + tw - font().width(t) : textX, y + 11, Draw.fade(timeColor.argb(), alpha), shadow.on());
			} else {
				String t = time(e);
				g.drawString(font(), t, right ? textX + tw - font().width(t) : textX, y + 1, Draw.fade(nc, alpha), shadow.on());
			}
			y += rowH + 2;
		}
	}
}
