package com.shadowclient.combat;

import java.util.Locale;
import java.util.Optional;
import org.bukkit.configuration.ConfigurationSection;

/** The server's combat rules (config.yml), plus the small pieces of timing math. */
public record CombatRules(boolean enabled, int hitDelayTicks, boolean noAttackCooldown, boolean noShieldDelay, boolean allowPlayerToggle) {

    /** Vanilla: a target ignores further full hits for 10 ticks (half its 20 no-damage ticks). */
    public static final int VANILLA_HIT_DELAY = 10;

    /** Attack speed base value used for "no attack cooldown"; also marks values we set. */
    public static final double NO_COOLDOWN_ATTACK_SPEED = 1024.0;

    public static CombatRules from(ConfigurationSection c) {
        return new CombatRules(
                c.getBoolean("enabled", true),
                clampHitDelay(c.getInt("hit-delay-ticks", VANILLA_HIT_DELAY)),
                c.getBoolean("no-attack-cooldown", false),
                c.getBoolean("no-shield-delay", false),
                c.getBoolean("allow-player-toggle", true));
    }

    public void save(ConfigurationSection c) {
        c.set("enabled", enabled);
        c.set("hit-delay-ticks", hitDelayTicks);
        c.set("no-attack-cooldown", noAttackCooldown);
        c.set("no-shield-delay", noShieldDelay);
        c.set("allow-player-toggle", allowPlayerToggle);
    }

    public static int clampHitDelay(int ticks) {
        return Math.max(0, Math.min(VANILLA_HIT_DELAY, ticks));
    }

    /**
     * No-damage ticks to give a target one tick after a player's hit, so its next full hit can
     * land {@code delay} ticks after the last one. Hits only count fully once the target's
     * no-damage ticks drop to half the maximum, so we set it to half + delay. Never raised.
     */
    public static int noDamageTicksAfterHit(int current, int maximum, int delay) {
        return Math.max(0, Math.min(current, maximum / 2 + clampHitDelay(delay)));
    }

    /** "on"/"off" (and a few common spellings) → value; anything else → empty. */
    public static Optional<Boolean> parseOnOff(String s) {
        return switch (s.toLowerCase(Locale.ROOT)) {
            case "on", "true", "yes", "enable", "enabled" -> Optional.of(true);
            case "off", "false", "no", "disable", "disabled" -> Optional.of(false);
            default -> Optional.empty();
        };
    }

    public CombatRules withEnabled(boolean v) {
        return new CombatRules(v, hitDelayTicks, noAttackCooldown, noShieldDelay, allowPlayerToggle);
    }

    public CombatRules withHitDelay(int v) {
        return new CombatRules(enabled, clampHitDelay(v), noAttackCooldown, noShieldDelay, allowPlayerToggle);
    }

    public CombatRules withNoAttackCooldown(boolean v) {
        return new CombatRules(enabled, hitDelayTicks, v, noShieldDelay, allowPlayerToggle);
    }

    public CombatRules withNoShieldDelay(boolean v) {
        return new CombatRules(enabled, hitDelayTicks, noAttackCooldown, v, allowPlayerToggle);
    }

    public CombatRules withAllowPlayerToggle(boolean v) {
        return new CombatRules(enabled, hitDelayTicks, noAttackCooldown, noShieldDelay, v);
    }
}
