package com.shadowclient.menu.setting;

import com.google.gson.JsonElement;
import com.google.gson.JsonPrimitive;
import java.util.Locale;
import java.util.function.DoubleFunction;

/** A slider between {@code min} and {@code max}, snapped to {@code step}. */
public final class NumberSetting extends Setting<Double> {
	public final double min, max, step;
	private DoubleFunction<String> format;

	public NumberSetting(String id, String name, double def, double min, double max, double step) {
		super(id, name, def);
		this.min = min;
		this.max = max;
		this.step = step;
		this.format = step >= 1 ? v -> String.valueOf(Math.round(v)) : v -> String.format(Locale.ROOT, step >= 0.1 ? "%.1f" : "%.2f", v);
	}

	/** Shown value, e.g. {@code v -> Math.round(v * 100) + "%"}. */
	public NumberSetting format(DoubleFunction<String> f) {
		this.format = f;
		return this;
	}

	public NumberSetting suffix(String s) {
		DoubleFunction<String> base = format;
		this.format = v -> base.apply(v) + s;
		return this;
	}

	public String text() {
		return format.apply(value);
	}

	public float f() {
		return value.floatValue();
	}

	public int i() {
		return (int) Math.round(value);
	}

	/** 0..1 position of the value on the slider. */
	public double fraction() {
		return max <= min ? 0 : (value - min) / (max - min);
	}

	public void setFraction(double f) {
		set(min + Math.max(0, Math.min(1, f)) * (max - min));
	}

	@Override
	public void set(Double v) {
		if (v == null || v.isNaN()) return;
		double snapped = step > 0 ? min + Math.round((v - min) / step) * step : v;
		snapped = Math.max(min, Math.min(max, snapped));
		// avoid 0.30000000000000004 in the file
		snapped = Math.round(snapped * 10000) / 10000.0;
		super.set(snapped);
	}

	@Override
	public JsonElement toJson() {
		return new JsonPrimitive(value);
	}

	@Override
	public void fromJson(JsonElement e) {
		if (e.isJsonPrimitive() && e.getAsJsonPrimitive().isNumber()) value = Math.max(min, Math.min(max, e.getAsDouble()));
	}
}
