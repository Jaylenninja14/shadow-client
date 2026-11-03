//! Integration tests: every external service is a local mock server and every game folder is
//! a temporary directory. Nothing here touches the real `.minecraft`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use mockito::Matcher;
use serde_json::json;

use crate::auth;
use crate::java::JavaInstall;
use crate::minecraft::install;
use crate::minecraft::launch::{self, ArgInputs, GameManager};
use crate::minecraft::loaders::LoaderKind;
use crate::mods::manage::{self, ProfileScope};
use crate::mods::metadata::tests::make_jar;
use crate::mods::resolver;
use crate::mods::{builtin, ContentKind, Target};
use crate::profiles::{NewProfile, ProfileKind, Profiles};
use crate::testutil::{ctx, sha1, sha512};
use crate::{modpacks, Error};

fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut cur = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut cur);
        for (name, data) in entries {
            z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap();
    }
    cur.into_inner()
}

fn jar_bytes(dir: &Path, fabric_json: &str) -> Vec<u8> {
    let p = dir.join(format!("tmp-{}.jar", uuid::Uuid::new_v4().simple()));
    make_jar(&p, fabric_json);
    let b = std::fs::read(&p).unwrap();
    std::fs::remove_file(p).unwrap();
    b
}

fn rx(path: &str) -> Matcher {
    Matcher::Regex(format!("^{}($|\\?)", regex::escape(path)))
}

fn test_auth() -> auth::LaunchAuth {
    auth::LaunchAuth {
        username: "Tester".into(),
        uuid: auth::offline_uuid("Tester"),
        access_token: "offline".into(),
        user_type: "legacy".into(),
        xuid: "0".into(),
        client_id: String::new(),
        offline: true,
    }
}

// ------------------------------------------------------------------------ game install

