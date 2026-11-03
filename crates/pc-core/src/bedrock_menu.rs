//! Bedrock's mod menu. Bedrock can't load mods, so its "modules" are the two things a launcher
//! can safely change from outside the game:
//!
//! * Bedrock's own settings in the account's `minecraftpe/options.txt` (VSync, render distance,
//!   FOV, hurt cam, …). The file as it was before Shadow Client first changed it is kept, plus
//!   the version before the latest change.
//! * A small resource pack Shadow Client builds ("Shadow Client Modules": custom crosshair, no
//!   pumpkin overlay, …) and switches on in the account's global resource packs.
//!
//! Bedrock rewrites options.txt when it closes, so nothing is changed while it's running.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{util, Error, Result};

pub const PACK_UUID: &str = "5ad0c1e7-8f2b-4c6a-9d3e-7b1f0a4c2e91";
const PACK_MODULE_UUID: &str = "c3e8b6a2-1d4f-4f7a-8e2b-9a6c5d3f1b07";
const PACK_FOLDER: &str = "shadow_client_modules";
const PACK_ICON: &[u8] = include_bytes!("../../../src-tauri/icons/128x128.png");
const STATE_FILE: &str = "bedrock-modules.json";

/// Where a module's value lives.
enum Kind {
    /// options.txt key with its on/off values
    Toggle { key: &'static str, on: &'static str, off: &'static str },
    /// options.txt key with the values offered (value, label)
    Cycle { key: &'static str, values: &'static [(&'static str, &'static str)] },
    /// a part of Shadow Client's pack, on or off
    PackToggle,
    /// a part of Shadow Client's pack with a choice ("default" leaves Bedrock's own)
    PackCycle { values: &'static [(&'static str, &'static str)] },
}

struct Def {
    id: &'static str,
    name: &'static str,
    tab: &'static str,
    icon: &'static str,
    about: &'static str,
    kind: Kind,
}

const ON_OFF: (&str, &str) = ("1", "0");

const fn toggle(key: &'static str) -> Kind {
    Kind::Toggle { key, on: ON_OFF.0, off: ON_OFF.1 }
}

/// "On" means the setting's 0 (e.g. Paper Doll on = gfx_hidepaperdoll 0).
const fn inverted(key: &'static str) -> Kind {
    Kind::Toggle { key, on: ON_OFF.1, off: ON_OFF.0 }
}

const RENDER_DISTANCES: &[(&str, &str)] = &[
    ("96", "6 chunks"), ("128", "8 chunks"), ("192", "12 chunks"), ("256", "16 chunks"), ("384", "24 chunks"), ("512", "32 chunks"),
    ("768", "48 chunks"), ("1024", "64 chunks"), ("1280", "80 chunks"), ("1536", "96 chunks"),
];
const FOVS: &[(&str, &str)] = &[("60", "60"), ("70", "70"), ("80", "80"), ("90", "90"), ("100", "100"), ("110", "110")];
const MSAA: &[(&str, &str)] = &[("1", "Off"), ("2", "2x"), ("4", "4x"), ("8", "8x"), ("16", "16x")];
const GAMMA: &[(&str, &str)] = &[("0.25", "25%"), ("0.5", "50%"), ("0.75", "75%"), ("1", "100%")];
const CROSSHAIRS: &[(&str, &str)] = &[("default", "Bedrock's"), ("dot", "Dot"), ("plus", "Plus"), ("circle", "Circle"), ("x", "X")];
const CROSSHAIR_COLOURS: &[(&str, &str)] = &[("white", "White"), ("purple", "Purple"), ("green", "Green"), ("red", "Red"), ("aqua", "Aqua")];
const VOLUME: &[(&str, &str)] = &[("0", "Off"), ("0.25", "25%"), ("0.5", "50%"), ("0.75", "75%"), ("1", "100%")];
const PERCENT_OFF: &[(&str, &str)] = VOLUME;
const PERCENT: &[(&str, &str)] = &[("0.25", "25%"), ("0.5", "50%"), ("0.75", "75%"), ("1", "100%")];
const SENSITIVITY: &[(&str, &str)] = &[("0.2", "20%"), ("0.3", "30%"), ("0.4", "40%"), ("0.5", "50%"), ("0.6", "60%"), ("0.7", "70%"), ("0.8", "80%"), ("1", "100%")];
const PERSPECTIVES: &[(&str, &str)] = &[("0", "First Person"), ("1", "Third Person Back"), ("2", "Third Person Front")];
const PANORAMA: &[(&str, &str)] = &[("0", "Still"), ("0.5", "Slow"), ("1", "Normal")];
const GUI_SCALES: &[(&str, &str)] = &[("-2", "Smallest"), ("-1", "Smaller"), ("0", "Normal")];
/// 0 is Bedrock's "Unlimited".
const MAX_FPS: &[(&str, &str)] = &[("0", "Unlimited"), ("30", "30"), ("60", "60"), ("120", "120"), ("144", "144"), ("165", "165"), ("240", "240")];
const UI_PROFILES: &[(&str, &str)] = &[("0", "Classic"), ("1", "Pocket")];
const CHAT_FONTS: &[(&str, &str)] = &[("0", "Mojangles"), ("1", "Noto Sans")];
const CHAT_SIZES: &[(&str, &str)] = &[("10", "10"), ("12", "12"), ("14", "14"), ("16", "16"), ("18", "18")];
/// Bedrock stores chat colours as formatting codes.
const CHAT_COLOURS: &[(&str, &str)] = &[("§f", "White"), ("§7", "Grey"), ("§e", "Yellow"), ("§6", "Gold"), ("§b", "Aqua"), ("§a", "Green"), ("§d", "Pink")];
const MENTION_COLOURS: &[(&str, &str)] = &[("§e", "Yellow"), ("§6", "Gold"), ("§b", "Aqua"), ("§a", "Green"), ("§d", "Pink"), ("§c", "Red"), ("§f", "White")];
const GLINT_COLOURS: &[(&str, &str)] = &[("default", "Bedrock's"), ("purple", "Purple"), ("red", "Red"), ("aqua", "Aqua"), ("gold", "Gold"), ("green", "Green"), ("none", "None")];
/// Settings Bedrock keeps twice (simple and advanced video settings): both get the value.
const MIRRORS: &[(&str, &str)] = &[("gfx_max_framerate", "gfx_max_framerate_advanced")];
const PANORAMA_FACES: [&[u8]; 6] = [
    include_bytes!("../data/bedrock/panorama_0.png"),
    include_bytes!("../data/bedrock/panorama_1.png"),
    include_bytes!("../data/bedrock/panorama_2.png"),
    include_bytes!("../data/bedrock/panorama_3.png"),
    include_bytes!("../data/bedrock/panorama_4.png"),
    include_bytes!("../data/bedrock/panorama_5.png"),
];
const PANORAMA_PATHS: [&str; 6] = [
    "textures/ui/panorama_0.png",
    "textures/ui/panorama_1.png",
    "textures/ui/panorama_2.png",
    "textures/ui/panorama_3.png",
    "textures/ui/panorama_4.png",
    "textures/ui/panorama_5.png",
];
/// Bedrock's stained glass colours (light grey is "silver") with the frame colour used for each.
const GLASS_COLOURS: &[(&str, [u8; 3])] = &[
    ("textures/blocks/glass_white.png", [235, 235, 235]), ("textures/blocks/glass_orange.png", [235, 130, 40]),
    ("textures/blocks/glass_magenta.png", [200, 80, 200]), ("textures/blocks/glass_light_blue.png", [110, 170, 230]),
    ("textures/blocks/glass_yellow.png", [235, 220, 60]), ("textures/blocks/glass_lime.png", [130, 210, 40]),
    ("textures/blocks/glass_pink.png", [240, 150, 180]), ("textures/blocks/glass_gray.png", [90, 90, 90]),
    ("textures/blocks/glass_silver.png", [160, 160, 160]), ("textures/blocks/glass_cyan.png", [40, 150, 160]),
    ("textures/blocks/glass_purple.png", [130, 60, 190]), ("textures/blocks/glass_blue.png", [50, 70, 190]),
    ("textures/blocks/glass_brown.png", [120, 80, 50]), ("textures/blocks/glass_green.png", [80, 110, 40]),
    ("textures/blocks/glass_red.png", [170, 50, 45]), ("textures/blocks/glass_black.png", [30, 30, 30]),
];

const DEFS: &[Def] = &[
    // performance
    Def { id: "vsync", name: "VSync", tab: "performance", icon: "monitor", about: "Off removes the frame cap and input delay VSync adds.", kind: toggle("gfx_vsync") },
    Def { id: "render-distance", name: "Render Distance", tab: "performance", icon: "mountain", about: "How far chunks are drawn. Bedrock lowers it to what this PC allows.", kind: Kind::Cycle { key: "gfx_viewdistance", values: RENDER_DISTANCES } },
    Def { id: "improved-input", name: "Improved Input Response", tab: "performance", icon: "mouse-pointer-click", about: "Bedrock's lower-latency input handling.", kind: toggle("ctrl_improvedInputResponse") },
    Def { id: "multithreaded", name: "Multithreaded Rendering", tab: "performance", icon: "cpu", about: "Spreads rendering over more CPU cores.", kind: toggle("gfx_multithreaded_renderer") },
    Def { id: "antialiasing", name: "Antialiasing", tab: "performance", icon: "sparkles", about: "Smoother edges; lower is faster.", kind: Kind::Cycle { key: "gfx_msaa", values: MSAA } },
    Def { id: "fancy-leaves", name: "Fancy Leaves", tab: "performance", icon: "leaf", about: "See-through leaves. Off is faster.", kind: toggle("gfx_transparentleaves") },
    Def { id: "smooth-lighting", name: "Smooth Lighting", tab: "performance", icon: "lightbulb", about: "Soft shadows on blocks.", kind: toggle("gfx_smoothlighting") },
    Def { id: "beautiful-skies", name: "Beautiful Skies", tab: "performance", icon: "sunset", about: "Fancy sky colours.", kind: toggle("gfx_fancyskies") },
    Def { id: "clouds", name: "Clouds", tab: "performance", icon: "cloud", about: "Render clouds.", kind: toggle("gfx_toggleclouds") },
    Def { id: "fancy-bubbles", name: "Fancy Bubbles", tab: "performance", icon: "droplets", about: "Bubble particles under water.", kind: toggle("gfx_bubble_particles") },
    // PvP
    Def { id: "fov", name: "Field of View", tab: "pvp", icon: "scan-eye", about: "Wider sees more around you.", kind: Kind::Cycle { key: "gfx_field_of_view", values: FOVS } },
    Def { id: "dynamic-fov", name: "Dynamic FOV", tab: "pvp", icon: "zoom-in", about: "Sprinting and effects change your FOV.", kind: toggle("gfx_field_of_view_toggle") },
    Def { id: "hurt-cam", name: "Hurt Cam", tab: "pvp", icon: "heart-crack", about: "The camera jolt when you take damage.", kind: toggle("gfx_damagebobbing") },
    Def { id: "camera-shake", name: "Camera Shake", tab: "pvp", icon: "vibrate", about: "Screen shake from explosions and effects.", kind: toggle("camera_shake") },
    Def { id: "auto-sprint", name: "Auto Sprint", tab: "pvp", icon: "footprints", about: "Sprint whenever you move forward (Bedrock's \"Sprint on movement\").", kind: toggle("ctrl_sprintonmovement") },
    Def { id: "toggle-crouch", name: "Toggle Crouch", tab: "pvp", icon: "arrow-down-to-line", about: "Press crouch once instead of holding it.", kind: toggle("ctrl_togglecrouch_mouse") },
    Def { id: "crosshair", name: "Custom Crosshair", tab: "pvp", icon: "crosshair", about: "A cleaner crosshair from Shadow Client's pack.", kind: Kind::PackCycle { values: CROSSHAIRS } },
    Def { id: "no-fire", name: "No Fire Overlay", tab: "pvp", icon: "flame", about: "Hides the fire covering your screen while you burn (flames on burning mobs too).", kind: Kind::PackToggle },
    // visual
    Def { id: "view-bobbing", name: "View Bobbing", tab: "visual", icon: "move-vertical", about: "The camera sway while walking.", kind: toggle("gfx_viewbobbing") },
    Def { id: "brightness", name: "Brightness", tab: "visual", icon: "sun", about: "Gamma; 100% is Bedrock's brightest.", kind: Kind::Cycle { key: "gfx_gamma", values: GAMMA } },
    Def { id: "paper-doll", name: "Paper Doll", tab: "visual", icon: "person-standing", about: "Your character in the corner of the screen.", kind: inverted("gfx_hidepaperdoll") },
    Def { id: "outline-selection", name: "Outline Selection", tab: "visual", icon: "square-dashed", about: "Outline the block you look at instead of highlighting it.", kind: toggle("gfx_classic_box_selection") },
    Def { id: "player-names", name: "Player Names", tab: "visual", icon: "tag", about: "Name tags above players.", kind: toggle("gfx_ingame_player_names") },
    Def { id: "no-pumpkin", name: "No Pumpkin Overlay", tab: "visual", icon: "eye", about: "See normally while wearing a carved pumpkin.", kind: Kind::PackToggle },
    Def { id: "clean-glass", name: "Clean Glass", tab: "visual", icon: "square", about: "Glass with just a thin frame (Shadow Client's pack).", kind: Kind::PackToggle },
    Def { id: "hide-end-flash", name: "Hide End Flash", tab: "visual", icon: "zap-off", about: "No sky flashes in the End.", kind: toggle("hide_endflash") },
    // game
    Def { id: "fullscreen", name: "Fullscreen", tab: "game", icon: "maximize", about: "Start Bedrock in fullscreen.", kind: toggle("gfx_fullscreen") },
    Def { id: "auto-jump", name: "Auto Jump", tab: "game", icon: "chevrons-up", about: "Jump up blocks automatically.", kind: toggle("ctrl_autojump_mouse") },
    Def { id: "hide-hand", name: "Hide Hand", tab: "game", icon: "hand", about: "Hide your hand and held item (screenshots).", kind: toggle("gfx_hidehand") },
    Def { id: "autosave-icon", name: "Autosave Icon", tab: "game", icon: "save", about: "Show the icon while the world saves.", kind: toggle("gfx_showautosaveicon") },
    // sound mixer: turn music, weather and ambience down to hear footsteps
    Def { id: "master-volume", name: "Master Volume", tab: "audio", icon: "volume-2", about: "Everything Bedrock plays.", kind: Kind::Cycle { key: "audio_main", values: VOLUME } },
    Def { id: "sound-volume", name: "Sound Effects", tab: "audio", icon: "audio-lines", about: "All sound effects together.", kind: Kind::Cycle { key: "audio_sound", values: VOLUME } },
    Def { id: "music-volume", name: "Music", tab: "audio", icon: "music", about: "Background music.", kind: Kind::Cycle { key: "audio_music", values: VOLUME } },
    Def { id: "ambient-volume", name: "Ambient Sounds", tab: "audio", icon: "waves", about: "Caves, wind and other ambience.", kind: Kind::Cycle { key: "audio_ambient", values: VOLUME } },
    Def { id: "block-volume", name: "Block Sounds", tab: "audio", icon: "blocks", about: "Breaking, placing and footsteps on blocks.", kind: Kind::Cycle { key: "audio_block", values: VOLUME } },
    Def { id: "hostile-volume", name: "Hostile Mob Sounds", tab: "audio", icon: "skull", about: "Zombies, skeletons, creepers…", kind: Kind::Cycle { key: "audio_hostile", values: VOLUME } },
    Def { id: "friendly-volume", name: "Friendly Mob Sounds", tab: "audio", icon: "rabbit", about: "Animals and villagers.", kind: Kind::Cycle { key: "audio_neutral", values: VOLUME } },
    Def { id: "player-volume", name: "Player Sounds", tab: "audio", icon: "users", about: "Other players: footsteps, hits, eating.", kind: Kind::Cycle { key: "audio_player", values: VOLUME } },
    Def { id: "weather-volume", name: "Weather Sounds", tab: "audio", icon: "cloud-rain", about: "Rain and thunder.", kind: Kind::Cycle { key: "audio_weather", values: VOLUME } },
    Def { id: "jukebox-volume", name: "Jukebox & Note Blocks", tab: "audio", icon: "disc-3", about: "Music discs and note blocks.", kind: Kind::Cycle { key: "audio_record", values: VOLUME } },
    Def { id: "tts-volume", name: "Text to Speech Volume", tab: "audio", icon: "speech", about: "Narrated chat and menus.", kind: Kind::Cycle { key: "audio_texttospeech", values: VOLUME } },
    // aim and controls
    Def { id: "sensitivity", name: "Mouse Sensitivity", tab: "pvp", icon: "mouse", about: "Bedrock's mouse sensitivity slider.", kind: Kind::Cycle { key: "ctrl_sensitivity2_mouse", values: SENSITIVITY } },
    Def { id: "spyglass-sensitivity", name: "Spyglass Sensitivity", tab: "pvp", icon: "telescope", about: "How fast you turn while zoomed in with a spyglass.", kind: Kind::Cycle { key: "ctrl_spyglassdamp_mouse", values: PERCENT } },
    Def { id: "invert-mouse", name: "Invert Mouse", tab: "game", icon: "arrow-up-down", about: "Moving the mouse up looks down.", kind: toggle("ctrl_invertmouse_mouse") },
    Def { id: "full-keyboard", name: "Full Keyboard Gameplay", tab: "game", icon: "keyboard", about: "Play without a mouse: aim and use items with keys.", kind: toggle("ctrl_fullkeyboardgameplay") },
    Def { id: "perspective", name: "Camera Perspective", tab: "game", icon: "camera", about: "The view Bedrock starts in (F5 in game).", kind: Kind::Cycle { key: "game_thirdperson", values: PERSPECTIVES } },
    Def { id: "pause-unfocused", name: "Pause When Unfocused", tab: "game", icon: "pause", about: "Open the pause menu when you alt-tab.", kind: toggle("pause_menu_on_focus_lost") },
    // more video
    Def { id: "frame-pacing", name: "Frame Pacing", tab: "performance", icon: "gauge", about: "Evens out frame times so motion stutters less.", kind: toggle("frame_pacing_enabled") },
    Def { id: "texel-aa", name: "Texel Antialiasing", tab: "performance", icon: "grid-3x3", about: "Smooths texture edges up close; off is a little faster.", kind: toggle("gfx_texel_aa_2") },
    Def { id: "upscaling", name: "Upscaling", tab: "performance", icon: "scaling", about: "Render at a lower resolution and scale up for more FPS.", kind: toggle("gfx_upscaling") },
    Def { id: "texture-streaming", name: "Texture Streaming", tab: "performance", icon: "image", about: "Loads textures as needed to save memory.", kind: toggle("gfx_texture_streaming") },
    // visual
    Def { id: "distortion", name: "Screen Distortion", tab: "visual", icon: "aperture", about: "Nausea and portal wobble; lower is calmer.", kind: Kind::Cycle { key: "screen_distortion", values: PERCENT_OFF } },
    Def { id: "darkness-pulse", name: "Darkness Pulsing", tab: "visual", icon: "moon", about: "How strongly the Darkness effect pulses.", kind: Kind::Cycle { key: "darkness_effect_modifier", values: PERCENT_OFF } },
    Def { id: "glint-strength", name: "Enchant Glint Strength", tab: "visual", icon: "sparkle", about: "How bright enchanted items shimmer.", kind: Kind::Cycle { key: "glint_strength", values: PERCENT_OFF } },
    Def { id: "glint-speed", name: "Enchant Glint Speed", tab: "visual", icon: "sparkles", about: "How fast the shimmer moves.", kind: Kind::Cycle { key: "glint_speed", values: PERCENT_OFF } },
    Def { id: "highlight-block", name: "Highlight Looked-at Block", tab: "visual", icon: "mouse-pointer-2", about: "Always show the outline of the block under the crosshair.", kind: toggle("gfx_alwayshighlighthoveringboxincrosshair") },
    Def { id: "screen-animations", name: "Screen Animations", tab: "visual", icon: "clapperboard", about: "Menu transitions.", kind: toggle("screen_animations") },
    Def { id: "panorama-speed", name: "Menu Panorama Speed", tab: "visual", icon: "rotate-cw", about: "How fast the main menu background turns.", kind: Kind::Cycle { key: "panorama_scroll_speed", values: PANORAMA } },
    Def { id: "shadow-background", name: "Shadow Background", tab: "visual", icon: "moon", about: "Shadow Client's night-sky panorama behind Bedrock's menus.", kind: Kind::PackToggle },
    Def { id: "clean-stained-glass", name: "Clean Stained Glass", tab: "pvp", icon: "palette", about: "Every stained glass colour with just a thin coloured frame.", kind: Kind::PackToggle },
    Def { id: "crosshair-colour", name: "Crosshair Colour", tab: "pvp", icon: "crosshair", about: "Colour of the Custom Crosshair.", kind: Kind::PackCycle { values: CROSSHAIR_COLOURS } },
    // HUD and chat
    Def { id: "hide-hud", name: "Hide HUD", tab: "hud", icon: "eye-off", about: "Hide the hotbar, hearts and other HUD.", kind: toggle("gfx_hidehud") },
    Def { id: "interface-opacity", name: "Interface Opacity", tab: "hud", icon: "layers", about: "How see-through the HUD is.", kind: Kind::Cycle { key: "gfx_interface_opacity", values: PERCENT } },
    Def { id: "hud-text-background", name: "HUD Text Background", tab: "hud", icon: "message-square-text", about: "Background behind HUD text.", kind: Kind::Cycle { key: "gfx_hud_text_background_opacity", values: PERCENT_OFF } },
    Def { id: "actionbar-background", name: "Action Bar Background", tab: "hud", icon: "message-square", about: "Background behind action bar messages.", kind: Kind::Cycle { key: "gfx_actionbar_text_background_opacity", values: PERCENT_OFF } },
    Def { id: "chat-background", name: "Chat Background", tab: "hud", icon: "message-square", about: "Background behind chat.", kind: Kind::Cycle { key: "gfx_chat_background_opacity", values: PERCENT_OFF } },
    Def { id: "gui-scale", name: "GUI Scale", tab: "hud", icon: "scaling", about: "Make the HUD and menus smaller.", kind: Kind::Cycle { key: "gfx_guiscale_offset", values: GUI_SCALES } },
    Def { id: "tooltips", name: "Tooltips", tab: "hud", icon: "info", about: "Show item tooltips.", kind: toggle("gfx_tooltips") },
    Def { id: "emote-chat", name: "Emote Chat Messages", tab: "hud", icon: "smile", about: "Show emotes players use in chat.", kind: toggle("emoteChat") },
    Def { id: "copy-coordinates", name: "Copy Coordinates Button", tab: "hud", icon: "copy", about: "A button to copy your coordinates.", kind: toggle("copy_coordinate_ui") },
    Def { id: "game-tips", name: "Game Tips", tab: "hud", icon: "lightbulb", about: "Tips that pop up while playing.", kind: toggle("game_tips_enabled") },
    Def { id: "control-tips", name: "Control Tips", tab: "hud", icon: "keyboard", about: "Key hints on screen.", kind: toggle("show_control_tips") },
    Def { id: "subtitles", name: "Subtitles", tab: "hud", icon: "captions", about: "Text for sounds around you (helps spot players).", kind: toggle("enable_gameplay_subtitles") },
    Def { id: "hide-own-subtitles", name: "Hide My Own Subtitles", tab: "hud", icon: "captions", about: "Leave out sounds you make.", kind: toggle("hide_own_gameplay_subtitles") },
    Def { id: "hide-ambient-subtitles", name: "Hide Ambient Subtitles", tab: "hud", icon: "captions", about: "Leave out ambience in subtitles.", kind: toggle("hide_ambient_gameplay_subtitles") },
    // multiplayer and privacy
    Def { id: "lan-visible", name: "Visible on LAN", tab: "privacy", icon: "wifi", about: "Let players on your network see your worlds.", kind: toggle("mp_server_visible") },
    Def { id: "xbox-visible", name: "Visible to Xbox Friends", tab: "privacy", icon: "gamepad-2", about: "Let Xbox Live friends see and join your worlds.", kind: toggle("mp_xboxlive_visible") },
    Def { id: "crossplay", name: "Cross-Platform Play", tab: "privacy", icon: "globe", about: "Play with consoles and phones.", kind: toggle("crossplatform_toggle") },
    Def { id: "filter-profanity", name: "Filter Profanity", tab: "privacy", icon: "shield-check", about: "Hide swear words in chat.", kind: toggle("filter_profanity") },
    Def { id: "trusted-skins", name: "Only Trusted Skins", tab: "privacy", icon: "badge-check", about: "Show other players' custom skins only when trusted.", kind: toggle("only_show_trusted_skins") },
    // added in 0.4.2
    Def { id: "max-framerate", name: "Max Framerate", tab: "performance", icon: "timer", about: "Cap the frame rate (saves power and heat) or leave it unlimited.", kind: Kind::Cycle { key: "gfx_max_framerate", values: MAX_FPS } },
    Def { id: "no-weather", name: "No Weather Effects", tab: "performance", icon: "cloud-off", about: "Hides falling rain and snow so you can see (Shadow Client's pack). The sound stays; turn it down under Audio.", kind: Kind::PackToggle },
    Def { id: "clear-portals", name: "Clear Nether Portals", tab: "visual", icon: "door-open", about: "See through Nether portals: just a light purple tint (Shadow Client's pack).", kind: Kind::PackToggle },
    Def { id: "glint-colour", name: "Enchant Glint Colour", tab: "visual", icon: "wand-sparkles", about: "Recolour the shimmer on enchanted items and armour, or turn it off (Shadow Client's pack).", kind: Kind::PackCycle { values: GLINT_COLOURS } },
    Def { id: "ui-profile", name: "UI Profile", tab: "hud", icon: "smartphone", about: "Classic is the PC layout; Pocket has bigger buttons and simpler menus.", kind: Kind::Cycle { key: "gfx_ui_profile", values: UI_PROFILES } },
    Def { id: "chat-font", name: "Chat Font", tab: "hud", icon: "type", about: "The typeface chat uses: Minecraft's or the cleaner Noto Sans.", kind: Kind::Cycle { key: "chat_typeface", values: CHAT_FONTS } },
    Def { id: "chat-font-size", name: "Chat Font Size", tab: "hud", icon: "a-large-small", about: "How big chat text is.", kind: Kind::Cycle { key: "chat_font_size", values: CHAT_SIZES } },
    Def { id: "chat-colour", name: "Chat Text Colour", tab: "hud", icon: "palette", about: "The colour of chat messages.", kind: Kind::Cycle { key: "chat_color_code", values: CHAT_COLOURS } },
    Def { id: "mention-colour", name: "Mention Colour", tab: "hud", icon: "at-sign", about: "The colour of chat messages that mention your name.", kind: Kind::Cycle { key: "chat_mentions_color_code", values: MENTION_COLOURS } },
];

// ------------------------------------------------------------------ views

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleView {
    pub id: String,
    pub name: String,
    pub tab: String,
    pub icon: String,
    pub about: String,
    /// "toggle" or "cycle"
    pub kind: String,
    /// "settings" (Bedrock's options) or "pack" (Shadow Client's resource pack)
    pub source: String,
    pub enabled: bool,
    pub value: String,
    pub value_label: String,
    pub choices: Vec<Choice>,
    /// false when this Bedrock version doesn't have the setting
    pub available: bool,
}

/// One Xbox account's Bedrock data (GDK builds keep one folder per account).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockUser {
    pub id: String,
    pub label: String,
    pub last_played: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuView {
    pub users: Vec<BedrockUser>,
    pub user: Option<String>,
    pub running: bool,
    pub modules: Vec<ModuleView>,
    /// Shadow Client's pack is switched on for this account
    pub pack_active: bool,
    pub has_original: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PackState {
    #[serde(default)]
    version: u32,
    /// pack module id → value ("on" for toggles)
    #[serde(default)]
    modules: std::collections::BTreeMap<String, String>,
}

// ------------------------------------------------------------------ accounts

/// `…\Users\<account>\games\com.mojang\minecraftpe` folders next to the shared data folder, newest
/// first. Older (UWP) installs have a single `minecraftpe` inside the data folder.
fn account_dirs(data_dir: &Path) -> Vec<(String, PathBuf)> {
    let users = data_dir.parent().and_then(Path::parent).and_then(Path::parent);
    let mut out = vec![];
    if let Some(users) = users.filter(|u| u.file_name().is_some_and(|n| n.eq_ignore_ascii_case("Users"))) {
        for e in std::fs::read_dir(users).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let pe = e.path().join("games").join("com.mojang").join("minecraftpe");
            if !name.eq_ignore_ascii_case("Shared") && pe.join("options.txt").is_file() {
                out.push((name, pe));
            }
        }
    }
    if out.is_empty() && data_dir.join("minecraftpe").join("options.txt").is_file() {
        out.push(("default".into(), data_dir.join("minecraftpe")));
    }
    out.sort_by_key(|(_, pe)| std::cmp::Reverse(last_played(pe)));
    out
}

/// When the account last played: the newest file in its minecraftpe folder, ignoring options.txt
/// (Bedrock rewrites every account's copy when it starts).
fn last_played(pe: &Path) -> Option<std::time::SystemTime> {
    let mut newest = None;
    let mut visit = |p: &Path| {
        if let Ok(m) = std::fs::metadata(p).and_then(|m| m.modified()) {
            if newest.map_or(true, |n| m > n) {
                newest = Some(m);
            }
        }
    };
    for e in std::fs::read_dir(pe).into_iter().flatten().flatten() {
        let p = e.path();
        if p.file_name().is_some_and(|n| n == "options.txt") {
            continue;
        }
        if p.is_dir() {
            for f in std::fs::read_dir(&p).into_iter().flatten().flatten() {
                visit(&f.path());
            }
        } else {
            visit(&p);
        }
    }
    newest
}

pub fn users(data_dir: &Path) -> Vec<BedrockUser> {
    account_dirs(data_dir)
        .into_iter()
        .enumerate()
        .map(|(i, (id, pe))| {
            let tail: String = id.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
            let label = if id == "default" { "This PC".to_string() } else if i == 0 { format!("Account …{tail} (played last)") } else { format!("Account …{tail}") };
            BedrockUser { id, label, last_played: last_played(&pe).map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()) }
        })
        .collect()
}

pub(crate) fn account_dir(data_dir: &Path, user: Option<&str>) -> Result<(String, PathBuf)> {
    let all = account_dirs(data_dir);
    match user {
        Some(u) => all.into_iter().find(|(id, _)| id == u).ok_or_else(|| Error::NotFound(format!("Bedrock account {u}"))),
        None => all.into_iter().next().ok_or_else(|| Error::NotFound("No Bedrock settings yet. Start Bedrock once, then try again.".into())),
    }
}

/// Bedrock writes its settings when it closes, so changes are refused while it runs.
pub fn is_running() -> bool {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, sysinfo::ProcessRefreshKind::nothing());
    sys.processes().values().any(|p| p.name().eq_ignore_ascii_case("Minecraft.Windows.exe"))
}

