//! Server list: Java servers you save in the launcher (pinged with the game's own "server list
//! ping", joined with one click through Quick Play) and Bedrock pings over RakNet. No SRV lookup:
//! servers that only publish an SRV record need their real host:port typed in.

use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

use crate::{util, Error, Result};

const TIMEOUT: Duration = Duration::from_secs(4);
const FILE: &str = "servers.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedServer {
    pub id: String,
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub online: bool,
    pub ping_ms: Option<u32>,
    pub motd: String,
    pub version: String,
    pub players_online: Option<u32>,
    pub players_max: Option<u32>,
    /// data:image/png;base64,… (Java servers only)
    pub icon: Option<String>,
    pub error: Option<String>,
}

// ------------------------------------------------------------------ saved list

pub fn list(root: &Path) -> Vec<SavedServer> {
    util::read_json_opt(&root.join(FILE)).ok().flatten().unwrap_or_default()
}

pub fn save(root: &Path, mut servers: Vec<SavedServer>) -> Result<Vec<SavedServer>> {
    for s in &mut servers {
        s.name = s.name.trim().to_string();
        s.address = s.address.trim().to_string();
        if s.address.is_empty() || s.address.len() > 255 || s.address.contains(char::is_whitespace) {
            return Err(Error::Invalid(format!("\"{}\" isn't a server address", s.address)));
        }
        parse_address(&s.address, 25565)?;
        if s.name.is_empty() {
            s.name = s.address.clone();
        }
        if s.id.is_empty() {
            s.id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
        }
    }
    util::write_json(&root.join(FILE), &servers)?;
    Ok(servers)
}

/// "host", "host:port", "[ipv6]:port" → (host, port).
pub fn parse_address(address: &str, default_port: u16) -> Result<(String, u16)> {
    let a = address.trim();
    let bad = || Error::Invalid(format!("\"{a}\" isn't a server address"));
    if let Some(rest) = a.strip_prefix('[') {
        let (host, after) = rest.split_once(']').ok_or_else(bad)?;
        let port = match after.strip_prefix(':') {
            Some(p) => p.parse().map_err(|_| bad())?,
            None if after.is_empty() => default_port,
            None => return Err(bad()),
        };
        return Ok((host.to_string(), port));
    }
    match a.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => Ok((host.to_string(), port.parse().map_err(|_| bad())?)),
        Some(_) => Ok((a.to_string(), default_port)), // a bare IPv6 address
        None if !a.is_empty() => Ok((a.to_string(), default_port)),
        None => Err(bad()),
    }
}

// ------------------------------------------------------------------ Java: server list ping

fn write_varint(buf: &mut Vec<u8>, value: i32) {
    let mut u = value as u32;
    loop {
        if u & !0x7F == 0 {
            buf.push(u as u8);
            return;
        }
        buf.push(((u & 0x7F) | 0x80) as u8);
        u >>= 7;
    }
}

async fn read_varint<R: AsyncRead + Unpin>(r: &mut R) -> Result<i32> {
    let mut value = 0i32;
    for i in 0..5 {
        let b = r.read_u8().await?;
        value |= ((b & 0x7F) as i32) << (7 * i);
        if b & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(Error::msg("The server sent a malformed reply"))
}

fn packet(id: i32, body: &[u8]) -> Vec<u8> {
    let mut inner = vec![];
    write_varint(&mut inner, id);
    inner.extend_from_slice(body);
    let mut out = vec![];
    write_varint(&mut out, inner.len() as i32);
    out.extend(inner);
    out
}

/// Ask a Java server for its status (MOTD, players, version, icon) and measure the ping.
pub async fn ping_java(address: &str) -> ServerStatus {
    match tokio::time::timeout(TIMEOUT * 2, ping_java_inner(address)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => ServerStatus { error: Some(e.to_string()), ..Default::default() },
        Err(_) => ServerStatus { error: Some("No answer (timed out)".into()), ..Default::default() },
    }
}

async fn ping_java_inner(address: &str) -> Result<ServerStatus> {
    let (host, port) = parse_address(address, 25565)?;
    let mut stream = tokio::time::timeout(TIMEOUT, tokio::net::TcpStream::connect((host.as_str(), port)))
        .await
        .map_err(|_| Error::msg("Couldn't connect (timed out)"))?
        .map_err(|e| Error::msg(format!("Couldn't connect: {e}")))?;
    let mut hs = vec![];
    write_varint(&mut hs, 767); // protocol: any number works for a status request
    write_varint(&mut hs, host.len() as i32);
    hs.extend_from_slice(host.as_bytes());
    hs.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut hs, 1); // next state: status
    stream.write_all(&packet(0x00, &hs)).await?;
    stream.write_all(&packet(0x00, &[])).await?;

    let _len = read_varint(&mut stream).await?;
    if read_varint(&mut stream).await? != 0x00 {
        return Err(Error::msg("The server sent an unexpected reply"));
    }
    let json_len = read_varint(&mut stream).await?;
    if !(0..=1 << 21).contains(&json_len) {
        return Err(Error::msg("The server sent a malformed reply"));
    }
    let mut json = vec![0u8; json_len as usize];
    stream.read_exact(&mut json).await?;
    let mut status = parse_java_status(&String::from_utf8_lossy(&json))?;

    let t0 = Instant::now();
    stream.write_all(&packet(0x01, &42i64.to_be_bytes())).await?;
    let pong = async {
        let _len = read_varint(&mut stream).await?;
        let _id = read_varint(&mut stream).await?;
        stream.read_i64().await?;
        Ok::<_, Error>(())
    };
    if tokio::time::timeout(TIMEOUT, pong).await.is_ok_and(|r| r.is_ok()) {
        status.ping_ms = Some(t0.elapsed().as_millis().min(u32::MAX as u128) as u32);
    }
    Ok(status)
}

