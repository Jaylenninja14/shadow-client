package com.shadowclient.combat;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Optional;
import org.bukkit.configuration.file.YamlConfiguration;
import org.junit.jupiter.api.Test;

class CombatRulesTest {

    @Test
    void hitDelayMath() {
        // vanilla target: 20 max no-damage ticks, full hits again once it drops to 10
        assertEquals(10, CombatRules.noDamageTicksAfterHit(19, 20, 0), "delay 0: next full hit right away");
        assertEquals(15, CombatRules.noDamageTicksAfterHit(19, 20, 5));
        assertEquals(19, CombatRules.noDamageTicksAfterHit(19, 20, 10), "vanilla delay leaves it unchanged");
        assertEquals(7, CombatRules.noDamageTicksAfterHit(7, 20, 5), "never raised");
        assertEquals(10, CombatRules.noDamageTicksAfterHit(19, 20, -3), "clamped to 0");
        assertEquals(19, CombatRules.noDamageTicksAfterHit(19, 20, 99), "clamped to vanilla");
        assertEquals(0, CombatRules.noDamageTicksAfterHit(0, 0, 0));
    }

    @Test
    void configRoundTripAndClamping() {
        YamlConfiguration c = new YamlConfiguration();
        CombatRules defaults = CombatRules.from(c);
        assertTrue(defaults.enabled());
        assertEquals(10, defaults.hitDelayTicks());
        assertFalse(defaults.noAttackCooldown());
        assertFalse(defaults.noShieldDelay());
        assertTrue(defaults.allowPlayerToggle());

        c.set("hit-delay-ticks", 42);
        assertEquals(10, CombatRules.from(c).hitDelayTicks());

        CombatRules custom = defaults.withHitDelay(3).withNoAttackCooldown(true).withNoShieldDelay(true).withAllowPlayerToggle(false).withEnabled(false);
        YamlConfiguration saved = new YamlConfiguration();
        custom.save(saved);
        assertEquals(custom, CombatRules.from(saved));
    }

    @Test
    void onOffParsing() {
        assertEquals(Optional.of(true), CombatRules.parseOnOff("ON"));
        assertEquals(Optional.of(false), CombatRules.parseOnOff("off"));
        assertEquals(Optional.of(true), CombatRules.parseOnOff("enable"));
        assertEquals(Optional.empty(), CombatRules.parseOnOff("maybe"));
    }
}