// ------------------------------------------------------------------ options.txt

fn read_options(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => Error::NotFound("Bedrock hasn't saved settings for this account yet. Start Bedrock once, then try again.".into()),
        _ => e.into(),
    })
}

fn option_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| l.trim_end_matches('\r').split_once(':').filter(|(k, _)| *k == key).map(|(_, v)| v.trim().to_string()))
}

/// Set `key:value`, keeping every other line (and the file's line endings) as it was.
fn set_option(text: &str, key: &str, value: &str) -> String {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|l| {
            let l = l.trim_end_matches('\r');
            match l.split_once(':') {
                Some((k, _)) if k == key => {
                    found = true;
                    format!("{key}:{value}")
                }
                _ => l.to_string(),
            }
        })
        .collect();
    if !found {
        lines.push(format!("{key}:{value}"));
    }
    let mut out = lines.join(nl);
    if text.ends_with('\n') || !found {
        out.push_str(nl);
    }
    out
}

/// [`set_option`], plus the copy Bedrock keeps of some settings when the file has it.
fn set_mirrored(text: &str, key: &str, value: &str) -> String {
    let mut out = set_option(text, key, value);
    for (_, twin) in MIRRORS.iter().filter(|(k, _)| *k == key) {
        if option_value(&out, twin).is_some() {
            out = set_option(&out, twin, value);
        }
    }
    out
}

