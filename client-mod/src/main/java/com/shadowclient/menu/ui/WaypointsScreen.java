package com.shadowclient.menu.ui;

import com.shadowclient.menu.module.Waypoints;
import java.util.List;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import org.jetbrains.annotations.Nullable;

/** The waypoints of the world you're in, with delete buttons. */
public final class WaypointsScreen extends Screen {
	@Nullable private final Screen parent;
	private static final int ROWS = 8;

	public WaypointsScreen(@Nullable Screen parent) {
		super(Component.literal("Waypoints"));
		this.parent = parent;
	}

	@Override
	protected void init() {
		List<Waypoints.Point> points = Waypoints.list();
		int y = 44;
		for (int i = 0; i < Math.min(points.size(), ROWS); i++) {
			int index = i;
			addRenderableWidget(Button.builder(Component.literal("Delete"), b -> {
				Waypoints.remove(index);
				rebuildWidgets();
			}).bounds(width / 2 + 100, y - 4, 50, 16).build());
			y += 20;
		}
		addRenderableWidget(Button.builder(Component.literal("Clear all"), b -> {
			Waypoints.clear();
			rebuildWidgets();
		}).bounds(width / 2 - 104, height - 32, 100, 20).build());
		addRenderableWidget(Button.builder(Component.literal("Done"), b -> onClose()).bounds(width / 2 + 4, height - 32, 100, 20).build());
	}

	@Override
	public void render(GuiGraphics g, int mx, int my, float partial) {
		super.render(g, mx, my, partial);
		g.drawCenteredString(font, title, width / 2, 14, 0xFFFFFFFF);
		List<Waypoints.Point> points = Waypoints.list();
		if (Waypoints.worldKey() == null) {
			g.drawCenteredString(font, "Join a world to see its waypoints.", width / 2, 50, 0xFFB4B4B4);
			return;
		}
		if (points.isEmpty()) {
			g.drawCenteredString(font, "No waypoints yet. Press the Add Waypoint key (B) where you want one.", width / 2, 50, 0xFFB4B4B4);
			return;
		}
		g.drawCenteredString(font, "Add Waypoint key: B (change it in Controls)", width / 2, 28, 0xFF7A7A7A);
		int y = 44;
		for (int i = 0; i < Math.min(points.size(), ROWS); i++) {
			Waypoints.Point p = points.get(i);
			g.fill(width / 2 - 150, y - 1, width / 2 - 144, y + 7, p.colour());
			String dim = p.dimension().substring(p.dimension().indexOf(':') + 1);
			g.drawString(font, p.name() + "  " + p.x() + " " + p.y() + " " + p.z() + "  (" + dim + ")", width / 2 - 138, y, 0xFFFFFFFF, true);
			y += 20;
		}
		if (points.size() > ROWS) g.drawString(font, "+" + (points.size() - ROWS) + " more", width / 2 - 138, y, 0xFF7A7A7A, true);
	}

	@Override
	public void onClose() {
		minecraft.setScreen(parent);
	}
}
