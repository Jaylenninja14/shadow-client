package com.shadowclient.menu.hud;

import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ChoiceSetting;
import com.shadowclient.menu.setting.ColorSetting;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.world.entity.EquipmentSlot;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;

/** Your armor (and held item) with how much durability is left. */
final class ArmorStatus extends HudElement {
	final ChoiceSetting layout = add(new ChoiceSetting("layout", "Layout", "vertical", List.of("vertical", "horizontal"), List.of("Vertical", "Horizontal")));
	final ChoiceSetting value = add(new ChoiceSetting("value", "Durability", "percent", List.of("percent", "number", "bar", "none"),
			List.of("Percent", "Points left", "Bar only", "Hidden")));
	final BoolSetting held = add(new BoolSetting("held", "Show held item", true));
	final BoolSetting offhand = add(new BoolSetting("offhand", "Show offhand item", false));
	final BoolSetting colorByDamage = add(new BoolSetting("damage-colour", "Colour by durability", true).group("Style"));
	final ColorSetting textColor = add(new ColorSetting("text-color", "Text colour", 0xFFFFFFFF, false).group("Style").showIf(() -> !colorByDamage.on()));
	final BoolSetting shadow = add(new BoolSetting("shadow", "Text shadow", true).group("Style"));
	private static final EquipmentSlot[] SLOTS = {EquipmentSlot.HEAD, EquipmentSlot.CHEST, EquipmentSlot.LEGS, EquipmentSlot.FEET};
	private static final ItemStack[] SAMPLE = {new ItemStack(Items.DIAMOND_HELMET), new ItemStack(Items.DIAMOND_CHESTPLATE), new ItemStack(Items.DIAMOND_LEGGINGS),
			new ItemStack(Items.DIAMOND_BOOTS), new ItemStack(Items.DIAMOND_SWORD)};

	ArmorStatus() {
		super("armor-status", "Armor Status", "armor", InfoElements.PVP, true, 0.996, 0.62);
		describe("Your armor and held item, with durability.");
		anchor(Anchor.BOTTOM_RIGHT);
	}

	private List<ItemStack> items(boolean preview) {
		List<ItemStack> out = new ArrayList<>();
		LocalPlayer p = mc.player;
		if (p == null) {
			for (ItemStack s : SAMPLE) out.add(s);
			return out;
		}
		for (EquipmentSlot slot : SLOTS) {
			ItemStack s = p.getItemBySlot(slot);
			if (!s.isEmpty()) out.add(s);
		}
		if (held.on() && !p.getMainHandItem().isEmpty()) out.add(p.getMainHandItem());
		if (offhand.on() && !p.getOffhandItem().isEmpty()) out.add(p.getOffhandItem());
		if (out.isEmpty() && preview) for (ItemStack s : SAMPLE) out.add(s);
		return out;
	}

	@Override
	public boolean hasContent() {
		return !items(false).isEmpty();
	}

	private String label(ItemStack s) {
		if (!s.isDamageableItem()) return "";
		int left = s.getMaxDamage() - s.getDamageValue();
		return switch (value.get()) {
			case "number" -> String.valueOf(left);
			case "percent" -> Math.round(left * 100f / s.getMaxDamage()) + "%";
			default -> "";
		};
	}

	private int textWidth(boolean preview) {
		int w = 0;
		for (ItemStack s : items(preview)) w = Math.max(w, font().width(label(s)));
		return w;
	}

	@Override
	public int width(boolean preview) {
		int n = Math.max(1, items(preview).size());
		int tw = textWidth(preview);
		return layout.is("vertical") ? 16 + (tw > 0 ? 3 + tw : 0) : n * (16 + (tw > 0 ? 3 + tw : 0) + 4) - 4;
	}

	@Override
	public int height(boolean preview) {
		int n = Math.max(1, items(preview).size());
		return layout.is("vertical") ? n * 17 - 1 : 16;
	}

	@Override
	public void render(GuiGraphics g, boolean preview) {
		List<ItemStack> list = items(preview);
		int tw = textWidth(preview);
		int x = 0, y = 0;
		for (ItemStack s : list) {
			g.renderItem(s, x, y);
			// the durability bar, or the stack size for things like arrows and pearls
			if (value.is("bar") || !s.isDamageableItem()) g.renderItemDecorations(font(), s, x, y);
			String l = s.isDamageableItem() ? label(s) : "";
			if (!l.isEmpty()) {
				int c = textColor.argb();
				if (colorByDamage.on() && s.isDamageableItem()) {
					float f = 1f - s.getDamageValue() / (float) s.getMaxDamage();
					c = 0xFF000000 | (java.awt.Color.HSBtoRGB(f / 3f, 0.75f, 1f) & 0xFFFFFF);
				}
				g.drawString(font(), l, x + 19, y + 4, c, shadow.on());
			}
			if (layout.is("vertical")) y += 17;
			else x += 16 + (tw > 0 ? 3 + tw : 0) + 4;
		}
	}
}