pub(crate) fn backup_dir(backups: &Path, user: &str) -> PathBuf {
    backups.join("bedrock").join("settings").join(user)
}

/// Keep the file as it was before Shadow Client first changed it, and the version before this change.
pub(crate) fn backup_file(src: &Path, dir: &Path, name: &str) -> Result<()> {
    if !src.is_file() {
        return Ok(());
    }
    std::fs::create_dir_all(dir)?;
    let original = dir.join(format!("{name}.original"));
    if !original.exists() {
        std::fs::copy(src, &original)?;
    }
    std::fs::copy(src, dir.join(format!("{name}.previous")))?;
    Ok(())
}

// ------------------------------------------------------------------ the pack

fn state_path(launcher_root: &Path) -> PathBuf {
    launcher_root.join(STATE_FILE)
}

fn load_state(launcher_root: &Path) -> PackState {
    util::read_json_opt(&state_path(launcher_root)).ok().flatten().unwrap_or_default()
}

fn pack_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("resource_packs").join(PACK_FOLDER)
}

fn global_packs_path(pe: &Path) -> PathBuf {
    pe.join("global_resource_packs.json")
}

fn read_global(pe: &Path) -> Result<Vec<serde_json::Value>> {
    let path = global_packs_path(pe);
    if !path.is_file() {
        return Ok(vec![]);
    }
    let text = std::fs::read_to_string(&path)?;
    let text = text.trim_start_matches('\u{feff}');
    if text.trim().is_empty() {
        return Ok(vec![]);
    }
    serde_json::from_str::<Vec<serde_json::Value>>(text)
        .map_err(|e| Error::Invalid(format!("Bedrock's list of global resource packs couldn't be read ({e}); it was left as it is.")))
}

