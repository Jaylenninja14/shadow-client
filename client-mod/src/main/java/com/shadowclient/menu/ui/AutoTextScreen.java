package com.shadowclient.menu.ui;

import com.shadowclient.menu.module.AutoText;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.options.controls.KeyBindsScreen;
import net.minecraft.network.chat.Component;
import org.jetbrains.annotations.Nullable;

/** Edit the five Auto Text messages; their keys are set in Controls. */
public final class AutoTextScreen extends Screen {
	@Nullable private final Screen parent;
	private final List<EditBox> boxes = new ArrayList<>();

	public AutoTextScreen(@Nullable Screen parent) {
		super(Component.literal("Auto Text Hotkeys"));
		this.parent = parent;
	}

	@Override
	protected void init() {
		boxes.clear();
		List<String> messages = AutoText.messages();
		for (int i = 0; i < AutoText.SLOTS; i++) {
			EditBox box = new EditBox(font, width / 2 - 90, 48 + i * 26, 220, 18, Component.literal("Message " + (i + 1)));
			box.setMaxLength(256);
			box.setValue(messages.get(i));
			box.setHint(Component.literal("e.g. /gg or a chat message"));
			boxes.add(addRenderableWidget(box));
		}
		addRenderableWidget(Button.builder(Component.literal("Set keys…"), b -> {
			save();
			minecraft.setScreen(new KeyBindsScreen(this, minecraft.options));
		}).bounds(width / 2 - 104, height - 32, 100, 20).build());
		addRenderableWidget(Button.builder(Component.literal("Done"), b -> onClose()).bounds(width / 2 + 4, height - 32, 100, 20).build());
	}

	private void save() {
		List<String> messages = new ArrayList<>();
		for (EditBox b : boxes) messages.add(b.getValue());
		AutoText.save(messages);
	}

	@Override
	public void render(GuiGraphics g, int mx, int my, float partial) {
		super.render(g, mx, my, partial);
		g.drawCenteredString(font, title, width / 2, 14, 0xFFFFFFFF);
		g.drawCenteredString(font, "Messages starting with / are sent as commands. Keys: Controls → Shadow Menu.", width / 2, 30, 0xFF9A9A9A);
		for (int i = 0; i < AutoText.SLOTS; i++) g.drawString(font, "Key " + (i + 1), width / 2 - 130, 53 + i * 26, 0xFFB4B4B4, true);
	}

	@Override
	public void onClose() {
		save();
		minecraft.setScreen(parent);
	}
}
