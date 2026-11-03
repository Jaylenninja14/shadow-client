package com.shadowclient.menu.feature;

import com.shadowclient.menu.module.Module.Tab;
import com.shadowclient.menu.setting.BoolSetting;
import com.shadowclient.menu.setting.ChoiceSetting;
import com.shadowclient.menu.setting.KeySetting;
import java.util.EnumSet;
import java.util.List;
import net.minecraft.client.CameraType;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.util.Mth;

/**
 * Hold the freelook key (V) to look around your player without turning: the camera moves to third
 * person and the mouse turns only the camera. Some servers don't allow it.
 */
public final class Freelook extends Feature {
	public static final Freelook I = new Freelook();
	static KeyMapping key;
	final KeySetting keySetting = add(new KeySetting("key", "Freelook key", () -> key));
	final ChoiceSetting mode = add(new ChoiceSetting("mode", "Key mode", "hold", List.of("hold", "toggle"), List.of("Hold", "Toggle")));
	final ChoiceSetting view = add(new ChoiceSetting("view", "Camera", "back", List.of("back", "front", "first"), List.of("Third person back", "Third person front", "First person")));
	final BoolSetting invert = add(new BoolSetting("invert", "Invert up/down", false));
	private boolean active, toggled;
	private float yaw, pitch;
	private CameraType before = CameraType.FIRST_PERSON;

	private Freelook() {
		super("freelook", "Freelook", "freelook", EnumSet.of(Tab.PVP), true);
		describe("Hold V to look around without turning your player. Not allowed on some servers.");
	}

	static void tick(Minecraft mc) {
		Freelook f = I;
		if (key == null) return;
		boolean want;
		if (f.mode.is("toggle")) {
			while (key.consumeClick()) f.toggled = !f.toggled;
			want = f.toggled;
		} else {
			while (key.consumeClick()) {
				// clicks are only for toggle mode
			}
			want = key.isDown();
		}
		want &= f.on() && mc.player != null && mc.screen == null;
		if (want == f.active) return;
		f.active = want;
		if (want) {
			f.yaw = mc.player.getYRot();
			f.pitch = mc.player.getXRot();
			f.before = mc.options.getCameraType();
			CameraType to = switch (f.view.get()) {
				case "front" -> CameraType.THIRD_PERSON_FRONT;
				case "first" -> CameraType.FIRST_PERSON;
				default -> CameraType.THIRD_PERSON_BACK;
			};
			mc.options.setCameraType(to);
		} else {
			f.toggled = false;
			mc.options.setCameraType(f.before);
		}
	}

	public static boolean active() {
		return I.active;
	}

	/** Mouse movement while freelooking turns the camera (same speed as turning the player). */
	public static void turn(double dx, double dy) {
		Freelook f = I;
		f.yaw += (float) dx * 0.15f;
		f.pitch = Mth.clamp(f.pitch + (float) dy * 0.15f * (f.invert.on() ? -1 : 1), -90, 90);
	}

	public static float yaw(float original) {
		return I.active ? I.yaw : original;
	}

	public static float pitch(float original) {
		return I.active ? I.pitch : original;
	}
}