fn pack_active(pe: &Path) -> bool {
    read_global(pe).is_ok_and(|l| l.iter().any(|p| p["pack_id"].as_str() == Some(PACK_UUID)))
}

/// Build the pack from the chosen modules (or remove it when none are on), then put it at the
/// top of the account's global resource packs so its choices win over other packs.
fn sync_pack(data_dir: &Path, pe: &Path, backups: &Path, user: &str, state: &mut PackState) -> Result<()> {
    let files = pack_files(state);
    let dir = pack_dir(data_dir);
    let mut list = read_global(pe)?;
    list.retain(|p| p["pack_id"].as_str() != Some(PACK_UUID));
    if files.is_empty() {
        if dir.exists() {
            let to = backups.join("bedrock").join(util::timestamp_slug()).join(PACK_FOLDER);
            util::move_path(&dir, &to)?;
        }
    } else {
        // A new version each time, so Bedrock doesn't keep showing a cached copy.
        state.version += 1;
        let staging = dir.with_file_name(format!(".{PACK_FOLDER}-new"));
        if staging.exists() {
            std::fs::remove_dir_all(&staging)?;
        }
        std::fs::create_dir_all(&staging)?;
        std::fs::write(staging.join("manifest.json"), manifest(state.version))?;
        std::fs::write(staging.join("pack_icon.png"), PACK_ICON)?;
        for (rel, bytes) in &files {
            let p = staging.join(rel);
            std::fs::create_dir_all(p.parent().expect("pack files are in folders"))?;
            std::fs::write(p, bytes)?;
        }
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        std::fs::rename(&staging, &dir)?;
        list.insert(0, serde_json::json!({ "pack_id": PACK_UUID, "version": [1, 0, state.version] }));
    }
    let path = global_packs_path(pe);
    backup_file(&path, &backup_dir(backups, user), "global_resource_packs.json")?;
    util::atomic_write(&path, serde_json::to_string_pretty(&list).map_err(|e| Error::msg(e.to_string()))?.as_bytes())?;
    Ok(())
}