#[tokio::test]
async fn installs_vanilla_and_fabric_from_mock_mojang() {
    let mut s = mockito::Server::new_async().await;
    let base = s.url();
    let d = tempfile::tempdir().unwrap();
    let (ctx, _) = ctx(d.path(), &base);

    let lib_a = b"lib-a-bytes".to_vec();
    let natives = zip_bytes(&[("lwjgl.dll", b"dll"), ("META-INF/MANIFEST.MF", b"x")]);
    let client = b"client-jar".to_vec();
    let asset = b"{\"lang\":1}".to_vec();
    let idx = json!({"objects": {"minecraft/lang/en_us.json": {"hash": sha1(&asset), "size": asset.len()}}}).to_string();
    let log = b"<Configuration/>".to_vec();
    let loader_jar = b"fabric-loader".to_vec();
    let version = json!({
        "id": "1.21.11", "type": "release", "mainClass": "net.minecraft.client.main.Main",
        "javaVersion": {"component": "java-runtime-delta", "majorVersion": 21},
        "assetIndex": {"id": "29", "sha1": sha1(idx.as_bytes()), "size": idx.len(), "url": format!("{base}/idx.json")},
        "assets": "29",
        "downloads": {"client": {"sha1": sha1(&client), "size": client.len(), "url": format!("{base}/client.jar")}},
        "logging": {"client": {"argument": "-Dlog4j.configurationFile=${path}", "file": {"id": "client-1.21.2.xml", "sha1": sha1(&log), "size": log.len(), "url": format!("{base}/log.xml")}, "type": "log4j2-xml"}},
        "libraries": [
            {"name": "com.example:lib-a:1.0", "downloads": {"artifact": {"path": "com/example/lib-a/1.0/lib-a-1.0.jar", "sha1": sha1(&lib_a), "size": lib_a.len(), "url": format!("{base}/lib/a.jar")}}},
            {"name": "org.lwjgl:lwjgl:3.3.3:natives-windows", "downloads": {"artifact": {"path": "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar", "sha1": sha1(&natives), "size": natives.len(), "url": format!("{base}/lib/n.jar")}}, "rules": [{"action": "allow", "os": {"name": "windows"}}]},
            {"name": "com.example:mac-only:1.0", "downloads": {"artifact": {"path": "com/example/mac-only/1.0/mac-only-1.0.jar", "sha1": "00", "size": 1, "url": format!("{base}/lib/mac.jar")}}, "rules": [{"action": "allow", "os": {"name": "osx"}}]}
        ],
        "arguments": {
            "game": ["--username", "${auth_player_name}", "--version", "${version_name}", "--gameDir", "${game_directory}", "--assetsDir", "${assets_root}", "--assetIndex", "${assets_index_name}", "--accessToken", "${auth_access_token}",
                     {"rules": [{"action": "allow", "features": {"is_demo_user": true}}], "value": "--demo"}],
            "jvm": ["-Djava.library.path=${natives_directory}", "-cp", "${classpath}"]
        }
    })
    .to_string();
    let manifest = json!({"latest": {"release": "1.21.11", "snapshot": "1.21.11"}, "versions": [{"id": "1.21.11", "type": "release", "url": format!("{base}/v/1.21.11.json"), "time": "2025-12-01T00:00:00+00:00", "releaseTime": "2025-12-01T00:00:00+00:00", "sha1": sha1(version.as_bytes())}]});
    let _m1 = s.mock("GET", "/mc/game/version_manifest_v2.json").with_body(manifest.to_string()).create_async().await;
    let _m2 = s.mock("GET", "/v/1.21.11.json").with_body(version.clone()).create_async().await;
    let _m3 = s.mock("GET", "/idx.json").with_body(idx.clone()).create_async().await;
    let _m4 = s.mock("GET", "/client.jar").with_body(client.clone()).create_async().await;
    let _m5 = s.mock("GET", "/log.xml").with_body(log.clone()).create_async().await;
    let _m6 = s.mock("GET", "/lib/a.jar").with_body(lib_a.clone()).create_async().await;
    let _m7 = s.mock("GET", "/lib/n.jar").with_body(natives.clone()).create_async().await;
    let mac = s.mock("GET", "/lib/mac.jar").expect(0).create_async().await;
    let _m8 = s.mock("GET", format!("/resources/{}/{}", &sha1(&asset)[..2], sha1(&asset)).as_str()).with_body(asset.clone()).create_async().await;

    let rv = install::resolve_version(&ctx, "1.21.11", LoaderKind::Vanilla, None).await.unwrap();
    assert_eq!(crate::minecraft::manifest::required_java(&rv.json), 21);
    let inst = install::install(&ctx, rv, &[], true).await.unwrap();
    assert!(ctx.paths.libraries_dir().join("com/example/lib-a/1.0/lib-a-1.0.jar").is_file());
    assert!(!ctx.paths.libraries_dir().join("com/example/mac-only").exists(), "rule-excluded library not downloaded");
    mac.assert_async().await;
    assert!(inst.natives_dir.join("lwjgl.dll").is_file(), "natives extracted");
    assert!(!inst.natives_dir.join("META-INF").exists());
    assert!(ctx.paths.assets_dir().join("objects").join(&sha1(&asset)[..2]).join(sha1(&asset)).is_file());

    let java = JavaInstall { path: PathBuf::from("java"), home: PathBuf::new(), version: "21".into(), major: 21, vendor: None, arch: "x64".into(), source: "test".into() };
    let game_dir = d.path().join("game");
    let a = test_auth();
    let args = launch::build_args(&ArgInputs {
        installed: &inst,
        java: &java,
        game_dir: &game_dir,
        auth: &a,
        memory_min_mb: 512,
        memory_max_mb: 2048,
        jvm_preset: "mojang",
        extra_jvm_args: vec!["-Dfoo=bar".into()],
        resolution: None,
        fullscreen: false,
        libraries_dir: &ctx.paths.libraries_dir(),
        assets_dir: &ctx.paths.assets_dir(),
        join_server: None,
    });
    let joined = launch::build_args(&ArgInputs {
        installed: &inst,
        java: &java,
        game_dir: &game_dir,
        auth: &a,
        memory_min_mb: 512,
        memory_max_mb: 2048,
        jvm_preset: "mojang",
        extra_jvm_args: vec![],
        resolution: None,
        fullscreen: false,
        libraries_dir: &ctx.paths.libraries_dir(),
        assets_dir: &ctx.paths.assets_dir(),
        join_server: Some("play.example.net:25570"),
    });
    let quick = joined.windows(2).any(|w| w[0] == "--quickPlayMultiplayer" && w[1] == "play.example.net:25570");
    let legacy = joined.windows(2).any(|w| w[0] == "--server" && w[1] == "play.example.net") && joined.windows(2).any(|w| w[0] == "--port" && w[1] == "25570");
    assert!(quick || legacy, "join arguments missing: {joined:?}");
    let joined = args.join(" ");
    assert!(args.windows(2).any(|w| w[0] == "--username" && w[1] == "Tester"));
    assert!(args.contains(&"-Xmx2048M".to_string()) && args.contains(&"-Dfoo=bar".to_string()));
    assert!(joined.contains("lib-a-1.0.jar") && joined.contains("1.21.11.jar"));
    assert!(joined.contains("-Dlog4j.configurationFile=") && joined.contains("-Dlog4j2.formatMsgNoLookups=true"));
    assert!(!args.contains(&"--demo".to_string()));
    assert!(!joined.contains("${"), "all placeholders substituted: {joined}");

    // Fabric on top of the same vanilla files.
    let _f1 = s.mock("GET", "/fabric/v2/versions/loader/1.21.11").with_body(json!([{"loader": {"version": "0.17.3", "stable": true}}]).to_string()).create_async().await;
    let fabric_profile = json!({
        "id": "fabric-loader-0.17.3-1.21.11", "inheritsFrom": "1.21.11", "type": "release",
        "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
        "arguments": {"game": [], "jvm": ["-DFabricMcEmu= net.minecraft.client.main.Main "]},
        "libraries": [{"name": "net.fabricmc:fabric-loader:0.17.3", "url": format!("{base}/maven/"), "sha1": sha1(&loader_jar), "size": loader_jar.len()}]
    });
    let _f2 = s.mock("GET", "/fabric/v2/versions/loader/1.21.11/0.17.3/profile/json").with_body(fabric_profile.to_string()).create_async().await;
    let _f3 = s.mock("GET", "/maven/net/fabricmc/fabric-loader/0.17.3/fabric-loader-0.17.3.jar").with_body(loader_jar.clone()).create_async().await;
    let rv = install::resolve_version(&ctx, "1.21.11", LoaderKind::Fabric, None).await.unwrap();
    assert_eq!(rv.id, "fabric-loader-0.17.3-1.21.11");
    assert_eq!(rv.loader_version.as_deref(), Some("0.17.3"));
    let inst = install::install(&ctx, rv, &[], false).await.unwrap();
    let args = launch::build_args(&ArgInputs {
        installed: &inst,
        java: &java,
        game_dir: &game_dir,
        auth: &a,
        memory_min_mb: 512,
        memory_max_mb: 2048,
        jvm_preset: "minimal",
        extra_jvm_args: vec![],
        resolution: Some((1280, 720)),
        fullscreen: true,
        libraries_dir: &ctx.paths.libraries_dir(),
        assets_dir: &ctx.paths.assets_dir(),
        join_server: None,
    });
    assert!(args.contains(&"net.fabricmc.loader.impl.launch.knot.KnotClient".to_string()));
    assert!(args.join(" ").contains("fabric-loader-0.17.3.jar"));
    assert!(args.contains(&"--fullscreen".to_string()));
}

