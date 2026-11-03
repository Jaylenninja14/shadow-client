package com.shadowclient.combat;

import io.papermc.paper.datacomponent.DataComponentTypes;
import io.papermc.paper.datacomponent.item.BlocksAttacks;
import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Optional;
import org.bukkit.Bukkit;
import org.bukkit.NamespacedKey;
import org.bukkit.attribute.Attribute;
import org.bukkit.attribute.AttributeInstance;
import org.bukkit.command.Command;
import org.bukkit.command.CommandSender;
import org.bukkit.entity.LivingEntity;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.entity.EntityDamageByEntityEvent;
import org.bukkit.event.player.PlayerJoinEvent;
import org.bukkit.event.player.PlayerQuitEvent;
import org.bukkit.inventory.EquipmentSlot;
import org.bukkit.inventory.ItemStack;
import org.bukkit.persistence.PersistentDataType;
import org.bukkit.plugin.java.JavaPlugin;

/**
 * Server-side combat timing for your own server, every rule toggleable in game.
 * Everything is applied by the server, so all clients see the same rules:
 *  - hit delay: how soon a target takes the next full hit from a player (vanilla 10 ticks)
 *  - no attack cooldown: attack speed is raised so weapons are always fully charged; spear
 *    jabs (and Lunge) are limited by this charge, see Player#cannotAttackWithItem
 *  - no shield delay: held shields get block_delay_seconds 0 (the original is kept on the item
 *    and restored when the rule or the player's tweaks are turned off)
 */
public final class ShadowCombatPlugin extends JavaPlugin implements Listener {

    private CombatRules rules;
    private NamespacedKey optOutKey;
    private NamespacedKey shieldKey;

    @Override
    public void onEnable() {
        carryOverOldConfig();
        saveDefaultConfig();
        optOutKey = new NamespacedKey(this, "opted_out");
        shieldKey = new NamespacedKey(this, "shield_delay");
        rules = CombatRules.from(getConfig());
        getServer().getPluginManager().registerEvents(this, this);
        Bukkit.getScheduler().runTaskTimer(this, () -> Bukkit.getOnlinePlayers().forEach(this::applyShields), 5L, 5L);
        Bukkit.getOnlinePlayers().forEach(this::apply);
        getLogger().info("Rules: " + describe());
    }

    /** The plugin used to be called SunnyCombat: its settings are copied over once. */
    private void carryOverOldConfig() {
        File config = new File(getDataFolder(), "config.yml");
        File old = new File(getDataFolder().getParentFile(), "SunnyCombat/config.yml");
        if (config.exists() || !old.isFile()) return;
        try {
            Files.createDirectories(getDataFolder().toPath());
            Files.copy(old.toPath(), config.toPath());
            getLogger().info("Copied your SunnyCombat settings (the plugin's old name).");
        } catch (IOException e) {
            getLogger().warning("Couldn't copy the old SunnyCombat settings: " + e.getMessage());
        }
    }

    @Override
    public void onDisable() {
        // Leave nothing behind in player data or on items.
        for (Player p : Bukkit.getOnlinePlayers()) {
            restore(p);
        }
    }

    // ------------------------------------------------------------------ rules per player

    private boolean optedOut(Player p) {
        return rules.allowPlayerToggle() && p.getPersistentDataContainer().has(optOutKey, PersistentDataType.BOOLEAN);
    }

    private boolean active(Player p) {
        return rules.enabled() && !optedOut(p);
    }

    private void apply(Player p) {
        applyAttackSpeed(p, active(p) && rules.noAttackCooldown());
        applyShields(p);
    }

    private void restore(Player p) {
        applyAttackSpeed(p, false);
        fixShield(p, EquipmentSlot.HAND, false);
        fixShield(p, EquipmentSlot.OFF_HAND, false);
    }