fn manifest(version: u32) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "format_version": 2,
        "header": {
            "name": "Shadow Client Modules",
            "description": "Made by Shadow Client's Bedrock mod menu. Change it there.",
            "uuid": PACK_UUID,
            "version": [1, 0, version],
            "min_engine_version": [1, 21, 0]
        },
        "modules": [{ "type": "resources", "uuid": PACK_MODULE_UUID, "version": [1, 0, version] }]
    }))
    .expect("static json")
}

/// The pack's files for the modules that are on (empty when none are).
fn pack_files(state: &PackState) -> Vec<(&'static str, Vec<u8>)> {
    let mut out = vec![];
    let on = |id: &str| state.modules.get(id).is_some_and(|v| v == "on");
    if let Some(style) = state.modules.get("crosshair").filter(|s| s.as_str() != "default") {
        let colour = state.modules.get("crosshair-colour").map(String::as_str).unwrap_or("white");
        out.push(("textures/ui/cross_hair.png", crosshair(style, colour)));
    }
    if on("shadow-background") {
        for (path, bytes) in PANORAMA_PATHS.iter().zip(PANORAMA_FACES) {
            out.push((path, bytes.to_vec()));
        }
    }
    if on("clean-stained-glass") {
        for (path, rgb) in GLASS_COLOURS {
            out.push((path, framed_glass(*rgb, 210)));
        }
    }
    if on("no-fire") {
        out.push(("textures/flame_atlas.png", png(16, 16, &[0; 16 * 16 * 4])));
    }
    if on("no-pumpkin") {
        out.push(("textures/misc/pumpkinblur.png", png(16, 16, &[0; 16 * 16 * 4])));
    }
    if on("clean-glass") {
        out.push(("textures/blocks/glass.png", clean_glass()));
    }
    if on("no-weather") {
        out.push(("textures/environment/weather.png", png(16, 16, &[0; 16 * 16 * 4])));
    }
    if on("clear-portals") {
        // one frame (the portal's flipbook takes as many as the texture has)
        out.push(("textures/blocks/portal.png", png(16, 16, &[150, 70, 230, 60].repeat(16 * 16))));
    }
    if let Some(colour) = state.modules.get("glint-colour").filter(|c| c.as_str() != "default") {
        let g = glint(colour);
        out.push(("textures/misc/enchanted_item_glint.png", g.clone()));
        out.push(("textures/misc/enchanted_actor_glint.png", g));
    }
    out
}

