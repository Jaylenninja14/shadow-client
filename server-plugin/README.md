# ShadowCombat (Paper plugin)

Toggleable combat timing for a Paper server you run. The server decides hit delay, attack
cooldown and shield delay, so the rules live here. Every client follows them automatically,
including Shadow Client, vanilla and other launchers. Nothing client-side is needed or used.

Built against `paper-api 26.3.build.227-stable` (Java 25 bytecode).

## Rules

| Command (op) | Effect |
| --- | --- |
| `/shadowcombat` | Show the current rules |
| `/shadowcombat hitdelay <0-10>` | Ticks before a target takes the next full hit from a player (vanilla 10). Only player melee hits; fire, lava, falls keep vanilla timing. |
| `/shadowcombat nocooldown on\|off` | Attack speed raised so weapons are always fully charged: full-damage sword/axe hits, and spear jabs (Lunge triggers on each jab) as fast as you click. In 26.x the jab is gated by `minimum_attack_charge` against the attack-speed charge (`Player#cannotAttackWithItem`). |
| `/shadowcombat noshielddelay on\|off` | Held shields get `block_delay_seconds` 0. The original value is stored on the item and restored when the rule is off. |
| `/shadowcombat enabled on\|off` | Master switch |
| `/shadowcombat playertoggle on\|off` | Allow `/combat on\|off` for players |
| `/shadowcombat reload` | Re-read `config.yml` |
| `/combat on\|off` (everyone) | Turn the tweaks off/on for yourself (stored in your player data) |

Defaults: master on, everything else vanilla. Changes are saved to `plugins/ShadowCombat/config.yml`.

The plugin leaves no lasting changes: it restores attack speed when a player leaves and when the
plugin is disabled, and puts shields back to their original delay when the rule is turned off.

## Build

```bash
JAVA_HOME="C:/Program Files/Java/jdk-26.0.1" mvn -B package
```

Output: `target/ShadowCombat-1.0.0.jar`. The launcher embeds a copy
(`crates/pc-core/data/server/`); refresh it after changing the plugin.

## Install

Shadow Client → Features → "For your server: ShadowCombat" → **Install on a server…**, pick the
server folder (or its `plugins` folder), restart the server. Or copy the jar into `plugins/` by hand.
