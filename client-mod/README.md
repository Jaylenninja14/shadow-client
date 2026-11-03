# Shadow Menu (Fabric mod, Minecraft 1.21.11)

Shadow Client's in-game client. Since 2.0 it does everything itself, so AxolotlClient is no longer
needed (Fabric marks the two as conflicting: keeping both shows HUD elements twice and binds the
same keys).

## What's in it

- **Mod menu** (`ui/ModMenuScreen`, **Right Shift**, in game and on the title screen): categories on
  the left (All, Favourites, New, HUD, PvP, Hypixel, Visual, Performance, Game), search at the top
  (just start typing), **Edit HUD**, and a grid of cards with icon, ♥ favourite, ⚙ settings and an
  animated Enabled/Disabled bar (or a value to click through: left click forward, right click back).
  Cards fade in when the menu opens and light up under the mouse.
- **Settings pages** (`ui/SettingsPage`): every Shadow module has one, with switches, sliders,
  colour pickers (saturation/brightness square, hue, alpha, chroma, presets, hex), choices,
  keybinds (they edit the real key mapping, so Controls shows the same key), text fields and
  buttons; groups get headings; "Reset to defaults" asks once more.
- **Client Settings** (bottom of the sidebar, `ui/Theme`): Dark / Midnight / Black / Light theme,
  accent colour (surfaces are tinted towards it, like the launcher), card size, blur behind the
  menu, animations and their speed, click sounds, the menu key, HUD editor options.
- **HUD** (`hud/`): 22 movable modules drawn by Shadow Menu: FPS, CPS, Keystrokes, Ping, Armor
  Status, Potion Effects, Coordinates, Direction (compass strip), Clock, Speed, Reach Display,
  Combo Display, Sprint Status, Memory, TPS, Server Address, Pack Display, Arrow Counter, Scoreboard
  (redrawn, can hide the red numbers), Boss Bar (Minecraft's, moved), Player Model and the Shadow HUD
  stack. Text modules share style options: text colour (or chroma), text shadow, [brackets],
  background (colour, rounded corners, padding, same-size boxes) and border. Positions are stored as
  a fraction of the free space, so a module glued to an edge or the centre stays there when the
  window or GUI scale changes. Potion Effects can hide Minecraft's own effect icons.
- **HUD editor** (`ui/HudEditorScreen`): drag to move (snaps to the screen edges, the centre and
  other modules, with guide lines; Shift places freely), drag the corner or scroll to resize,
  right-click for the module's settings page, arrow keys nudge (Shift: 10 px), Delete hides, R
  resets one module; Snap / Grid / Reset All / Mod Menu buttons in the middle.
- **Native modules** (`feature/`, each with its own mixin): Zoom (C, smooth, scroll to zoom
  further, slower mouse, optional cinematic camera and hidden hand), Freelook (V, hold or toggle,
  back/front/first-person view), Motion Blur (own post effect, 10–90%), Custom Crosshair (7 shapes,
  size, gap, thickness, outline, target colour), Fullbright (with a toggle key), Time Changer, Block
  Outline (colour and thickness), Hit Color, No Hurt Cam, Low Fire, Low Shield, Minimal View Bob,
  Remove Vignette, No Rain, Nametags (own nametag in third person, background darkness), TNT Timer,
  Hide Beacon Beams, Hit Particles, Scrollable Tooltips (only tooltips bigger than the screen take
  the mouse wheel), Confirm Disconnect, AutoGG (+ gf), AutoGLHF, AutoTip, AutoBoop, Nick Hider
  (your/others' names in chat, tab list and nametags; default skins), Chat Heads (the sender's head
  in front of chat lines, drawn with Minecraft's own player-head text object), Saturation Display
  (saturation under the hunger bar, held food's refill flashing), Ping in Tab (coloured numbers in
  a wider ping column) and Copy Screenshots (F2 also copies to the clipboard, Windows).
- **Mods** (sidebar, and the Mods chip on the title screen; `ui/ModsPage`): every installed mod with
  its icon, version, authors and description, and Settings for mods that offer a settings screen
  through Mod Menu's entrypoint. Shadow Client no longer installs Mod Menu, Chat Heads, AppleSkin or
  Better Ping Display: Shadow Menu does those itself.
- **Shadow Client title screen** (`ui/ShadowTitleScreen`): always shown, not a setting. `MinecraftMixin`
  swaps any `TitleScreen` (Minecraft's, or a mod's built on it) for it when it's opened, and a tick
  check catches anything that slipped past. Shadow panorama, logo, themed buttons that slide in
  (Singleplayer, Multiplayer, Shadow Menu, Options, Quit), Realms / Mods (when Mod Menu is installed) /
  Language / Accessibility chips, version and copyright in the corners.
- Kept from 1.x: Shadow HUD lines and PvP warnings, PvP tracker, waypoints, Auto Text, the overlay
  texture packs and Shadow Background, chat tweaks, Minecraft-option cards (`GameModules`, FPS Boost)
  and the window title "Shadow Client 1.21.11".

Not carried over from AxolotlClient, because they used its online services or Hypixel's API:
Level Head, the Bedwars overlay, client badges, Discord status and credits music. Hit colour on
armor isn't done either.

## Settings file

Everything lives in `config/shadowmenu.json`: favourites, FPS Boost's saved values, tweaks,
waypoints, Auto Text, and `"modules"` → id → settings for every Shadow module (`"hud.fps"`,
`"zoom"`, `"client"` for the theme, `"hud"` for editor options). Changes are written half a second
after they happen and when the game closes. The launcher's HUD chooser edits only `enabled` (and
the CPS module's `right`).

## Build

Needs JDK 21+ (tested setup: JDK 26). No other jars needed.

```bash
gradlew.bat build
```

Output: `build/libs/shadow-menu-2.0.1.jar`. Icons are generated by `tools/IconGen.java`
(`java tools/IconGen.java src/main/resources/assets/shadowmenu/textures/gui/icon`).

## Testing without touching the game

`gradlew.bat runClient` with the environment variable `SHADOWMENU_SELFTEST=1` runs a screenshot
tour (`SelfTest`): title screen, menu, two settings pages, then a flat creative test world with a
PvP HUD, the HUD editor and the in-game menu, saved as `run/screenshots/shadow-*.png`; it logs
"Shadow Menu self-test done" and quits. Add `JAVA_TOOL_OPTIONS=-Dmixin.debug.countInjections=true`
to have Mixin warn about any injection that found no target.