/// A soft diagonal shimmer in one colour (the glint is drawn added on top, so black adds nothing).
fn glint(colour: &str) -> Vec<u8> {
    let rgb: [f64; 3] = match colour {
        "red" => [255.0, 70.0, 70.0],
        "aqua" => [70.0, 230.0, 255.0],
        "gold" => [255.0, 190.0, 60.0],
        "green" => [100.0, 255.0, 110.0],
        "none" => [0.0, 0.0, 0.0],
        _ => [160.0, 100.0, 255.0],
    };
    const N: usize = 64;
    let mut px = vec![0u8; N * N * 4];
    for y in 0..N {
        for x in 0..N {
            let t = (x + y) as f64 / N as f64 * std::f64::consts::TAU;
            let k = (0.55 + 0.45 * t.sin()) * (0.75 + 0.25 * (t * 3.0).cos());
            let i = (y * N + x) * 4;
            for c in 0..3 {
                px[i + c] = (rgb[c] * k).round() as u8;
            }
            px[i + 3] = if colour == "none" { 0 } else { 255 };
        }
    }
    png(N as u32, N as u32, &px)
}

fn crosshair(style: &str, colour: &str) -> Vec<u8> {
    let rgba: [u8; 4] = match colour {
        "purple" => [155, 123, 255, 255],
        "green" => [122, 230, 90, 255],
        "red" => [255, 80, 80, 255],
        "aqua" => [85, 255, 255, 255],
        _ => [255, 255, 255, 255],
    };
    let mut px = vec![0u8; 16 * 16 * 4];
    let mut put = |x: i32, y: i32| {
        if (0..16).contains(&x) && (0..16).contains(&y) {
            let i = ((y * 16 + x) * 4) as usize;
            px[i..i + 4].copy_from_slice(&rgba);
        }
    };
    match style {
        "dot" => (7..9).for_each(|y| (7..9).for_each(|x| put(x, y))),
        "plus" => (3..13).for_each(|i| {
            put(i, 7);
            put(7, i);
        }),
        "x" => (3..13).for_each(|i| {
            put(i, i);
            put(i, 15 - i);
        }),
        _ => {
            // a ring around a dot
            for y in 0..16 {
                for x in 0..16 {
                    let d = ((x as f64 - 7.5).powi(2) + (y as f64 - 7.5).powi(2)).sqrt();
                    if (d - 5.0).abs() < 0.55 || d < 1.0 {
                        put(x, y);
                    }
                }
            }
        }
    }
    png(16, 16, &px)
}

fn clean_glass() -> Vec<u8> {
    framed_glass([220, 235, 240], 200)
}

/// Glass that is see-through except a one-pixel frame of the given colour.
fn framed_glass(rgb: [u8; 3], alpha: u8) -> Vec<u8> {
    let mut px = vec![0u8; 16 * 16 * 4];
    for y in 0..16 {
        for x in 0..16 {
            if x == 0 || y == 0 || x == 15 || y == 15 {
                let i = (y * 16 + x) * 4;
                px[i..i + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], alpha]);
            }
        }
    }
    png(16, 16, &px)
}

/// A minimal RGBA PNG (stored, uncompressed deflate): these textures are tiny.
fn png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    }
    let row = w as usize * 4;
    let mut raw = Vec::with_capacity((row + 1) * h as usize);
    for y in 0..h as usize {
        raw.push(0); // filter: none
        raw.extend_from_slice(&rgba[y * row..(y + 1) * row]);
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, b) in blocks.iter().enumerate() {
        z.push(u8::from(i + 1 == blocks.len()));
        let n = b.len() as u16;
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&(!n).to_le_bytes());
        z.extend_from_slice(b);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = vec![];
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

// ------------------------------------------------------------------ public API

fn view(def: &Def, options: &str, state: &PackState) -> ModuleView {
    let (kind, source, enabled, value, choices, available) = match &def.kind {
        Kind::Toggle { key, on, .. } => {
            let v = option_value(options, key);
            ("toggle", "settings", v.as_deref() == Some(on), v.clone().unwrap_or_default(), vec![], v.is_some())
        }
        Kind::Cycle { key, values } => {
            let v = option_value(options, key);
            ("cycle", "settings", true, v.clone().unwrap_or_default(), values.to_vec(), v.is_some())
        }
        Kind::PackToggle => {
            let on = state.modules.get(def.id).is_some_and(|v| v == "on");
            ("toggle", "pack", on, if on { "on" } else { "off" }.to_string(), vec![], true)
        }
        Kind::PackCycle { values } => {
            let v = state.modules.get(def.id).cloned().unwrap_or_else(|| values[0].0.to_string());
            ("cycle", "pack", v != values[0].0, v, values.to_vec(), true)
        }
    };
    let value_label = choices
        .iter()
        .find(|(v, _)| v.parse::<f64>().ok().zip(value.parse::<f64>().ok()).map_or(*v == value, |(a, b)| (a - b).abs() < 1e-6))
        .map(|(_, l)| l.to_string())
        .unwrap_or_else(|| match value.parse::<f64>() {
            // a value set in Bedrock between our steps, on a percent scale (e.g. music at 0.3)
            Ok(v) if choices.iter().any(|(_, l)| l.ends_with('%')) => format!("{}%", (v * 100.0).round()),
            _ => value.clone(),
        });
    ModuleView {
        id: def.id.into(),
        name: def.name.into(),
        tab: def.tab.into(),
        icon: def.icon.into(),
        about: def.about.into(),
        kind: kind.into(),
        source: source.into(),
        enabled,
        value,
        value_label,
        choices: choices.into_iter().map(|(v, l)| Choice { value: v.into(), label: l.into() }).collect(),
        available,
    }
}

