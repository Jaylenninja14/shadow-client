//! Mod management: metadata, sources (Modrinth, CurseForge, built-in), resolver, installs.

pub mod builtin;
pub mod compat;
pub mod curseforge;
pub mod index;
pub mod manage;
pub mod metadata;
pub mod modrinth;
pub mod resolver;

use serde::{Deserialize, Serialize};

use crate::minecraft::loaders::LoaderKind;

/// What a profile can run: its Minecraft version and loader.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub mc_version: String,
    pub loader: LoaderKind,
}

impl Target {
    pub fn new(mc: &str, loader: LoaderKind) -> Self {
        Self { mc_version: mc.to_string(), loader }
    }
}

/// Kinds of content that live in a game directory.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    Mod,
    Resourcepack,
    Shader,
}

impl ContentKind {
    pub fn folder(&self) -> &'static str {
        match self {
            ContentKind::Mod => "mods",
            ContentKind::Resourcepack => "resourcepacks",
            ContentKind::Shader => "shaderpacks",
        }
    }
    pub fn modrinth_type(&self) -> &'static str {
        match self {
            ContentKind::Mod => "mod",
            ContentKind::Resourcepack => "resourcepack",
            ContentKind::Shader => "shader",
        }
    }
    pub fn curseforge_class(&self) -> u32 {
        match self {
            ContentKind::Mod => 6,
            ContentKind::Resourcepack => 12,
            ContentKind::Shader => 6552,
        }
    }
}
