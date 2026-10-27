use crate::packets::PacketReader;

static DEBUG_FLAGS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

pub fn reload_debug_flags() {
    let flags: Vec<String> = std::fs::read_to_string("legacy_debug.txt")
        .unwrap_or_default()
        .split(|c: char| c == ',' || c.is_whitespace())
        .map(|f| f.trim().to_lowercase())
        .filter(|f| !f.is_empty())
        .collect();
    if let Ok(mut g) = DEBUG_FLAGS.lock() {
        *g = flags;
    }
}

fn flag(name: &str) -> bool {
    DEBUG_FLAGS.lock().map(|g| g.iter().any(|f| f == name)).unwrap_or(false)
}

fn leading_id(payload: &[u8]) -> Option<i32> {
    payload.get(..4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub const LEGACY_MAX_BUILD: i64 = 20120722;
pub const LEGACY_PROTOCOL_VERSION: i32 = 7;

pub fn is_legacy_build(build: Option<i64>) -> bool {
    matches!(build, Some(b) if b > 0 && b <= LEGACY_MAX_BUILD)
}

fn put_string(out: &mut Vec<u8>, s: &str) {
    if s.is_empty() {
        out.push(0);
        return;
    }
    out.push(0x0b);
    let mut len = s.len();
    loop {
        let mut byte = (len & 0x7f) as u8;
        len >>= 7;
        if len != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if len == 0 {
            break;
        }
    }
    out.extend_from_slice(s.as_bytes());
}

fn frame(id: u16, payload: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(&id.to_le_bytes());
    out.push(0);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
}

fn stats_to_legacy(payload: &[u8]) -> Option<Vec<u8>> {
    let mut r = PacketReader::new(payload);
    let id = r.read_i32().ok()?;
    let action = r.read_u8().ok()?;
    let text = r.read_string().ok()?;
    let md5 = r.read_string().ok()?;
    let mods = r.read_u32().ok()?;
    let mode = r.read_u8().ok()?;
    let map_id = r.read_i32().ok()?;
    let ranked = r.take(8).ok()?.to_vec();
    let acc = r.take(4).ok()?.to_vec();
    let plays = r.take(4).ok()?.to_vec();
    let total = r.take(8).ok()?.to_vec();
    let rank = r.take(4).ok()?.to_vec();
    let mut o = Vec::new();
    o.extend_from_slice(&id.to_le_bytes());
    o.push(action);
    put_string(&mut o, &text);
    put_string(&mut o, &md5);
    o.extend_from_slice(&(mods as u16).to_le_bytes());
    o.push(mode);
    o.extend_from_slice(&map_id.to_le_bytes());
    o.extend_from_slice(&ranked);
    o.extend_from_slice(&acc);
    o.extend_from_slice(&plays);
    o.extend_from_slice(&total);
    o.extend_from_slice(&rank);
    Some(o)
}

fn presence_to_legacy(payload: &[u8]) -> Option<Vec<u8>> {
    let mut r = PacketReader::new(payload);
    let id = r.read_i32().ok()?;
    let name = r.read_string().ok()?;
    let tz = r.read_u8().ok()?;
    let country = r.read_u8().ok()?;
    let privs = r.read_u8().ok()?;
    let lon = r.take(4).ok()?.to_vec();
    let lat = r.take(4).ok()?.to_vec();
    let rank = r.take(4).ok()?.to_vec();
    let mut o = Vec::new();
    o.extend_from_slice(&id.to_le_bytes());
    put_string(&mut o, &name);
    o.push(1);
    o.push(tz);
    o.push(country);
    put_string(&mut o, "");
    o.push(privs & 0x1f);
    o.extend_from_slice(&lon);
    o.extend_from_slice(&lat);
    o.extend_from_slice(&rank);
    Some(o)
}

fn quit_to_legacy(payload: &[u8]) -> Option<Vec<u8>> {
    let mut r = PacketReader::new(payload);
    let id = r.read_i32().ok()?;
    let mut o = Vec::new();
    o.extend_from_slice(&id.to_le_bytes());
    o.push(0);
    put_string(&mut o, "");
    put_string(&mut o, "");
    o.extend_from_slice(&0u16.to_le_bytes());
    o.push(0);
    o.extend_from_slice(&0i32.to_le_bytes());
    o.extend_from_slice(&0u64.to_le_bytes());
    o.extend_from_slice(&0f32.to_le_bytes());
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(&0u64.to_le_bytes());
    o.extend_from_slice(&0u32.to_le_bytes());
    Some(o)
}

fn legacy_chat_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open].replace(']', ")"));
        let after = &rest[open + 1..];
        match after.find(']') {
            Some(close) => {
                let inner = &after[..close];
                match inner.split_once(' ') {
                    Some((url, label)) if url.contains("://") && !label.trim().is_empty() => {
                        out.push_str(&format!("({})[{}]", label.trim(), url));
                    }
                    _ => {
                        out.push('(');
                        out.push_str(inner);
                        out.push(')');
                    }
                }
                rest = &after[close + 1..];
            }
            None => {
                out.push('(');
                rest = after;
            }
        }
    }
    out.push_str(&rest.replace(']', ")"));
    out
}