/// The menu for one account (the most recently played one when `user` is None).
pub fn menu(data_dir: &Path, launcher_root: &Path, backups: &Path, user: Option<&str>) -> Result<MenuView> {
    let users = users(data_dir);
    let running = is_running();
    let Ok((id, pe)) = account_dir(data_dir, user) else {
        return Ok(MenuView { users, user: None, running, modules: vec![], pack_active: false, has_original: false });
    };
    let options = read_options(&pe.join("options.txt"))?;
    let state = load_state(launcher_root);
    Ok(MenuView {
        modules: DEFS.iter().map(|d| view(d, &options, &state)).collect(),
        pack_active: pack_active(&pe),
        has_original: backup_dir(backups, &id).join("options.txt.original").is_file(),
        users,
        user: Some(id),
        running,
    })
}

/// Turn a module on/off ("on"/"off") or pick one of its values.
pub fn set(data_dir: &Path, launcher_root: &Path, backups: &Path, user: Option<&str>, id: &str, value: &str) -> Result<ModuleView> {
    set_inner(data_dir, launcher_root, backups, user, id, value, is_running())
}

fn set_inner(data_dir: &Path, launcher_root: &Path, backups: &Path, user: Option<&str>, id: &str, value: &str, running: bool) -> Result<ModuleView> {
    if running {
        return Err(Error::Invalid("Close Minecraft Bedrock first: it saves its settings when it closes and would undo this.".into()));
    }
    let def = DEFS.iter().find(|d| d.id == id).ok_or_else(|| Error::NotFound(format!("module {id}")))?;
    let (uid, pe) = account_dir(data_dir, user)?;
    let options_path = pe.join("options.txt");
    let mut state = load_state(launcher_root);
    match &def.kind {
        Kind::Toggle { key, on, off } => {
            let v = match value {
                "on" => *on,
                "off" => *off,
                _ => return Err(Error::Invalid("Use on or off".into())),
            };
            let text = read_options(&options_path)?;
            backup_file(&options_path, &backup_dir(backups, &uid), "options.txt")?;
            util::atomic_write(&options_path, set_mirrored(&text, key, v).as_bytes())?;
        }
        Kind::Cycle { key, values } => {
            if !values.iter().any(|(v, _)| *v == value) {
                return Err(Error::Invalid(format!("{} can't be set to {value}", def.name)));
            }
            let text = read_options(&options_path)?;
            backup_file(&options_path, &backup_dir(backups, &uid), "options.txt")?;
            util::atomic_write(&options_path, set_mirrored(&text, key, value).as_bytes())?;
        }
        Kind::PackToggle | Kind::PackCycle { .. } => {
            let ok = match &def.kind {
                Kind::PackCycle { values } => values.iter().any(|(v, _)| *v == value),
                _ => value == "on" || value == "off",
            };
            if !ok {
                return Err(Error::Invalid(format!("{} can't be set to {value}", def.name)));
            }
            if value == "off" || value == "default" {
                state.modules.remove(def.id);
            } else {
                state.modules.insert(def.id.into(), value.into());
            }
            sync_pack(data_dir, &pe, backups, &uid, &mut state)?;
            util::write_json(&state_path(launcher_root), &state)?;
        }
    }
    let options = read_options(&options_path)?;
    Ok(view(def, &options, &state))
}

/// Put back the account's settings from before Shadow Client first changed them, and switch
/// Shadow Client's pack off (its folder goes to backups).
pub fn restore(data_dir: &Path, launcher_root: &Path, backups: &Path, user: Option<&str>) -> Result<()> {
    restore_inner(data_dir, launcher_root, backups, user, is_running())
}