// ------------------------------------------------------------------------ mods

struct ModrinthMock {
    _mocks: Vec<mockito::Mock>,
}

async fn mock_project(s: &mut mockito::ServerGuard, base: &str, id: &str, mod_id: &str, mc: &str, deps: serde_json::Value, jar: &[u8]) -> Vec<mockito::Mock> {
    let fname = format!("{mod_id}.jar");
    let version = json!({
        "id": format!("V{id}"), "project_id": id, "name": mod_id, "version_number": "1.0.0", "version_type": "release",
        "game_versions": [mc], "loaders": ["fabric"], "date_published": "2025-12-01T00:00:00Z", "downloads": 10,
        "dependencies": deps,
        "files": [{"hashes": {"sha1": sha1(jar), "sha512": sha512(jar)}, "url": format!("{base}/files/{fname}"), "filename": fname, "primary": true, "size": jar.len()}]
    });
    let project = json!({"id": id, "slug": mod_id, "title": format!("Mod {mod_id}"), "description": "d", "project_type": "mod", "game_versions": [mc], "loaders": ["fabric"], "client_side": "required", "server_side": "optional"});
    vec![
        s.mock("GET", rx(&format!("/modrinth/v2/project/{id}"))).with_body(project.to_string()).create_async().await,
        s.mock("GET", rx(&format!("/modrinth/v2/project/{id}/version"))).with_body(json!([version]).to_string()).create_async().await,
        s.mock("GET", rx(&format!("/modrinth/v2/version/V{id}"))).with_body(version.to_string()).create_async().await,
        s.mock("GET", format!("/files/{fname}").as_str()).with_body(jar.to_vec()).create_async().await,
    ]
}

fn scope(d: &Path, name: &str) -> ProfileScope {
    ProfileScope {
        profile_id: name.into(),
        game_dir: d.join(name).join("game"),
        data_dir: d.join(name).join("data"),
        target: Target::new("1.21.11", LoaderKind::Fabric),
    }
}