fn parse_java_status(json: &str) -> Result<ServerStatus> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| Error::msg(format!("The server's status couldn't be read ({e})")))?;
    Ok(ServerStatus {
        online: true,
        ping_ms: None,
        motd: strip_codes(&chat_text(&v["description"])).trim().to_string(),
        version: strip_codes(v["version"]["name"].as_str().unwrap_or_default()),
        players_online: v["players"]["online"].as_u64().map(|n| n as u32),
        players_max: v["players"]["max"].as_u64().map(|n| n as u32),
        icon: v["favicon"].as_str().filter(|f| f.starts_with("data:image/png;base64,") && f.len() < 200_000).map(String::from),
        error: None,
    })
}

/// Flatten a chat component (string, {"text", "extra"} or a list) into plain text.
fn chat_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(a) => a.iter().map(chat_text).collect(),
        serde_json::Value::Object(o) => {
            let mut s = o.get("text").and_then(|t| t.as_str()).unwrap_or_default().to_string();
            if let Some(extra) = o.get("extra") {
                s.push_str(&chat_text(extra));
            }
            s
        }
        _ => String::new(),
    }
}

/// Remove § formatting codes.
fn strip_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut skip = false;
    for c in s.chars() {
        if skip {
            skip = false;
        } else if c == '§' {
            skip = true;
        } else {
            out.push(c);
        }
    }
    out
}

// ------------------------------------------------------------------ Bedrock: RakNet unconnected ping

const RAKNET_MAGIC: [u8; 16] = [0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78];

pub async fn ping_bedrock(address: &str) -> ServerStatus {
    match tokio::time::timeout(TIMEOUT, ping_bedrock_inner(address)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => ServerStatus { error: Some(e.to_string()), ..Default::default() },
        Err(_) => ServerStatus { error: Some("No answer (timed out)".into()), ..Default::default() },
    }
}

async fn ping_bedrock_inner(address: &str) -> Result<ServerStatus> {
    let (host, port) = parse_address(address, 19132)?;
    let socket = tokio::net::UdpSocket::bind(if host.contains(':') { "[::]:0" } else { "0.0.0.0:0" }).await?;
    socket.connect((host.as_str(), port)).await.map_err(|e| Error::msg(format!("Couldn't reach the server: {e}")))?;
    let mut ping = vec![0x01];
    ping.extend_from_slice(&(chrono::Utc::now().timestamp_millis()).to_be_bytes());
    ping.extend_from_slice(&RAKNET_MAGIC);
    ping.extend_from_slice(&0x5348_4144_4f57u64.to_be_bytes()); // client guid
    let t0 = Instant::now();
    socket.send(&ping).await?;
    let mut buf = vec![0u8; 2048];
    let n = socket.recv(&mut buf).await?;
    let mut status = parse_bedrock_pong(&buf[..n])?;
    status.ping_ms = Some(t0.elapsed().as_millis().min(u32::MAX as u128) as u32);
    Ok(status)
}