fn restore_inner(data_dir: &Path, launcher_root: &Path, backups: &Path, user: Option<&str>, running: bool) -> Result<()> {
    if running {
        return Err(Error::Invalid("Close Minecraft Bedrock first.".into()));
    }
    let (uid, pe) = account_dir(data_dir, user)?;
    let original = backup_dir(backups, &uid).join("options.txt.original");
    if original.is_file() {
        let options = pe.join("options.txt");
        backup_file(&options, &backup_dir(backups, &uid), "options.txt")?;
        util::atomic_write(&options, &std::fs::read(&original)?)?;
    }
    let mut state = load_state(launcher_root);
    state.modules.clear();
    sync_pack(data_dir, &pe, backups, &uid, &mut state)?;
    util::write_json(&state_path(launcher_root), &state)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A GDK-style layout: Users\<account>\games\com.mojang\minecraftpe and Users\Shared\games\com.mojang.
    fn layout(root: &Path) -> (PathBuf, PathBuf) {
        let data = root.join("Users/Shared/games/com.mojang");
        let pe = root.join("Users/1234567890/games/com.mojang/minecraftpe");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&pe).unwrap();
        std::fs::write(pe.join("options.txt"), "gfx_vsync:1\r\ngfx_hidepaperdoll:1\r\ngfx_viewdistance:1280\r\ngfx_gamma:0.5\r\naudio_music:0.3\r\n").unwrap();
        std::fs::write(pe.join("global_resource_packs.json"), "[\n\t{\n\t\t\"pack_id\" : \"other\",\n\t\t\"version\" : [ 1, 0, 0 ]\n\t}\n]").unwrap();
        (data, pe)
    }

    #[test]
    fn settings_change_only_their_line_and_keep_backups() {
        let d = tempfile::tempdir().unwrap();
        let (data, pe) = layout(d.path());
        let (launcher, backups) = (d.path().join("launcher"), d.path().join("backups"));
        let m = menu(&data, &launcher, &backups, None).unwrap();
        assert_eq!(m.user.as_deref(), Some("1234567890"));
        let vsync = m.modules.iter().find(|x| x.id == "vsync").unwrap();
        assert!(vsync.enabled && vsync.available);
        let doll = m.modules.iter().find(|x| x.id == "paper-doll").unwrap();
        assert!(!doll.enabled, "hidepaperdoll 1 = paper doll off");
        assert_eq!(m.modules.iter().find(|x| x.id == "brightness").unwrap().value_label, "50%");
        let music = m.modules.iter().find(|x| x.id == "music-volume").unwrap();
        assert_eq!(music.value_label, "30%", "a value between the steps still reads as a percentage");
        assert!(!m.modules.iter().find(|x| x.id == "hurt-cam").unwrap().available, "missing key");

        set_inner(&data, &launcher, &backups, None, "vsync", "off", false).unwrap();
        let v = set_inner(&data, &launcher, &backups, None, "render-distance", "256", false).unwrap();
        assert_eq!(v.value_label, "16 chunks");
        let v = set_inner(&data, &launcher, &backups, None, "hurt-cam", "off", false).unwrap();
        assert!(v.available && !v.enabled, "a missing key is added");
        let text = std::fs::read_to_string(pe.join("options.txt")).unwrap();
        assert_eq!(text, "gfx_vsync:0\r\ngfx_hidepaperdoll:1\r\ngfx_viewdistance:256\r\ngfx_gamma:0.5\r\naudio_music:0.3\r\ngfx_damagebobbing:0\r\n");
        assert!(set_inner(&data, &launcher, &backups, None, "render-distance", "99999", false).is_err());
        assert!(set_inner(&data, &launcher, &backups, None, "vsync", "on", true).is_err(), "refused while Bedrock runs");

        let dir = backup_dir(&backups, "1234567890");
        assert!(std::fs::read_to_string(dir.join("options.txt.original")).unwrap().starts_with("gfx_vsync:1"));
        assert!(restore_inner(&data, &launcher, &backups, None, true).is_err(), "refused while Bedrock runs");
        restore_inner(&data, &launcher, &backups, None, false).unwrap();
        assert!(std::fs::read_to_string(pe.join("options.txt")).unwrap().starts_with("gfx_vsync:1\r\ngfx_hidepaperdoll:1\r\ngfx_viewdistance:1280"));
    }

    #[test]
    fn pack_modules_build_and_switch_on_the_pack() {
        let d = tempfile::tempdir().unwrap();
        let (data, pe) = layout(d.path());
        let (launcher, backups) = (d.path().join("launcher"), d.path().join("backups"));
        let v = set_inner(&data, &launcher, &backups, None, "crosshair", "dot", false).unwrap();
        assert!(v.enabled && v.value_label == "Dot");
        set_inner(&data, &launcher, &backups, None, "no-pumpkin", "on", false).unwrap();
        let pack = pack_dir(&data);
        assert!(pack.join("textures/ui/cross_hair.png").is_file() && pack.join("textures/misc/pumpkinblur.png").is_file());
        let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(pack.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["header"]["uuid"], PACK_UUID);
        assert_eq!(manifest["header"]["version"], serde_json::json!([1, 0, 2]), "a new version per change");
        let list = read_global(&pe).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["pack_id"], PACK_UUID, "ours on top");
        assert_eq!(list[1]["pack_id"], "other", "the player's own packs stay");
        assert!(menu(&data, &launcher, &backups, None).unwrap().pack_active);

        // the Shadow background and stained glass go into the same pack, and come out again
        set_inner(&data, &launcher, &backups, None, "shadow-background", "on", false).unwrap();
        set_inner(&data, &launcher, &backups, None, "clean-stained-glass", "on", false).unwrap();
        assert!((0..6).all(|i| pack.join(format!("textures/ui/panorama_{i}.png")).is_file()));
        assert_eq!(std::fs::read(pack.join("textures/ui/panorama_0.png")).unwrap(), PANORAMA_FACES[0]);
        assert!(pack.join("textures/blocks/glass_red.png").is_file() && pack.join("textures/blocks/glass_silver.png").is_file());
        set_inner(&data, &launcher, &backups, None, "shadow-background", "off", false).unwrap();
        set_inner(&data, &launcher, &backups, None, "clean-stained-glass", "off", false).unwrap();
        assert!(!pack.join("textures/ui/panorama_0.png").exists(), "rebuilt without it");
        assert_eq!(read_global(&pe).unwrap()[0]["pack_id"], PACK_UUID, "still on for the crosshair");

        // all pack modules off: the pack leaves the list and its folder goes to backups
        set_inner(&data, &launcher, &backups, None, "crosshair", "default", false).unwrap();
        set_inner(&data, &launcher, &backups, None, "no-pumpkin", "off", false).unwrap();
        assert!(!pack.exists());
        let list = read_global(&pe).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["pack_id"], "other");
        assert!(backup_dir(&backups, "1234567890").join("global_resource_packs.json.original").is_file());
    }

    #[test]
    fn new_modules_write_bedrocks_own_values() {
        let d = tempfile::tempdir().unwrap();
        let (data, pe) = layout(d.path());
        std::fs::write(pe.join("options.txt"), "gfx_max_framerate:0\r\ngfx_max_framerate_advanced:0\r\nchat_color_code:§f\r\nchat_typeface:0\r\n").unwrap();
        let (launcher, backups) = (d.path().join("launcher"), d.path().join("backups"));
        let m = menu(&data, &launcher, &backups, None).unwrap();
        let fps = m.modules.iter().find(|x| x.id == "max-framerate").unwrap();
        assert!(fps.available && fps.value_label == "Unlimited");
        assert_eq!(m.modules.iter().find(|x| x.id == "chat-colour").unwrap().value_label, "White");
        assert!(!m.modules.iter().find(|x| x.id == "ui-profile").unwrap().available, "not in this file");

        set_inner(&data, &launcher, &backups, None, "max-framerate", "144", false).unwrap();
        let v = set_inner(&data, &launcher, &backups, None, "chat-colour", "§b", false).unwrap();
        assert_eq!(v.value_label, "Aqua");
        set_inner(&data, &launcher, &backups, None, "chat-font", "1", false).unwrap();
        let text = std::fs::read_to_string(pe.join("options.txt")).unwrap();
        assert_eq!(text, "gfx_max_framerate:144\r\ngfx_max_framerate_advanced:144\r\nchat_color_code:§b\r\nchat_typeface:1\r\n", "both framerate copies");

        // pack ones
        set_inner(&data, &launcher, &backups, None, "no-weather", "on", false).unwrap();
        set_inner(&data, &launcher, &backups, None, "clear-portals", "on", false).unwrap();
        let g = set_inner(&data, &launcher, &backups, None, "glint-colour", "aqua", false).unwrap();
        assert!(g.enabled && g.value_label == "Aqua");
        let pack = pack_dir(&data);
        for f in ["textures/environment/weather.png", "textures/blocks/portal.png", "textures/misc/enchanted_item_glint.png", "textures/misc/enchanted_actor_glint.png"] {
            assert!(pack.join(f).is_file(), "{f}");
        }
        set_inner(&data, &launcher, &backups, None, "glint-colour", "default", false).unwrap();
        assert!(!pack.join("textures/misc/enchanted_item_glint.png").exists(), "Bedrock's own glint again");
        assert!(set_inner(&data, &launcher, &backups, None, "glint-colour", "blue", false).is_err());
    }

    #[test]
    fn module_ids_are_unique() {
        let mut ids: Vec<&str> = DEFS.iter().map(|d| d.id).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn png_is_well_formed() {
        let p = crosshair("circle", "white");
        assert_eq!(&p[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&p[12..16], b"IHDR");
        assert_eq!(&p[16..24], &[0, 0, 0, 16, 0, 0, 0, 16]);
        assert!(p.ends_with(&[0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82]), "IEND with its fixed CRC");
        // to check one in an image viewer: SHADOW_DUMP_PNG=<folder> cargo test png_is_well_formed
        if let Ok(dir) = std::env::var("SHADOW_DUMP_PNG") {
            for s in ["dot", "plus", "circle", "x"] {
                std::fs::write(Path::new(&dir).join(format!("cross_{s}.png")), crosshair(s, "purple")).unwrap();
            }
            std::fs::write(Path::new(&dir).join("glass.png"), clean_glass()).unwrap();
            for c in ["purple", "aqua", "gold"] {
                std::fs::write(Path::new(&dir).join(format!("glint_{c}.png")), glint(c)).unwrap();
            }
        }
    }
}