#[tokio::test]
async fn resolves_dependencies_installs_and_isolates_profiles() {
    let mut s = mockito::Server::new_async().await;
    let base = s.url();
    let d = tempfile::tempdir().unwrap();
    let (ctx, _) = ctx(d.path(), &base);
    let ja = jar_bytes(d.path(), r#"{"id":"moda","version":"1.0.0","depends":{"minecraft":"1.21.11","modb":"*"}}"#);
    let jb = jar_bytes(d.path(), r#"{"id":"modb","version":"1.0.0","depends":{"minecraft":">=1.21"}}"#);
    let mut mocks = vec![];
    mocks.extend(mock_project(&mut s, &base, "AAAA1111", "moda", "1.21.11", json!([{"project_id": "BBBB2222", "dependency_type": "required"}, {"project_id": "CCCC3333", "dependency_type": "optional"}]), &ja).await);
    mocks.extend(mock_project(&mut s, &base, "BBBB2222", "modb", "1.21.11", json!([]), &jb).await);
    mocks.push(s.mock("GET", rx("/modrinth/v2/project/CCCC3333")).with_body(json!({"id": "CCCC3333", "title": "Optional C", "description": "", "project_type": "mod"}).to_string()).create_async().await);
    let _keep = ModrinthMock { _mocks: mocks };

    let a = scope(d.path(), "A");
    let b = scope(d.path(), "B");
    std::fs::create_dir_all(a.folder(ContentKind::Mod)).unwrap();
    std::fs::create_dir_all(b.folder(ContentKind::Mod)).unwrap();
    let installed = manage::installed_set(&manage::scan_mods(&a).unwrap());
    let plan = resolver::resolve_modrinth(&ctx, &a.target, ContentKind::Mod, "AAAA1111", None, &installed, false).await.unwrap();
    assert!(!plan.blocked, "{:?}", plan.blocked_reason);
    assert_eq!(plan.files.len(), 2);
    assert_eq!(plan.files[1].reason, "dependency");
    assert_eq!(plan.optional[0].title, "Optional C");
    let report = manage::execute(&ctx, &a, &plan, builtin::embedded).await.unwrap();
    assert_eq!(report.installed.len(), 2);
    let mods = manage::scan_mods(&a).unwrap();
    assert_eq!(mods.len(), 2);
    assert!(mods.iter().all(|m| m.warnings.is_empty()), "{:?}", mods.iter().map(|m| &m.warnings).collect::<Vec<_>>());
    assert!(mods.iter().any(|m| m.source.as_ref().is_some_and(|s| s.source == "modrinth" && !s.explicit)), "dependency recorded as non-explicit");
    // isolation
    assert!(manage::scan_mods(&b).unwrap().is_empty());
    // reinstalling reports already installed instead of duplicating
    let installed = manage::installed_set(&manage::scan_mods(&a).unwrap());
    let again = resolver::resolve_modrinth(&ctx, &a.target, ContentKind::Mod, "AAAA1111", None, &installed, false).await.unwrap();
    assert!(again.files.is_empty() && !again.already_installed.is_empty());
}

#[tokio::test]
async fn blocks_incompatible_versions_conflicts_and_lying_metadata() {
    let mut s = mockito::Server::new_async().await;
    let base = s.url();
    let d = tempfile::tempdir().unwrap();
    let (ctx, _) = ctx(d.path(), &base);
    let old = jar_bytes(d.path(), r#"{"id":"oldmod","version":"1"}"#);
    let liar = jar_bytes(d.path(), r#"{"id":"liar","version":"1","depends":{"minecraft":"1.20.1"}}"#);
    let sod = jar_bytes(d.path(), r#"{"id":"sodium","version":"0.6.0"}"#);
    let mut keep = vec![];
    keep.extend(mock_project(&mut s, &base, "DDDD4444", "oldmod", "1.20.1", json!([]), &old).await);
    keep.extend(mock_project(&mut s, &base, "EEEE5555", "liar", "1.21.11", json!([]), &liar).await);
    keep.extend(mock_project(&mut s, &base, "AANobbMI", "sodium", "1.21.11", json!([]), &sod).await);

    let a = scope(d.path(), "A");
    std::fs::create_dir_all(a.folder(ContentKind::Mod)).unwrap();
    // wrong Minecraft version ⇒ blocked with a suggestion
    let plan = resolver::resolve_modrinth(&ctx, &a.target, ContentKind::Mod, "DDDD4444", None, &Default::default(), false).await.unwrap();
    assert!(plan.blocked);
    assert!(plan.blocked_reason.unwrap().contains("no release for Minecraft 1.21.11"));
    assert_eq!(plan.suggestion.unwrap().mc_versions, vec!["1.20.1"]);
    assert!(manage::execute(&ctx, &a, &resolver::InstallPlan { blocked: true, ..Default::default() }, builtin::embedded).await.is_err());

    // jar metadata contradicts the API ⇒ nothing installed
    let plan = resolver::resolve_modrinth(&ctx, &a.target, ContentKind::Mod, "EEEE5555", None, &Default::default(), false).await.unwrap();
    let err = manage::execute(&ctx, &a, &plan, builtin::embedded).await.unwrap_err();
    assert!(matches!(err, Error::Incompatible(_)), "{err}");
    assert!(std::fs::read_dir(a.folder(ContentKind::Mod)).unwrap().next().is_none(), "nothing was placed in mods/");

    // known conflict (Sodium vs OptiFabric already installed) ⇒ blocked
    make_jar(&a.folder(ContentKind::Mod).join("optifabric.jar"), r#"{"id":"optifabric","version":"1"}"#);
    let installed = manage::installed_set(&manage::scan_mods(&a).unwrap());
    let plan = resolver::resolve_modrinth(&ctx, &a.target, ContentKind::Mod, "AANobbMI", None, &installed, false).await.unwrap();
    assert!(plan.blocked && plan.conflicts.iter().any(|c| c.b == "optifabric"));

    // vanilla profile ⇒ mods refused
    let v = Target::new("1.21.11", LoaderKind::Vanilla);
    assert!(resolver::resolve_modrinth(&ctx, &v, ContentKind::Mod, "AANobbMI", None, &Default::default(), false).await.unwrap().blocked);
}

#[tokio::test]
async fn builtin_mods_install_with_fabric_api_dependency() {
    let mut s = mockito::Server::new_async().await;
    let base = s.url();
    let d = tempfile::tempdir().unwrap();
    let (ctx, _) = ctx(d.path(), &base);
    let api = jar_bytes(d.path(), r#"{"id":"fabric-api","version":"0.140.0+1.21.11","depends":{"minecraft":"1.21.11"}}"#);
    let _k = mock_project(&mut s, &base, "P7dR8mSH", "fabric-api", "1.21.11", json!([]), &api).await;
    let a = scope(d.path(), "A");
    std::fs::create_dir_all(a.folder(ContentKind::Mod)).unwrap();
    let (plan, notes) = builtin::plan(&ctx, &a.target, &Default::default()).await.unwrap();
    assert!(!plan.blocked, "{:?}", plan.blocked_reason);
    assert_eq!(plan.files.len(), 4, "three built-ins (incl. Shadow Menu) + Fabric API: {:?}", plan.files.iter().map(|f| &f.title).collect::<Vec<_>>());
    assert!(notes.iter().any(|n| n.contains("server-side")), "Always Shield note shown");
    manage::execute(&ctx, &a, &plan, builtin::embedded).await.unwrap();
    let mods = manage::scan_mods(&a).unwrap();
    let marlow = mods.iter().find(|m| m.meta.as_ref().is_some_and(|x| x.mod_id == "marlowcrystal")).unwrap();
    assert!(marlow.builtin && marlow.warnings.is_empty(), "{:?}", marlow.warnings);
    assert!(mods.iter().any(|m| m.meta.as_ref().is_some_and(|x| x.mod_id == "alwaysshield")));
    let menu = mods.iter().find(|m| m.meta.as_ref().is_some_and(|x| x.mod_id == "shadowmenu")).expect("Shadow Menu installed");
    assert!(menu.builtin && menu.warnings.is_empty(), "{:?}", menu.warnings);
    // The menu under its old name (and an older build) is swapped for the current file, recoverably.
    make_jar(&a.folder(ContentKind::Mod).join("sunny-menu-1.0.0.jar"), r#"{"id":"sunnymenu","version":"1.0.0"}"#);
    std::fs::remove_file(&menu.path).unwrap();
    make_jar(&a.folder(ContentKind::Mod).join("shadow-menu-0.9.0.jar"), r#"{"id":"shadowmenu","version":"0.9.0"}"#);
    crate::play::ensure_builtins(&ctx, &a).await.unwrap();
    let names: Vec<String> = manage::scan_mods(&a).unwrap().into_iter().map(|m| m.file_name).collect();
    assert!(names.contains(&menu.file_name) && !names.iter().any(|n| n.starts_with("sunny-menu") || n == "shadow-menu-0.9.0.jar"), "{names:?}");
    assert_eq!(manage::list_backups(&a, ContentKind::Mod).len(), 2, "old files kept in backups");
    // Not pinned for 26.2 and Modrinth (mock) has nothing ⇒ skipped with notes, never forced.
    let _none = s.mock("GET", Matcher::Regex("^/modrinth/v2/project/(ozpC8eDC|f5J5N4a3)".into())).with_status(404).create_async().await;
    let t = Target::new("26.2", LoaderKind::Fabric);
    let (plan, notes) = builtin::plan(&ctx, &t, &Default::default()).await.unwrap();
    assert!(plan.files.iter().all(|f| !f.builtin), "{:?}", plan.files);
    assert_eq!(notes.iter().filter(|n| n.contains("skipped")).count(), 2, "{notes:?}");
}

// ------------------------------------------------------------------------ modpacks

#[tokio::test]
async fn mrpack_install_update_and_traversal_protection() {
    let mut s = mockito::Server::new_async().await;
    let base = s.url();
    let d = tempfile::tempdir().unwrap();
    let (ctx, _) = ctx(d.path(), &base);
    let x = jar_bytes(d.path(), r#"{"id":"x","version":"1"}"#);
    let y = jar_bytes(d.path(), r#"{"id":"y","version":"1"}"#);
    let _mx = s.mock("GET", "/f/x.jar").with_body(x.clone()).create_async().await;
    let _my = s.mock("GET", "/f/y.jar").with_body(y.clone()).create_async().await;
    let evil_dl = s.mock("GET", "/f/evil.jar").expect(0).create_async().await;

    let file = |path: &str, data: &[u8], url: &str, client: &str| json!({"path": path, "hashes": {"sha1": sha1(data), "sha512": sha512(data)}, "env": {"client": client, "server": "required"}, "downloads": [url], "fileSize": data.len()});
    let index1 = json!({
        "formatVersion": 1, "game": "minecraft", "versionId": "1.0", "name": "Test Pack",
        "files": [
            file("mods/x.jar", &x, &format!("{base}/f/x.jar"), "required"),
            file("../evil.jar", &x, &format!("{base}/f/evil.jar"), "required"),
            file("mods/server.jar", &x, &format!("{base}/f/evil.jar"), "unsupported"),
            file("mods/badhost.jar", &x, "https://evil.example/x.jar", "required")
        ],
        "dependencies": {"minecraft": "1.21.11", "fabric-loader": "0.17.3"}
    });
    let p1 = d.path().join("pack1.mrpack");
    std::fs::write(&p1, zip_bytes(&[
        ("modrinth.index.json", index1.to_string().as_bytes()),
        ("overrides/config/a.txt", b"cfg"),
        ("overrides/../escape.txt", b"bad"),
        ("overrides/C:/Windows/evil.dll", b"bad"),
        ("client-overrides/options.txt", b"version:4671\nrenderDistance:8\n"),
    ])).unwrap();

    let summary = modpacks::inspect(&p1).unwrap();
    assert_eq!((summary.mc_version.as_str(), summary.loader), ("1.21.11", LoaderKind::Fabric));
    assert!(summary.warnings.iter().any(|w| w.contains("hosts")));
    let (profile, report) = modpacks::install(&ctx, &p1, None).await.unwrap();
    let game = Profiles::new(&ctx.paths).game_dir(&profile);
    assert!(game.join("mods/x.jar").is_file());
    assert!(game.join("config/a.txt").is_file());
    assert!(!d.path().join("data").join("instances").join("escape.txt").exists() && !game.parent().unwrap().join("escape.txt").exists());
    assert!(!game.join("mods/server.jar").exists() && !game.join("mods/badhost.jar").exists());
    assert!(report.skipped.len() >= 4, "{:?}", report.skipped);
    evil_dl.assert_async().await;
    assert_eq!(profile.loader_version.as_deref(), Some("0.17.3"));

    // Player changes settings and makes a world; then the pack updates.
    std::fs::write(game.join("options.txt"), "version:4671\nrenderDistance:12\n").unwrap();
    std::fs::create_dir_all(game.join("saves/My World")).unwrap();
    std::fs::write(game.join("saves/My World/level.dat"), b"world").unwrap();
    let index2 = json!({"formatVersion": 1, "game": "minecraft", "versionId": "2.0", "name": "Test Pack",
        "files": [file("mods/y.jar", &y, &format!("{base}/f/y.jar"), "required")],
        "dependencies": {"minecraft": "1.21.11", "fabric-loader": "0.17.3"}});
    let p2 = d.path().join("pack2.mrpack");
    std::fs::write(&p2, zip_bytes(&[("modrinth.index.json", index2.to_string().as_bytes()), ("overrides/options.txt", b"version:4671\nrenderDistance:2\n")])).unwrap();
    let r2 = modpacks::update(&ctx, &profile.id, &p2).await.unwrap();
    assert!(game.join("mods/y.jar").is_file() && !game.join("mods/x.jar").exists());
    assert!(std::fs::read_to_string(game.join("options.txt")).unwrap().contains("renderDistance:12"), "personal options kept");
    assert_eq!(std::fs::read(game.join("saves/My World/level.dat")).unwrap(), b"world");
    assert!(r2.backup.as_ref().unwrap().is_file());

    // Updating to a different Minecraft version is refused.
    let index3 = json!({"formatVersion": 1, "game": "minecraft", "versionId": "3.0", "name": "Test Pack", "files": [], "dependencies": {"minecraft": "26.2", "fabric-loader": "0.18.0"}});
    let p3 = d.path().join("pack3.mrpack");
    std::fs::write(&p3, zip_bytes(&[("modrinth.index.json", index3.to_string().as_bytes())])).unwrap();
    assert!(matches!(modpacks::update(&ctx, &profile.id, &p3).await, Err(Error::Incompatible(_))));

    // Export round-trip.
    let out = modpacks::export_mrpack(&ctx, &profile.id, &d.path().join("export.mrpack"), false).unwrap();
    let back = modpacks::inspect(&out).unwrap();
    assert_eq!(back.mc_version, "1.21.11");
    assert!(modpacks::embedded_profile_settings(&out).is_some());

    // Forge packs are refused with an explanation (no installer is executed).
    let forge = json!({"formatVersion": 1, "game": "minecraft", "versionId": "1", "name": "F", "files": [], "dependencies": {"minecraft": "1.20.1", "forge": "47.2.0"}});
    let pf = d.path().join("forge.mrpack");
    std::fs::write(&pf, zip_bytes(&[("modrinth.index.json", forge.to_string().as_bytes())])).unwrap();
    assert!(matches!(modpacks::install(&ctx, &pf, None).await, Err(Error::Incompatible(_))));
}

#[tokio::test]
async fn curseforge_pack_requires_key_and_lists_manual_files() {
    let mut s = mockito::Server::new_async().await;
    let base = s.url();
    let d = tempfile::tempdir().unwrap();
    let (ctx, _) = ctx(d.path(), &base);
    let manifest = json!({"manifestType": "minecraftModpack", "manifestVersion": 1, "name": "CF Pack", "version": "1", "author": "me",
        "minecraft": {"version": "1.21.11", "modLoaders": [{"id": "fabric-0.17.3", "primary": true}]},
        "files": [{"projectID": 1, "fileID": 11, "required": true}, {"projectID": 2, "fileID": 22, "required": true}], "overrides": "overrides"});
    let pack = d.path().join("cf.zip");
    std::fs::write(&pack, zip_bytes(&[("manifest.json", manifest.to_string().as_bytes()), ("overrides/config/c.txt", b"x")])).unwrap();
    if std::env::var("SHADOW_CURSEFORGE_API_KEY").is_err() {
        assert!(matches!(modpacks::install(&ctx, &pack, None).await, Err(Error::MissingConfig(_))), "no key ⇒ clear setup error");
    }
    ctx.secrets.set(crate::secrets::CURSEFORGE_KEY, "test-key").unwrap();
    let j = jar_bytes(d.path(), r#"{"id":"cfmod","version":"1"}"#);
    let files = json!({"data": [
        {"id": 11, "modId": 1, "displayName": "A", "fileName": "a.jar", "releaseType": 1, "fileDate": "2025-01-01", "fileLength": j.len(), "downloadUrl": format!("{base}/cf/a.jar"), "gameVersions": ["1.21.11", "Fabric"], "hashes": [{"value": sha1(&j), "algo": 1}]},
        {"id": 22, "modId": 2, "displayName": "B", "fileName": "b.jar", "releaseType": 1, "fileDate": "2025-01-01", "fileLength": 5, "downloadUrl": null, "gameVersions": ["1.21.11", "Fabric"], "hashes": []}
    ]});
    let mods = json!({"data": [
        {"id": 1, "name": "Mod A", "slug": "mod-a", "summary": "", "classId": 6, "links": {"websiteUrl": "https://www.curseforge.com/minecraft/mc-mods/mod-a"}},
        {"id": 2, "name": "Mod B", "slug": "mod-b", "summary": "", "classId": 6, "links": {"websiteUrl": "https://www.curseforge.com/minecraft/mc-mods/mod-b"}}
    ]});
    let key_check = s.mock("POST", "/curseforge/v1/mods/files").match_header("x-api-key", "test-key").with_body(files.to_string()).create_async().await;
    let _m = s.mock("POST", "/curseforge/v1/mods").with_body(mods.to_string()).create_async().await;
    let _d = s.mock("GET", "/cf/a.jar").with_body(j.clone()).create_async().await;
    let (profile, report) = modpacks::install(&ctx, &pack, None).await.unwrap();
    key_check.assert_async().await;
    let game = Profiles::new(&ctx.paths).game_dir(&profile);
    assert!(game.join("mods/a.jar").is_file());
    assert_eq!(report.manual.len(), 1);
    assert_eq!(report.manual[0].name, "Mod B");
    assert!(report.manual[0].url.contains("mod-b/files/22"));
    assert!(game.join("config/c.txt").is_file());
}

// ------------------------------------------------------------------------ auth

async fn msa_server(xsts_status: usize, xsts_body: serde_json::Value, entitlements: serde_json::Value, profile_status: usize) -> mockito::ServerGuard {
    let mut s = mockito::Server::new_async().await;
    s.mock("POST", "/msa/devicecode")
        .with_body(json!({"device_code": "dc", "user_code": "ABCD-1234", "verification_uri": "https://microsoft.com/link", "expires_in": 60, "interval": 1}).to_string())
        .create_async()
        .await;
    s.mock("POST", "/msa/token").with_body(json!({"access_token": "msa-access", "refresh_token": "M.C123_BAY.refresh-token-value-long-enough"}).to_string()).create_async().await;
    s.mock("POST", "/xbox/user").with_body(json!({"Token": "xbl", "DisplayClaims": {"xui": [{"uhs": "4242"}]}}).to_string()).create_async().await;
    s.mock("POST", "/xbox/xsts").with_status(xsts_status).with_body(xsts_body.to_string()).create_async().await;
    s.mock("POST", "/mc/authentication/login_with_xbox").with_body(json!({"access_token": "mc-token-abc", "expires_in": 86400}).to_string()).create_async().await;
    s.mock("GET", "/mc/entitlements/mcstore").with_body(entitlements.to_string()).create_async().await;
    s.mock("GET", "/mc/minecraft/profile").with_status(profile_status).with_body(json!({"id": "0123456789abcdef0123456789abcdef", "name": "RealPlayer"}).to_string()).create_async().await;
    s
}

#[tokio::test]
async fn microsoft_login_success_and_error_paths() {
    // XSTS: no Xbox profile
    let s = msa_server(401, json!({"XErr": 2148916233u64}), json!({"items": [{"name": "game_minecraft"}]}), 200).await;
    let d = tempfile::tempdir().unwrap();
    let (c, _) = ctx(d.path(), &s.url());
    let code = auth::start_device_login(&c, "00000000-0000-0000-0000-000000000000").await.unwrap();
    assert_eq!(code.user_code, "ABCD-1234");
    let err = auth::finish_device_login(&c, "client", &code, Default::default()).await.unwrap_err();
    assert!(err.to_string().contains("no Xbox profile"), "{err}");
    assert!(auth::list_accounts(&c).unwrap().is_empty(), "nothing stored on failure");

    // Does not own the game
    let s = msa_server(200, json!({"Token": "xsts", "DisplayClaims": {"xui": [{"uhs": "4242", "xid": "99"}]}}), json!({"items": []}), 404).await;
    let d = tempfile::tempdir().unwrap();
    let (c, _) = ctx(d.path(), &s.url());
    let code = auth::start_device_login(&c, "client").await.unwrap();
    let err = auth::finish_device_login(&c, "client", &code, Default::default()).await.unwrap_err();
    assert!(err.to_string().contains("does not own"), "{err}");

    // Success: account stored encrypted, launch credentials returned without refresh
    let s = msa_server(200, json!({"Token": "xsts", "DisplayClaims": {"xui": [{"uhs": "4242", "xid": "99"}]}}), json!({"items": [{"name": "game_minecraft"}]}), 200).await;
    let d = tempfile::tempdir().unwrap();
    let (c, _) = ctx(d.path(), &s.url());
    let code = auth::start_device_login(&c, "client").await.unwrap();
    let acc = auth::finish_device_login(&c, "client", &code, Default::default()).await.unwrap();
    assert_eq!(acc.username, "RealPlayer");
    let la = auth::launch_auth(&c, &acc.id).await.unwrap();
    assert_eq!(la.access_token, "mc-token-abc");
    assert!(!la.offline);
    let raw = std::fs::read(c.paths.secrets_file()).unwrap();
    assert!(!String::from_utf8_lossy(&raw).contains("mc-token-abc"), "tokens are encrypted at rest");
    assert!(!format!("{la:?}").contains("mc-token-abc"), "debug output never shows tokens");
    auth::remove_account(&c, &acc.id).unwrap();
    assert!(auth::launch_auth(&c, &acc.id).await.is_err());

    // Offline accounts are validated
    assert!(auth::add_offline(&c, "a b").is_err());
    let off = auth::add_offline(&c, "Tester_1").unwrap();
    assert!(auth::launch_auth(&c, &off.id).await.unwrap().offline);
}

// ------------------------------------------------------------------------ launch

fn fake_java(dir: &Path, script: &str) -> JavaInstall {
    let p = dir.join(format!("fake-java-{}.bat", uuid::Uuid::new_v4().simple()));
    std::fs::write(&p, format!("@echo off\r\n{script}\r\n")).unwrap();
    JavaInstall { path: p, home: dir.to_path_buf(), version: "21".into(), major: 21, vendor: None, arch: "x64".into(), source: "test".into() }
}

#[cfg(windows)]
#[tokio::test]
async fn launch_failure_is_reported_not_claimed_as_success() {
    let d = tempfile::tempdir().unwrap();
    let (c, sink) = ctx(d.path(), "http://127.0.0.1:9");
    let profiles = Profiles::new(&c.paths);
    let p = profiles.create(NewProfile { name: "T".into(), kind: ProfileKind::Vanilla, mc_version: "1.21.11".into(), loader: LoaderKind::Vanilla, loader_version: None, game_dir: None, preset: None, builtin_mods: None, ingame_features: None }).unwrap();
    let games = Arc::new(GameManager::default());
    let java = fake_java(d.path(), "echo Error: Could not reserve enough space for 8388608KB object heap\r\nexit /b 1");
    let err = games.spawn(&c, &p.id, &java, vec![], &profiles.game_dir(&p), Duration::from_secs(20)).await.unwrap_err();
    assert!(err.to_string().contains("memory"), "{err}");
    assert_eq!(profiles.get(&p.id).unwrap().last_launch.unwrap().result, "failed");
    assert!(sink.names().iter().any(|n| n == "game:state"));
    assert!(!games.is_running(&p.id));

    // A process that reaches initialisation is reported running, then exited normally.
    let java = fake_java(d.path(), "echo [12:00:00] [Render thread/INFO]: Setting user: Tester\r\nping -n 3 127.0.0.1 >nul\r\nexit /b 0");
    let st = games.spawn(&c, &p.id, &java, vec![], &profiles.game_dir(&p), Duration::from_secs(20)).await.unwrap();
    assert_eq!(st.state, "running");
    assert!(st.message.is_none(), "startup was confirmed by the log");
    for _ in 0..100 {
        if !games.is_running(&p.id) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!games.is_running(&p.id));
    tokio::time::sleep(Duration::from_millis(200)).await;
    let rec = profiles.get(&p.id).unwrap().last_launch.unwrap();
    assert_eq!(rec.result, "ok", "{rec:?}");
    let log = std::fs::read_dir(c.paths.logs_dir().join("game")).unwrap().count();
    assert!(log >= 2);
}

#[cfg(windows)]
#[tokio::test]
async fn game_log_redacts_tokens() {
    let d = tempfile::tempdir().unwrap();
    let (c, _) = ctx(d.path(), "http://127.0.0.1:9");
    let games = Arc::new(GameManager::default());
    let java = fake_java(d.path(), "echo Setting user: X\r\necho Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U\r\nping -n 2 127.0.0.1 >nul");
    games.spawn(&c, "p1", &java, vec!["--accessToken".into(), "super-secret-token".into()], &d.path().join("g"), Duration::from_secs(20)).await.unwrap();
    for _ in 0..60 {
        if !games.is_running("p1") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let dir = c.paths.logs_dir().join("game");
    let f = std::fs::read_dir(&dir).unwrap().next().unwrap().unwrap().path();
    let text = std::fs::read_to_string(f).unwrap();
    assert!(!text.contains("super-secret-token") && !text.contains("eyJzdWIi"), "{text}");
}
