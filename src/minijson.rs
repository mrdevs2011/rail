//! Tiny JSON reader/writer helpers. Rail has no serde dependency, and the
//! customize files only need objects, arrays, strings and booleans.

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }
    #[allow(dead_code)]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }
    #[allow(dead_code)]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Num(n) => Some(*n as i64),
            _ => None,
        }
    }
    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a.as_slice()),
            _ => None,
        }
    }
    pub fn as_obj(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(items) => Some(items.as_slice()),
            _ => None,
        }
    }
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub fn parse(text: &str) -> Option<Json> {
    let chars: Vec<char> = text.chars().collect();
    let mut p = Parser { c: &chars, i: 0 };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i == p.c.len() { Some(v) } else { None }
}

struct Parser<'a> {
    c: &'a [char],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.c.len() && self.c[self.i].is_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, ch: char) -> bool {
        if self.c.get(self.i) == Some(&ch) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn lit(&mut self, word: &str) -> bool {
        let w: Vec<char> = word.chars().collect();
        if self.c.len() >= self.i + w.len() && self.c[self.i..self.i + w.len()] == w[..] {
            self.i += w.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        if depth > 32 {
            return None;
        }
        self.ws();
        match *self.c.get(self.i)? {
            '{' => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.eat('}') {
                    return Some(Json::Obj(items));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.ws();
                    if !self.eat(':') {
                        return None;
                    }
                    let v = self.value(depth + 1)?;
                    items.push((k, v));
                    self.ws();
                    if self.eat(',') {
                        continue;
                    }
                    if self.eat('}') {
                        return Some(Json::Obj(items));
                    }
                    return None;
                }
            }
            '[' => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.eat(']') {
                    return Some(Json::Arr(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.ws();
                    if self.eat(',') {
                        continue;
                    }
                    if self.eat(']') {
                        return Some(Json::Arr(items));
                    }
                    return None;
                }
            }
            '"' => self.string().map(Json::Str),
            't' if self.lit("true") => Some(Json::Bool(true)),
            'f' if self.lit("false") => Some(Json::Bool(false)),
            'n' if self.lit("null") => Some(Json::Null),
            _ => self.number(),
        }
    }

    fn number(&mut self) -> Option<Json> {
        let start = self.i;
        while self.i < self.c.len()
            && matches!(self.c[self.i], '0'..='9' | '-' | '+' | '.' | 'e' | 'E')
        {
            self.i += 1;
        }
        let s: String = self.c[start..self.i].iter().collect();
        s.parse::<f64>().ok().map(Json::Num)
    }

    fn hex4(&mut self) -> Option<u32> {
        if self.i + 4 > self.c.len() {
            return None;
        }
        let s: String = self.c[self.i..self.i + 4].iter().collect();
        self.i += 4;
        u32::from_str_radix(&s, 16).ok()
    }

    fn string(&mut self) -> Option<String> {
        if !self.eat('"') {
            return None;
        }
        let mut out = String::new();
        loop {
            let ch = *self.c.get(self.i)?;
            self.i += 1;
            match ch {
                '"' => return Some(out),
                '\\' => {
                    let e = *self.c.get(self.i)?;
                    self.i += 1;
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let mut cp = self.hex4()?;
                            if (0xD800..0xDC00).contains(&cp) && self.lit("\\u") {
                                let lo = self.hex4()?;
                                cp = 0x10000
                                    + ((cp - 0xD800) << 10)
                                    + (lo.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                        }
                        _ => return None,
                    }
                }
                c => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_shortcuts_shape() {
        let src = r#"{ "version": 1, "priority": "site",
            "shortcuts": { "a": { "enabled": false, "keys": ["Alt+\\", "F12"] } } }"#;
        let j = parse(src).expect("parse");
        assert_eq!(j.get("priority").and_then(|v| v.as_str()), Some("site"));
        let a = j.get("shortcuts").and_then(|s| s.get("a")).unwrap();
        assert_eq!(a.get("enabled").and_then(|v| v.as_bool()), Some(false));
        let keys = a.get("keys").and_then(|k| k.as_arr()).unwrap();
        assert_eq!(keys[0].as_str(), Some("Alt+\\"));
        assert_eq!(escape("Alt+\\"), "Alt+\\\\");
    }
}
