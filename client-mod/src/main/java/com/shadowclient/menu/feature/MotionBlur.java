package com.shadowclient.menu.feature;

import com.mojang.blaze3d.resource.GraphicsResourceAllocator;
import com.shadowclient.menu.ShadowMenu;
import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.NumberSetting;
import java.util.EnumSet;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.LevelTargetBundle;
import net.minecraft.client.renderer.PostChain;
import net.minecraft.resources.Identifier;

/**
 * Motion blur: each frame is blended with the one before it (Shadow Menu's own post effect,
 * assets/shadowmenu/post_effect/motion_blur_N.json, one file per strength step). Costs some FPS.
 */
public final class MotionBlur extends Feature {
	public static final MotionBlur I = new MotionBlur();
	final NumberSetting strength = add(new NumberSetting("strength", "Strength", 50, 10, 90, 10).suffix("%"));
	private static boolean warned;

	private MotionBlur() {
		super("motion-blur", "Motion Blur", "motionblur", EnumSet.of(Tab.VISUAL, Tab.PVP), false);
		describe("Blends each frame with the last one for a smooth, blurred look when you turn. Costs some FPS.");
	}

	/** After the world is drawn (before the HUD): blend it with the previous frame. */
	public static void apply(GraphicsResourceAllocator pool) {
		MotionBlur m = I;
		if (!m.on()) return;
		Minecraft mc = Minecraft.getInstance();
		Identifier id = Identifier.fromNamespaceAndPath("shadowmenu", "motion_blur_" + m.strength.i());
		try {
			PostChain chain = mc.getShaderManager().getPostChain(id, LevelTargetBundle.MAIN_TARGETS);
			if (chain != null) chain.process(mc.getMainRenderTarget(), pool);
		} catch (RuntimeException e) {
			if (!warned) {
				warned = true;
				ShadowMenu.LOG.warn("Motion blur couldn't run; it stays off for this session", e);
			}
			m.enabled.set(false);
		}
	}
}