fn replace_name(text: &str, name: &str) -> String {
    if name.is_empty() {
        return text.to_string();
    }
    let lower_text = text.to_lowercase();
    let lower_name = name.to_lowercase();
    if lower_text.len() != text.len() || lower_name.len() != name.len() {
        return text.replace(name, "you");
    }
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    while let Some(i) = lower_text[pos..].find(&lower_name) {
        out.push_str(&text[pos..pos + i]);
        out.push_str("you");
        pos += i + lower_name.len();
    }
    out.push_str(&text[pos..]);
    out
}

fn message_to_legacy(payload: &[u8], name: &str) -> Vec<Vec<u8>> {
    let mut r = PacketReader::new(payload);
    let (Ok(sender), Ok(content), Ok(target)) = (r.read_string(), r.read_string(), r.read_string()) else {
        return Vec::new();
    };
    if sender == "local" {
        return Vec::new();
    }
    let content = replace_name(&legacy_chat_text(&content), name);
    content
        .split(|c| c == '\n' || c == '\r')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|line| {
            let mut o = Vec::new();
            put_string(&mut o, &sender);
            put_string(&mut o, line);
            put_string(&mut o, &target);
            o
        })
        .collect()
}

fn channel_name(payload: &[u8]) -> Option<String> {
    PacketReader::new(payload).read_string().ok()
}

fn channel_to_legacy(payload: &[u8]) -> Option<Vec<u8>> {
    let mut r = PacketReader::new(payload);
    let name = r.read_string().ok()?;
    let mut o = Vec::new();
    put_string(&mut o, &name);
    Some(o)
}

pub fn to_legacy_out(data: &[u8]) -> Vec<u8> {
    to_legacy_out_for(data, "")
}

