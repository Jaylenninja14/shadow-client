package com.shadowclient.menu.mixin;

import com.shadowclient.menu.feature.Visuals;
import java.util.List;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.inventory.tooltip.ClientTooltipComponent;
import net.minecraft.client.gui.screens.inventory.tooltip.ClientTooltipPositioner;
import net.minecraft.resources.Identifier;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Scrollable Tooltips: a long tooltip moves by however far it was scrolled (the pose is pushed just before). */
@Mixin(GuiGraphics.class)
public abstract class GuiGraphicsMixin {
	@Inject(method = "renderTooltip", at = @At(value = "INVOKE",
			target = "Lnet/minecraft/client/gui/screens/inventory/tooltip/TooltipRenderUtil;renderTooltipBackground(Lnet/minecraft/client/gui/GuiGraphics;IIIILnet/minecraft/resources/Identifier;)V"),
			require = 0)
	private void shadowmenu$scrollTooltip(Font font, List<ClientTooltipComponent> lines, int x, int y, ClientTooltipPositioner positioner, Identifier style, CallbackInfo ci) {
		if (lines.isEmpty()) return;
		GuiGraphics g = (GuiGraphics) (Object) this;
		int w = 0, h = 0;
		for (ClientTooltipComponent line : lines) {
			w = Math.max(w, line.getWidth(font));
			h += line.getHeight(font);
		}
		// the same tooltip keeps its scroll; a different one (other size) starts at the top
		int[] off = Visuals.ScrollTooltips.offset(lines.size() + ":" + w + ":" + h, h + 8 > g.guiHeight() || w + 8 > g.guiWidth());
		if (off[0] != 0 || off[1] != 0) g.pose().translate(off[0], off[1]);
	}
}
