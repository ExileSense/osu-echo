
use crate::handlers::cho;
use crate::logger;
use crate::state::AppState;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

const PORTS: [u16; 4] = [13380, 13381, 13382, 13383];

pub async fn run(state: Arc<RwLock<AppState>>) {
    for port in PORTS {
        let state = state.clone();
        tokio::spawn(async move { listen(state, port).await });
    }
}

async fn listen(state: Arc<RwLock<AppState>>, port: u16) {
    let listener = match TcpListener::bind(("127.0.0.1", port)).await {
        Ok(l) => l,
        Err(e) => {
            logger::info(&format!("Could not bind TCP bancho port {} ({}). Skipping it.", port, e));
            return;
        }
    };
    logger::success(&format!("TCP bancho listening on 127.0.0.1:{} (for very old clients)", port));
    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => continue,
        };
        logger::info(&format!("[tcp-bancho] connection from {} on port {}", addr, port));
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = connection(state, stream).await {
                logger::info(&format!("[tcp-bancho] connection ended: {}", e));
            }
        });
    }
}

fn describe(buf: &[u8]) -> String {
    let mut out = Vec::new();
    let mut pos = 0;
    while buf.len() - pos >= 7 {
        let id = u16::from_le_bytes([buf[pos], buf[pos + 1]]);
        let len = u32::from_le_bytes([buf[pos + 3], buf[pos + 4], buf[pos + 5], buf[pos + 6]]) as usize;
        out.push(format!("{}:{}", id, len));
        pos += 7 + len;
    }
    out.join(" ")
}

fn complete_len(buf: &[u8]) -> usize {
    let mut pos = 0;
    while buf.len() - pos >= 7 {
        let len = u32::from_le_bytes([buf[pos + 3], buf[pos + 4], buf[pos + 5], buf[pos + 6]]) as usize;
        if buf.len() - pos < 7 + len {
            break;
        }
        pos += 7 + len;
    }
    pos
}

async fn connection(state: Arc<RwLock<AppState>>, mut sock: TcpStream) -> Result<(), String> {
    let _ = sock.set_nodelay(true);
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 4096];

    let login_end = loop {
        let nl: Vec<usize> = buf.iter().enumerate().filter(|(_, b)| **b == b'\n').map(|(i, _)| i).collect();
        if nl.len() >= 3 {
            break nl[2] + 1;
        }
        if buf.len() > 8192 {
            return Err("login too long".to_string());
        }
        let n = tokio::time::timeout(Duration::from_secs(15), sock.read(&mut tmp))
            .await
            .map_err(|_| "login timed out".to_string())?
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("closed before login".to_string());
        }
        buf.extend_from_slice(&tmp[..n]);
    };
    let login_text: Vec<u8> = buf[..login_end].iter().copied().filter(|b| *b != b'\r').collect();
    buf.drain(..login_end);

    let (reply, token) = cho::tcp_login(state.clone(), &login_text).await;
    logger::info(&format!("[tcp-bancho] sent login reply packets {}", describe(&reply)));
    sock.write_all(&reply).await.map_err(|e| e.to_string())?;
    if token == "fail" {
        return Err("login failed".to_string());
    }
    logger::info("[tcp-bancho] client logged in");

    let mut tick = tokio::time::interval(Duration::from_millis(500));
    let mut last_keepalive = std::time::Instant::now();
    loop {
        tokio::select! {
            read = sock.read(&mut tmp) => {
                let n = read.map_err(|e| e.to_string())?;
                if n == 0 {
                    return Err("client disconnected".to_string());
                }
                buf.extend_from_slice(&tmp[..n]);
                let done = complete_len(&buf);
                if done > 0 {
                    let chunk: Vec<u8> = buf.drain(..done).collect();
                    logger::info(&format!("[tcp-bancho] client sent {}", describe(&chunk)));
                    let resp = cho::handle(state.clone(), Some("tcp"), &chunk).await;
                    if !resp.body.is_empty() {
                        logger::info(&format!("[tcp-bancho] server sent {}", describe(&resp.body)));
                        sock.write_all(&resp.body).await.map_err(|e| e.to_string())?;
                    }
                }
            }
            _ = tick.tick() => {
                if state.read().await.player.is_none() {
                    return Err("player logged out".to_string());
                }
                if last_keepalive.elapsed() >= Duration::from_secs(12) {
                    last_keepalive = std::time::Instant::now();
                    let mut s = state.write().await;
                    if let Some(ref mut p) = s.player {
                        if !p.is_restricted {
                            p.enqueue_stats();
                        }
                    }
                }
                let resp = cho::handle(state.clone(), Some("tcp"), &[]).await;
                if !resp.body.is_empty() {
                    logger::info(&format!("[tcp-bancho] server sent {}", describe(&resp.body)));
                    sock.write_all(&resp.body).await.map_err(|e| e.to_string())?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::complete_len;

    #[test]
    fn only_complete_packets_are_counted() {
        let mut data = crate::packets::write_packet(4, &[]);
        data.extend_from_slice(&crate::packets::write_packet(1, b"abc"));
        let full = data.len();
        data.extend_from_slice(&[9, 0, 0, 5, 0, 0, 0, 1]);
        assert_eq!(complete_len(&data), full);
    }
}