    private void applyAttackSpeed(Player p, boolean noCooldown) {
        AttributeInstance a = p.getAttribute(Attribute.ATTACK_SPEED);
        if (a == null) return;
        if (noCooldown) {
            a.setBaseValue(CombatRules.NO_COOLDOWN_ATTACK_SPEED);
        } else if (a.getBaseValue() == CombatRules.NO_COOLDOWN_ATTACK_SPEED) {
            a.setBaseValue(a.getDefaultValue()); // only undo what we set
        }
    }

    private void applyShields(Player p) {
        boolean want = active(p) && rules.noShieldDelay();
        fixShield(p, EquipmentSlot.HAND, want);
        fixShield(p, EquipmentSlot.OFF_HAND, want);
    }

    private void fixShield(Player p, EquipmentSlot slot, boolean noDelay) {
        if (p.hasActiveItem()) return; // never swap the item while it's being used
        ItemStack item = p.getInventory().getItem(slot);
        if (item.isEmpty() || !item.hasData(DataComponentTypes.BLOCKS_ATTACKS)) return;
        BlocksAttacks current = item.getData(DataComponentTypes.BLOCKS_ATTACKS);
        Float original = item.getPersistentDataContainer().get(shieldKey, PersistentDataType.FLOAT);
        if (noDelay && original == null && current.blockDelaySeconds() > 0f) {
            ItemStack copy = item.clone();
            copy.setData(DataComponentTypes.BLOCKS_ATTACKS, withDelay(current, 0f));
            copy.editPersistentDataContainer(pdc -> pdc.set(shieldKey, PersistentDataType.FLOAT, current.blockDelaySeconds()));
            p.getInventory().setItem(slot, copy);
        } else if (!noDelay && original != null) {
            ItemStack copy = item.clone();
            copy.setData(DataComponentTypes.BLOCKS_ATTACKS, withDelay(current, original));
            copy.editPersistentDataContainer(pdc -> pdc.remove(shieldKey));
            p.getInventory().setItem(slot, copy);
        }
    }

    private static BlocksAttacks withDelay(BlocksAttacks b, float delay) {
        BlocksAttacks.Builder out = BlocksAttacks.blocksAttacks()
                .blockDelaySeconds(delay)
                .disableCooldownScale(b.disableCooldownScale())
                .damageReductions(b.damageReductions())
                .itemDamage(b.itemDamage());
        if (b.bypassedBy() != null) out.bypassedBy(b.bypassedBy());
        if (b.blockSound() != null) out.blockSound(b.blockSound());
        if (b.disableSound() != null) out.disableSound(b.disableSound());
        return out.build();
    }

    // ------------------------------------------------------------------ events

    @EventHandler
    public void onJoin(PlayerJoinEvent e) {
        apply(e.getPlayer());
    }