pub fn to_legacy_out_for(data: &[u8], name: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut pos = 0;
    while data.len() - pos >= 7 {
        let id = u16::from_le_bytes([data[pos], data[pos + 1]]);
        let len = u32::from_le_bytes([data[pos + 3], data[pos + 4], data[pos + 5], data[pos + 6]]) as usize;
        if data.len() - pos < 7 + len {
            break;
        }
        let payload = &data[pos + 7..pos + 7 + len];
        pos += 7 + len;
        let is_bot = matches!(leading_id(payload), Some(3) | Some(4));
        match id {
            83 | 11 | 12 if is_bot && flag("nobots") => {}
            72 if flag("nofriends") => frame(72, &0u16.to_le_bytes(), &mut out),
            24 if flag("nonotify") => {}
            7 if flag("nomsg") => {}
            64 | 65 if flag("onlyosu") && channel_name(payload).as_deref() != Some("#osu") => {}
            64 => {
                frame(64, payload, &mut out);
                if channel_name(payload).as_deref() == Some("#osu") && !flag("noannounce") && !flag("onlyosu") {
                    let mut o = Vec::new();
                    put_string(&mut o, "#announce");
                    frame(64, &o, &mut out);
                }
            }
            5 | 8 | 19 | 24 | 71 | 72 | 76 | 86 | 89 => frame(id, payload, &mut out),
            75 => frame(75, &LEGACY_PROTOCOL_VERSION.to_le_bytes(), &mut out),
            11 => {
                if let Some(p) = stats_to_legacy(payload) {
                    frame(11, &p, &mut out);
                }
            }
            12 => {
                if let Some(p) = quit_to_legacy(payload) {
                    frame(12, &p, &mut out);
                }
            }
            83 => {
                if let Some(p) = presence_to_legacy(payload) {
                    frame(83, &p, &mut out);
                }
            }
            7 => {
                for p in message_to_legacy(payload, name) {
                    frame(7, &p, &mut out);
                }
            }
            65 => {
                if let Some(p) = channel_to_legacy(payload) {
                    if channel_name(payload).as_deref() == Some("#osu") {
                        frame(if flag("noautojoin") { 65 } else { 67 }, &p, &mut out);
                        if !flag("noannounce") && !flag("onlyosu") {
                            let mut o = Vec::new();
                            put_string(&mut o, "#announce");
                            frame(if flag("noautojoin") { 65 } else { 67 }, &o, &mut out);
                        }
                    } else {
                        frame(65, &p, &mut out);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn status_from_legacy(payload: &[u8]) -> Option<Vec<u8>> {
    let mut r = PacketReader::new(payload);
    let action = r.read_u8().ok()?;
    let text = r.read_string().ok()?;
    let md5 = r.read_string().ok()?;
    let mods = r.read_u16().ok()?;
    let mode = r.read_u8().ok()?;
    let map_id = r.read_i32().ok()?;
    let mut o = Vec::new();
    o.push(action);
    put_string(&mut o, &text);
    put_string(&mut o, &md5);
    o.extend_from_slice(&(mods as u32).to_le_bytes());
    o.push(mode);
    o.extend_from_slice(&map_id.to_le_bytes());
    Some(o)
}

fn message_from_legacy(payload: &[u8]) -> Option<Vec<u8>> {
    let mut r = PacketReader::new(payload);
    let a = r.read_string().ok()?;
    let b = r.read_string().ok()?;
    let c = r.read_string().ok()?;
    let mut o = Vec::new();
    put_string(&mut o, &a);
    put_string(&mut o, &b);
    put_string(&mut o, &c);
    o.extend_from_slice(&0i32.to_le_bytes());
    Some(o)
}

pub fn from_legacy_in(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 16);
    let mut pos = 0;
    while data.len() - pos >= 7 {
        let id = u16::from_le_bytes([data[pos], data[pos + 1]]);
        let len = u32::from_le_bytes([data[pos + 3], data[pos + 4], data[pos + 5], data[pos + 6]]) as usize;
        if data.len() - pos < 7 + len {
            break;
        }
        let payload = &data[pos + 7..pos + 7 + len];
        pos += 7 + len;
        let converted = match id {
            0 => status_from_legacy(payload),
            1 | 25 => message_from_legacy(payload),
            _ => None,
        };
        match converted {
            Some(p) => frame(id, &p, &mut out),
            None => frame(id, payload, &mut out),
        }
    }
    out
}

pub const OLD_PROTOCOL_MAX_BUILD: i64 = 1787;
pub const OLD_PROTOCOL_VERSION: i32 = 5;

struct PresenceInfo {
    name: String,
    tz: u8,
    country: u8,
    privs: u8,
    lon: [u8; 4],
    lat: [u8; 4],
}

pub struct LegacyCtx {
    old5: bool,
    presence: std::collections::HashMap<i32, PresenceInfo>,
    sent: std::collections::HashSet<i32>,
    greeted: bool,
}

struct StatsFields {
    id: i32,
    action: u8,
    text: String,
    md5: String,
    mods: u32,
    mode: u8,
    map_id: i32,
    ranked: Vec<u8>,
    acc: Vec<u8>,
    plays: Vec<u8>,
    total: Vec<u8>,
    rank: Vec<u8>,
}

fn read_stats(payload: &[u8]) -> Option<StatsFields> {
    let mut r = PacketReader::new(payload);
    Some(StatsFields {
        id: r.read_i32().ok()?,
        action: r.read_u8().ok()?,
        text: r.read_string().ok()?,
        md5: r.read_string().ok()?,
        mods: r.read_u32().ok()?,
        mode: r.read_u8().ok()?,
        map_id: r.read_i32().ok()?,
        ranked: r.take(8).ok()?.to_vec(),
        acc: r.take(4).ok()?.to_vec(),
        plays: r.take(4).ok()?.to_vec(),
        total: r.take(8).ok()?.to_vec(),
        rank: r.take(4).ok()?.to_vec(),
    })
}

fn old5_stats_body(f: &StatsFields, completeness: u8, o: &mut Vec<u8>) {
    o.extend_from_slice(&f.id.to_le_bytes());
    o.push(completeness);
    o.push(f.action);
    o.push(1);
    put_string(o, &f.text);
    put_string(o, &f.md5);
    o.extend_from_slice(&(f.mods as u16).to_le_bytes());
    o.push(f.mode);
    o.extend_from_slice(&f.map_id.to_le_bytes());
    o.extend_from_slice(&f.ranked);
    o.extend_from_slice(&f.acc);
    o.extend_from_slice(&f.plays);
    o.extend_from_slice(&f.total);
    o.extend_from_slice(&f.rank);
}

fn country_acronym(byte: u8) -> String {
    match crate::utils::country_byte_to_code(byte) {
        "Unknown" | "--" => "XX".to_string(),
        code => code.to_string(),
    }
}

impl LegacyCtx {
    pub fn new(build: Option<i64>) -> Self {
        Self {
            old5: matches!(build, Some(b) if b > 0 && b <= OLD_PROTOCOL_MAX_BUILD),
            presence: Default::default(),
            sent: Default::default(),
            greeted: false,
        }
    }

    pub fn out(&mut self, data: &[u8], name: &str) -> Vec<u8> {
        if !self.old5 {
            return to_legacy_out_for(data, name);
        }
        let mut out = Vec::with_capacity(data.len());
        let mut pos = 0;
        while data.len() - pos >= 7 {
            let id = u16::from_le_bytes([data[pos], data[pos + 1]]);
            let len = u32::from_le_bytes([data[pos + 3], data[pos + 4], data[pos + 5], data[pos + 6]]) as usize;
            if data.len() - pos < 7 + len {
                break;
            }
            let payload = &data[pos + 7..pos + 7 + len];
            pos += 7 + len;
            match id {
                64 | 65 | 67 if flag("nochan") => {}
                83 | 11 | 12 if flag("nostats") => {}
                24 if flag("nonotify") => {}
                7 if flag("nomsg") => {}
                64 | 65 | 67 if flag("onlyosu") && channel_name(payload).as_deref() != Some("#osu") => {}
                72 if flag("nofriends") => frame(old5_wire(72), &0i32.to_le_bytes(), &mut out),
                83 | 11 | 12 if flag("nobots") && matches!(leading_id(payload), Some(3) | Some(4)) => {}
                5 | 8 | 19 | 24 | 71 | 64 => {
                    frame(old5_wire(id), payload, &mut out);
                    if id == 64 && !self.greeted && channel_name(payload).as_deref() == Some("#osu") {
                        self.greeted = true;
                        let mut p = Vec::new();
                        put_string(&mut p, "BanchoBot");
                        put_string(&mut p, "Welcome to osu-echo!");
                        put_string(&mut p, "#osu");
                        frame(old5_wire(7), &p, &mut out);
                    }
                }
                75 => frame(old5_wire(75), &OLD_PROTOCOL_VERSION.to_le_bytes(), &mut out),
                76 => frame(old5_wire(76), &[], &mut out),
                72 => {
                    let mut r = PacketReader::new(payload);
                    let count = r.read_i16().unwrap_or(0).max(0) as usize;
                    let mut o = Vec::new();
                    o.extend_from_slice(&(count as i32).to_le_bytes());
                    for _ in 0..count {
                        if let Ok(v) = r.read_i32() {
                            o.extend_from_slice(&v.to_le_bytes());
                        }
                    }
                    frame(old5_wire(72), &o, &mut out);
                }
                65 | 67 => {
                    if let Some(p) = channel_to_legacy(payload) {
                        frame(old5_wire(id), &p, &mut out);
                    }
                }
                7 => {
                    for p in message_to_legacy(payload, name) {
                        frame(old5_wire(7), &p, &mut out);
                    }
                }
                83 => {
                    let mut r = PacketReader::new(payload);
                    let parsed = (|| -> Option<(i32, PresenceInfo)> {
                        let id = r.read_i32().ok()?;
                        let name = r.read_string().ok()?;
                        let tz = r.read_u8().ok()?;
                        let country = r.read_u8().ok()?;
                        let privs = r.read_u8().ok()? & 0x1f;
                        let lon: [u8; 4] = r.take(4).ok()?.try_into().ok()?;
                        let lat: [u8; 4] = r.take(4).ok()?.try_into().ok()?;
                        Some((id, PresenceInfo { name, tz, country, privs, lon, lat }))
                    })();
                    if let Some((uid, info)) = parsed {
                        self.sent.remove(&uid);
                        self.presence.insert(uid, info);
                    }
                }
                11 => {
                    if let Some(f) = read_stats(payload) {
                        let mut o = Vec::new();
                        match self.presence.get(&f.id) {
                            Some(info) if !self.sent.contains(&f.id) => {
                                old5_stats_body(&f, 2, &mut o);
                                put_string(&mut o, &info.name);
                                put_string(&mut o, &format!("{}.png", f.id));
                                o.push(info.tz);
                                put_string(&mut o, &country_acronym(info.country));
                                o.push(info.privs);
                                o.extend_from_slice(&info.lon);
                                o.extend_from_slice(&info.lat);
                                self.sent.insert(f.id);
                            }
                            Some(_) => old5_stats_body(&f, 1, &mut o),
                            None => continue,
                        }
                        frame(old5_wire(11), &o, &mut out);
                    }
                }
                12 => {
                    if let Some(uid) = leading_id(payload) {
                        self.sent.remove(&uid);
                        let f = StatsFields {
                            id: uid,
                            action: 0,
                            text: String::new(),
                            md5: String::new(),
                            mods: 0,
                            mode: 0,
                            map_id: 0,
                            ranked: vec![0; 8],
                            acc: vec![0; 4],
                            plays: vec![0; 4],
                            total: vec![0; 8],
                            rank: vec![0; 4],
                        };
                        let mut o = Vec::new();
                        old5_stats_body(&f, 1, &mut o);
                        frame(old5_wire(12), &o, &mut out);
                    }
                }
                _ => {}
            }
        }
        out
    }

    pub fn input(&mut self, data: &[u8]) -> Vec<u8> {
        if !self.old5 {
            return from_legacy_in(data);
        }
        let mut out = Vec::with_capacity(data.len() + 16);
        let mut pos = 0;
        while data.len() - pos >= 7 {
            let id = u16::from_le_bytes([data[pos], data[pos + 1]]);
            let len = u32::from_le_bytes([data[pos + 3], data[pos + 4], data[pos + 5], data[pos + 6]]) as usize;
            if data.len() - pos < 7 + len {
                break;
            }
            let payload = &data[pos + 7..pos + 7 + len];
            pos += 7 + len;
            if id == 11 {
                continue;
            }
            let id = if id > 11 { id - 1 } else { id };
            let converted = match id {
                0 => status_from_old5(payload),
                1 | 25 => message_from_legacy(payload),
                _ => None,
            };
            match converted {
                Some(p) => frame(id, &p, &mut out),
                None => frame(id, payload, &mut out),
            }
        }
        out
    }
}

fn old5_wire(id: u16) -> u16 {
    if id >= 11 { id + 1 } else { id }
}

fn status_from_old5(payload: &[u8]) -> Option<Vec<u8>> {
    let mut r = PacketReader::new(payload);
    let action = r.read_u8().ok()?;
    let update = r.read_u8().ok()? != 0;
    let (text, md5, mods, mode, map_id) = if update {
        (r.read_string().ok()?, r.read_string().ok()?, r.read_u16().ok()? as u32, r.read_u8().ok()?, r.read_i32().ok()?)
    } else {
        (String::new(), String::new(), 0, 0, 0)
    };
    let mut o = Vec::new();
    o.push(action);
    put_string(&mut o, &text);
    put_string(&mut o, &md5);
    o.extend_from_slice(&mods.to_le_bytes());
    o.push(mode);
    o.extend_from_slice(&map_id.to_le_bytes());
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    fn payload(framed: &[u8]) -> Vec<u8> {
        framed[7..].to_vec()
    }

    #[test]
    fn legacy_presence_matches_the_2012_layout() {
        let modern = {
            let mut d = Vec::new();
            d.extend_from_slice(&1000i32.to_le_bytes());
            put_string(&mut d, "Bob");
            d.push(24);
            d.push(1);
            d.push(1);
            d.extend_from_slice(&1.5f32.to_le_bytes());
            d.extend_from_slice(&(-2.5f32).to_le_bytes());
            d.extend_from_slice(&42i32.to_le_bytes());
            d
        };
        let mut framed = Vec::new();
        frame(83, &modern, &mut framed);
        let out = to_legacy_out(&framed);
        assert_eq!(payload(&out), unhex("e80300000b03426f6201180100010000c03f000020c02a000000"));
    }

    #[test]
    fn legacy_stats_match_the_2012_layout() {
        let mut d = Vec::new();
        d.extend_from_slice(&1000i32.to_le_bytes());
        d.push(0);
        put_string(&mut d, "hi");
        put_string(&mut d, "abc");
        d.extend_from_slice(&0u32.to_le_bytes());
        d.push(0);
        d.extend_from_slice(&7i32.to_le_bytes());
        d.extend_from_slice(&123456i64.to_le_bytes());
        d.extend_from_slice(&0.9876f32.to_le_bytes());
        d.extend_from_slice(&5i32.to_le_bytes());
        d.extend_from_slice(&654321i64.to_le_bytes());
        d.extend_from_slice(&42i32.to_le_bytes());
        d.extend_from_slice(&123i16.to_le_bytes());
        let mut framed = Vec::new();
        frame(11, &d, &mut framed);
        let out = to_legacy_out(&framed);
        assert_eq!(
            payload(&out),
            unhex("e8030000000b0268690b036162630000000700000040e20100000000005bd37c3f05000000f1fb0900000000002a000000")
        );
    }

    #[test]
    fn unknown_modern_packets_are_dropped_and_channels_lose_their_extras() {
        let mut data = packets::user_presence_bundle(&[1, 2]);
        data.extend_from_slice(&packets::channel_info("#osu", "desc", 1));
        data.extend_from_slice(&packets::protocol_version(19));
        let out = to_legacy_out(&data);
        let mut expected = Vec::new();
        let mut chan = Vec::new();
        put_string(&mut chan, "#osu");
        frame(67, &chan, &mut expected);
        let mut announce = Vec::new();
        put_string(&mut announce, "#announce");
        frame(67, &announce, &mut expected);
        frame(75, &7i32.to_le_bytes(), &mut expected);
        assert_eq!(out, expected);
    }

    #[test]
    fn legacy_status_and_messages_become_modern_layouts() {
        let mut st = Vec::new();
        st.push(2);
        put_string(&mut st, "x");
        put_string(&mut st, "md5");
        st.extend_from_slice(&72u16.to_le_bytes());
        st.push(0);
        st.extend_from_slice(&9i32.to_le_bytes());
        let mut framed = Vec::new();
        frame(0, &st, &mut framed);
        let mut msg = Vec::new();
        put_string(&mut msg, "me");
        put_string(&mut msg, "hello");
        put_string(&mut msg, "#osu");
        frame(1, &msg, &mut framed);
        let out = from_legacy_in(&framed);
        let split = packets::split_packets(&out);
        let mut r = PacketReader::new(split[0].payload);
        assert_eq!(r.read_u8().unwrap(), 2);
        assert_eq!(r.read_string().unwrap(), "x");
        assert_eq!(r.read_string().unwrap(), "md5");
        assert_eq!(r.read_u32().unwrap(), 72);
        assert_eq!(r.read_u8().unwrap(), 0);
        assert_eq!(r.read_i32().unwrap(), 9);
        let mut r = PacketReader::new(split[1].payload);
        assert_eq!(r.read_string().unwrap(), "me");
        assert_eq!(r.read_string().unwrap(), "hello");
        assert_eq!(r.read_string().unwrap(), "#osu");
        assert_eq!(r.read_i32().unwrap(), 0);
    }

    #[test]
    fn chat_brackets_become_legacy_links_or_parentheses() {
        assert_eq!(legacy_chat_text("hi [https://osu.ppy.sh/b/1 Some Map] there"), "hi (Some Map)[https://osu.ppy.sh/b/1] there");
        assert_eq!(legacy_chat_text("A - B [Insane]\n+HD"), "A - B (Insane)\n+HD");
        assert_eq!(legacy_chat_text("a ] b [ c"), "a ) b ( c");
        assert_eq!(legacy_chat_text("plain"), "plain");
    }

    #[test]
    fn legacy_messages_have_no_newlines_local_sender_or_player_name() {
        let mut framed = Vec::new();
        frame(7, &packets::send_msg("BanchoBot", "Bob unlocked a medal\nline two for BOB", "#osu", -1)[7..], &mut framed);
        let out = to_legacy_out_for(&framed, "Bob");
        let split = packets::split_packets(&out);
        assert_eq!(split.len(), 2);
        let mut r = PacketReader::new(split[0].payload);
        assert_eq!(r.read_string().unwrap(), "BanchoBot");
        assert_eq!(r.read_string().unwrap(), "you unlocked a medal");
        assert_eq!(r.read_string().unwrap(), "#osu");
        let mut r = PacketReader::new(split[1].payload);
        r.read_string().unwrap();
        assert_eq!(r.read_string().unwrap(), "line two for you");
    }

    #[test]
    fn old_2010_clients_get_one_full_packet_per_user_then_stat_updates() {
        let mut pres = Vec::new();
        pres.extend_from_slice(&1000i32.to_le_bytes());
        put_string(&mut pres, "Bob");
        pres.push(24);
        pres.push(0);
        pres.push(1);
        pres.extend_from_slice(&0f32.to_le_bytes());
        pres.extend_from_slice(&0f32.to_le_bytes());
        pres.extend_from_slice(&42i32.to_le_bytes());
        let mut stats = Vec::new();
        stats.extend_from_slice(&1000i32.to_le_bytes());
        stats.push(0);
        put_string(&mut stats, "");
        put_string(&mut stats, "");
        stats.extend_from_slice(&0u32.to_le_bytes());
        stats.push(0);
        stats.extend_from_slice(&0i32.to_le_bytes());
        stats.extend_from_slice(&5i64.to_le_bytes());
        stats.extend_from_slice(&0.5f32.to_le_bytes());
        stats.extend_from_slice(&1i32.to_le_bytes());
        stats.extend_from_slice(&6i64.to_le_bytes());
        stats.extend_from_slice(&42i32.to_le_bytes());
        stats.extend_from_slice(&1i16.to_le_bytes());
        let mut data = Vec::new();
        frame(83, &pres, &mut data);
        frame(11, &stats, &mut data);
        frame(11, &stats, &mut data);
        let mut ctx = LegacyCtx::new(Some(1704));
        let out = ctx.out(&data, "Bob");
        let split = packets::split_packets(&out);
        assert_eq!(split.len(), 2);
        assert_eq!(split[0].payload[4], 2);
        assert_eq!(split[1].payload[4], 1);
        assert!(split[0].payload.len() > split[1].payload.len());
    }

    #[test]
    fn old_2010_wire_ids_are_shifted_by_one_from_eleven_up() {
        let mut data = packets::friends_list(&[3]);
        data.extend_from_slice(&packets::menu_icon("a", "b"));
        let mut ctx = LegacyCtx::new(Some(1704));
        let out = ctx.out(&data, "Bob");
        let ids: Vec<u16> = packets::split_packets(&out).iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![73, 77]);
        let mut inbound = Vec::new();
        frame(11, &[1, 2], &mut inbound);
        frame(26, &[], &mut inbound);
        frame(4, &[], &mut inbound);
        let conv = ctx.input(&inbound);
        let ids: Vec<u16> = packets::split_packets(&conv).iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![25, 4]);
    }

    #[test]
    fn old_2010_friends_list_uses_a_32_bit_count_and_menu_icon_is_empty() {
        let mut data = packets::friends_list(&[3, 4]);
        data.extend_from_slice(&packets::menu_icon("a", "b"));
        let mut ctx = LegacyCtx::new(Some(1704));
        let out = ctx.out(&data, "Bob");
        let split = packets::split_packets(&out);
        assert_eq!(split[0].payload, [2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]);
        assert_eq!(split[1].payload.len(), 0);
    }

    #[test]
    fn only_2012_july_and_older_builds_are_legacy() {
        assert!(is_legacy_build(Some(20120704)));
        assert!(!is_legacy_build(Some(20130303)));
        assert!(!is_legacy_build(Some(20250101)));
        assert!(!is_legacy_build(None));
    }
}