/// Unconnected pong: 0x1C, time, server guid, magic, then "MCPE;motd;protocol;version;online;max;…".
fn parse_bedrock_pong(b: &[u8]) -> Result<ServerStatus> {
    let bad = || Error::msg("The server sent a malformed reply");
    if b.len() < 35 || b[0] != 0x1C || b[17..33] != RAKNET_MAGIC {
        return Err(bad());
    }
    let len = u16::from_be_bytes([b[33], b[34]]) as usize;
    let text = String::from_utf8_lossy(b.get(35..35 + len).ok_or_else(bad)?).into_owned();
    let f: Vec<&str> = text.split(';').collect();
    if f.len() < 6 {
        return Err(bad());
    }
    let sub = f.get(7).copied().unwrap_or_default();
    let motd = if sub.is_empty() { strip_codes(f[1]) } else { format!("{}\n{}", strip_codes(f[1]), strip_codes(sub)) };
    Ok(ServerStatus {
        online: true,
        ping_ms: None,
        motd,
        version: f[3].to_string(),
        players_online: f[4].parse().ok(),
        players_max: f[5].parse().ok(),
        icon: None,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(parse_address("play.example.net", 25565).unwrap(), ("play.example.net".into(), 25565));
        assert_eq!(parse_address("1.2.3.4:25570", 25565).unwrap(), ("1.2.3.4".into(), 25570));
        assert_eq!(parse_address("[::1]:19133", 19132).unwrap(), ("::1".into(), 19133));
        assert_eq!(parse_address("::1", 19132).unwrap(), ("::1".into(), 19132));
        assert!(parse_address("host:notaport", 25565).is_err());
        assert!(parse_address("", 25565).is_err());
    }

    #[test]
    fn varints_round_trip() {
        for v in [0, 1, 127, 128, 255, 25565, 2097151, i32::MAX, -1] {
            let mut buf = vec![];
            write_varint(&mut buf, v);
            let got = tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async { read_varint(&mut buf.as_slice()).await.unwrap() });
            assert_eq!(got, v);
        }
    }

    #[tokio::test]
    async fn java_status_and_ping_against_a_local_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            // handshake + status request
            for _ in 0..2 {
                let len = read_varint(&mut s).await.unwrap();
                let mut body = vec![0u8; len as usize];
                s.read_exact(&mut body).await.unwrap();
            }
            let json = r#"{"version":{"name":"Paper 1.21.11","protocol":774},"players":{"max":100,"online":7},"description":{"text":"§aShadow ","extra":[{"text":"Test"}]},"favicon":"data:image/png;base64,AAAA"}"#;
            let mut body = vec![];
            write_varint(&mut body, json.len() as i32);
            body.extend_from_slice(json.as_bytes());
            s.write_all(&packet(0x00, &body)).await.unwrap();
            // ping → pong
            let len = read_varint(&mut s).await.unwrap();
            let mut body = vec![0u8; len as usize];
            s.read_exact(&mut body).await.unwrap();
            s.write_all(&packet(0x01, &body[1..])).await.unwrap();
        });
        let st = ping_java(&format!("127.0.0.1:{port}")).await;
        assert!(st.online, "{:?}", st.error);
        assert_eq!(st.motd, "Shadow Test");
        assert_eq!(st.version, "Paper 1.21.11");
        assert_eq!((st.players_online, st.players_max), (Some(7), Some(100)));
        assert!(st.ping_ms.is_some() && st.icon.is_some());
    }

    #[tokio::test]
    async fn closed_port_is_reported_not_hung() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let st = ping_java(&format!("127.0.0.1:{port}")).await;
        assert!(!st.online && st.error.is_some());
    }

    #[tokio::test]
    async fn bedrock_pong_against_a_local_server() {
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = server.local_addr().unwrap().port();
        tokio::spawn(async move {
            let mut buf = [0u8; 64];
            let (_, from) = server.recv_from(&mut buf).await.unwrap();
            let text = "MCPE;§bShadow Realm;766;1.21.120;3;20;123456;Lobby;Survival;1;19132;19133;";
            let mut pong = vec![0x1C];
            pong.extend_from_slice(&buf[1..9]);
            pong.extend_from_slice(&7u64.to_be_bytes());
            pong.extend_from_slice(&RAKNET_MAGIC);
            pong.extend_from_slice(&(text.len() as u16).to_be_bytes());
            pong.extend_from_slice(text.as_bytes());
            server.send_to(&pong, from).await.unwrap();
        });
        let st = ping_bedrock(&format!("127.0.0.1:{port}")).await;
        assert!(st.online, "{:?}", st.error);
        assert_eq!(st.motd, "Shadow Realm\nLobby");
        assert_eq!(st.version, "1.21.120");
        assert_eq!((st.players_online, st.players_max), (Some(3), Some(20)));
    }

    #[test]
    fn saved_list_validates_and_fills_in() {
        let d = tempfile::tempdir().unwrap();
        let saved = save(d.path(), vec![SavedServer { id: String::new(), name: " ".into(), address: " play.example.net ".into() }]).unwrap();
        assert_eq!(saved[0].name, "play.example.net");
        assert_eq!(saved[0].id.len(), 12);
        assert_eq!(list(d.path()), saved);
        assert!(save(d.path(), vec![SavedServer { id: "x".into(), name: "a".into(), address: "has space".into() }]).is_err());
    }
}