    @EventHandler
    public void onQuit(PlayerQuitEvent e) {
        // saved player data stays vanilla, even if the plugin is removed later
        restore(e.getPlayer());
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onHit(EntityDamageByEntityEvent e) {
        if (!(e.getDamager() instanceof Player attacker) || !(e.getEntity() instanceof LivingEntity target)) return;
        if (!active(attacker) || rules.hitDelayTicks() >= CombatRules.VANILLA_HIT_DELAY) return;
        int delay = rules.hitDelayTicks();
        Bukkit.getScheduler().runTask(this, () -> {
            if (target.isValid()) {
                target.setNoDamageTicks(CombatRules.noDamageTicksAfterHit(target.getNoDamageTicks(), target.getMaximumNoDamageTicks(), delay));
            }
        });
    }

    // ------------------------------------------------------------------ commands

    private String describe() {
        return String.format(Locale.ROOT, "enabled=%s, hitdelay=%d ticks%s, nocooldown=%s, noshielddelay=%s, playertoggle=%s",
                onOff(rules.enabled()), rules.hitDelayTicks(), rules.hitDelayTicks() == CombatRules.VANILLA_HIT_DELAY ? " (vanilla)" : "",
                onOff(rules.noAttackCooldown()), onOff(rules.noShieldDelay()), onOff(rules.allowPlayerToggle()));
    }

    private static String onOff(boolean b) {
        return b ? "on" : "off";
    }

    private void update(CombatRules next) {
        rules = next;
        rules.save(getConfig());
        saveConfig();
        Bukkit.getOnlinePlayers().forEach(this::apply);
    }

    @Override
    public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
        if (command.getName().equalsIgnoreCase("combat")) {
            return onCombat(sender, args);
        }
        if (args.length == 0 || args[0].equalsIgnoreCase("status")) {
            sender.sendMessage("ShadowCombat: " + describe());
            return true;
        }
        String what = args[0].toLowerCase(Locale.ROOT);
        if (what.equals("reload")) {
            reloadConfig();
            update(CombatRules.from(getConfig()));
            sender.sendMessage("ShadowCombat reloaded: " + describe());
            return true;
        }
        if (args.length < 2) return false;
        if (what.equals("hitdelay")) {
            int ticks;
            try {
                ticks = Integer.parseInt(args[1]);
            } catch (NumberFormatException ex) {
                sender.sendMessage("Hit delay is a number of ticks from 0 to 10 (vanilla 10).");
                return true;
            }
            if (ticks < 0 || ticks > CombatRules.VANILLA_HIT_DELAY) {
                sender.sendMessage("Hit delay must be 0 to 10 ticks (vanilla 10).");
                return true;
            }
            update(rules.withHitDelay(ticks));
        } else {
            Optional<Boolean> v = CombatRules.parseOnOff(args[1]);
            if (v.isEmpty()) return false;
            switch (what) {
                case "enabled" -> update(rules.withEnabled(v.get()));
                case "nocooldown" -> update(rules.withNoAttackCooldown(v.get()));
                case "noshielddelay" -> update(rules.withNoShieldDelay(v.get()));
                case "playertoggle" -> update(rules.withAllowPlayerToggle(v.get()));
                default -> {
                    return false;
                }
            }
        }
        sender.sendMessage("ShadowCombat: " + describe());
        return true;
    }

    private boolean onCombat(CommandSender sender, String[] args) {
        if (!(sender instanceof Player p)) {
            sender.sendMessage("Only players can use /combat.");
            return true;
        }
        if (!rules.allowPlayerToggle()) {
            sender.sendMessage("The server owner has turned off per-player toggles.");
            return true;
        }
        if (args.length == 0) {
            p.sendMessage("Combat tweaks are " + (active(p) ? "on" : "off") + " for you. Use /combat on or /combat off.");
            return true;
        }
        Optional<Boolean> v = CombatRules.parseOnOff(args[0]);
        if (v.isEmpty()) return false;
        if (v.get()) {
            p.getPersistentDataContainer().remove(optOutKey);
        } else {
            p.getPersistentDataContainer().set(optOutKey, PersistentDataType.BOOLEAN, true);
        }
        apply(p);
        p.sendMessage("Combat tweaks " + (v.get() ? "on" : "off") + " for you." + (rules.enabled() ? "" : " (They are switched off server-wide right now.)"));
        return true;
    }

    @Override
    public List<String> onTabComplete(CommandSender sender, Command command, String alias, String[] args) {
        List<String> options = new ArrayList<>();
        if (command.getName().equalsIgnoreCase("combat")) {
            if (args.length == 1) options.addAll(List.of("on", "off"));
        } else if (args.length == 1) {
            options.addAll(List.of("status", "enabled", "hitdelay", "nocooldown", "noshielddelay", "playertoggle", "reload"));
        } else if (args.length == 2) {
            if (args[0].equalsIgnoreCase("hitdelay")) options.addAll(List.of("0", "2", "5", "10"));
            else if (!args[0].equalsIgnoreCase("status") && !args[0].equalsIgnoreCase("reload")) options.addAll(List.of("on", "off"));
        }
        String prefix = args.length == 0 ? "" : args[args.length - 1].toLowerCase(Locale.ROOT);
        options.removeIf(o -> !o.startsWith(prefix));
        return options;
    }
}
