package com.shadowclient.menu.ui;

import java.util.HashMap;
import java.util.Map;

/**
 * Smoothly animated values for a screen (hover glow, switch knobs, page slides). Each value eases
 * towards its target at {@code rate} per second, scaled by the theme's animation speed; with
 * animations off it jumps straight there.
 */
public final class Anim {
	private final Map<String, Float> values = new HashMap<>();
	private long last = System.nanoTime();
	private float dt;

	/** Call once per frame before reading values. */
	public void frame() {
		long now = System.nanoTime();
		dt = Math.min(0.1f, (now - last) / 1e9f);
		last = now;
	}

	public float get(String key, float target, float rate) {
		float speed = Theme.speed();
		if (speed <= 0) {
			values.put(key, target);
			return target;
		}
		float cur = values.getOrDefault(key, target);
		float k = 1 - (float) Math.exp(-rate * speed * dt);
		cur += (target - cur) * k;
		if (Math.abs(target - cur) < 0.002f) cur = target;
		values.put(key, cur);
		return cur;
	}

	/** Start a value somewhere else (e.g. a page sliding in from the right). */
	public void set(String key, float value) {
		values.put(key, value);
	}

	public float peek(String key, float def) {
		return values.getOrDefault(key, def);
	}

	public float dt() {
		return dt;
	}
}
