//! Minimal QR Code encoder (ISO/IEC 18004): byte mode, error correction level M,
//! versions 1-10 (page addresses up to 213 bytes). No dependencies.

/// (total codewords, blocks, EC codewords per block) for versions 1..=10, level M.
const EC_M: [(usize, usize, usize); 10] = [
    (26, 1, 10),
    (44, 1, 16),
    (70, 1, 26),
    (100, 2, 18),
    (134, 2, 24),
    (172, 4, 16),
    (196, 4, 18),
    (242, 4, 22),
    (292, 5, 22),
    (346, 5, 26),
];

/// Alignment pattern center coordinates for versions 1..=10.
const ALIGN: [&[usize]; 10] = [
    &[],
    &[6, 18],
    &[6, 22],
    &[6, 26],
    &[6, 30],
    &[6, 34],
    &[6, 22, 38],
    &[6, 24, 42],
    &[6, 26, 46],
    &[6, 28, 50],
];

pub(crate) struct Qr {
    pub(crate) size: usize,
    modules: Vec<bool>,
}

impl Qr {
    pub(crate) fn get(&self, x: usize, y: usize) -> bool {
        self.modules[y * self.size + x]
    }
}

fn gf_mul(x: u8, y: u8) -> u8 {
    // GF(256), reduction polynomial 0x11D (the x^8 term is dropped by the u8 shift).
    let mut z: u8 = 0;
    for i in (0..8).rev() {
        z = (z << 1) ^ ((z >> 7) * 0x1D);
        z ^= ((y >> i) & 1) * x;
    }
    z
}

fn rs_divisor(degree: usize) -> Vec<u8> {
    let mut result = vec![0u8; degree];
    result[degree - 1] = 1;
    let mut root = 1u8;
    for _ in 0..degree {
        for j in 0..degree {
            result[j] = gf_mul(result[j], root);
            if j + 1 < degree {
                result[j] ^= result[j + 1];
            }
        }
        root = gf_mul(root, 2);
    }
    result
}

fn rs_remainder(data: &[u8], divisor: &[u8]) -> Vec<u8> {
    let mut result = vec![0u8; divisor.len()];
    for &b in data {
        let factor = b ^ result.remove(0);
        result.push(0);
        for (r, &d) in result.iter_mut().zip(divisor) {
            *r ^= gf_mul(d, factor);
        }
    }
    result
}

fn push_bits(bits: &mut Vec<bool>, value: u32, count: usize) {
    for i in (0..count).rev() {
        bits.push((value >> i) & 1 == 1);
    }
}

struct Grid {
    size: usize,
    dark: Vec<bool>,
    func: Vec<bool>,
}

impl Grid {
    fn set_func(&mut self, x: i32, y: i32, d: bool) {
        if x < 0 || y < 0 || x as usize >= self.size || y as usize >= self.size {
            return;
        }
        let i = y as usize * self.size + x as usize;
        self.dark[i] = d;
        self.func[i] = true;
    }

    fn finder(&mut self, cx: i32, cy: i32) {
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let dist = dx.abs().max(dy.abs());
                self.set_func(cx + dx, cy + dy, dist != 2 && dist != 4);
            }
        }
    }

    fn alignment(&mut self, cx: i32, cy: i32) {
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                self.set_func(cx + dx, cy + dy, dx.abs().max(dy.abs()) != 1);
            }
        }
    }

    fn draw_format(&mut self, mask: u32) {
        // Level M has format bits 00.
        let data = mask;
        let mut rem = data;
        for _ in 0..10 {
            rem = (rem << 1) ^ ((rem >> 9) * 0x537);
        }
        let bits = ((data << 10) | rem) ^ 0x5412;
        let bit = |i: u32| (bits >> i) & 1 == 1;
        let n = self.size as i32;
        for i in 0..=5 {
            self.set_func(8, i, bit(i as u32));
        }
        self.set_func(8, 7, bit(6));
        self.set_func(8, 8, bit(7));
        self.set_func(7, 8, bit(8));
        for i in 9..15 {
            self.set_func(14 - i, 8, bit(i as u32));
        }
        for i in 0..8 {
            self.set_func(n - 1 - i, 8, bit(i as u32));
        }
        for i in 8..15 {
            self.set_func(8, n - 15 + i, bit(i as u32));
        }
        self.set_func(8, n - 8, true);
    }

    fn draw_version(&mut self, version: usize) {
        if version < 7 {
            return;
        }
        let mut rem = version as u32;
        for _ in 0..12 {
            rem = (rem << 1) ^ ((rem >> 11) * 0x1F25);
        }
        let bits = ((version as u32) << 12) | rem;
        let n = self.size as i32;
        for i in 0..18 {
            let color = (bits >> i) & 1 == 1;
            let a = n - 11 + (i % 3);
            let b = i / 3;
            self.set_func(a, b, color);
            self.set_func(b, a, color);
        }
    }

    fn draw_function_patterns(&mut self, version: usize) {
        let n = self.size as i32;
        for i in 0..n {
            self.set_func(6, i, i % 2 == 0);
            self.set_func(i, 6, i % 2 == 0);
        }
        self.finder(3, 3);
        self.finder(n - 4, 3);
        self.finder(3, n - 4);
        let al = ALIGN[version - 1];
        let cnt = al.len();
        for i in 0..cnt {
            for j in 0..cnt {
                let skip =
                    (i == 0 && j == 0) || (i == 0 && j == cnt - 1) || (i == cnt - 1 && j == 0);
                if !skip {
                    self.alignment(al[i] as i32, al[j] as i32);
                }
            }
        }
        self.draw_format(0);
        self.draw_version(version);
    }

    fn place_codewords(&mut self, data: &[u8]) {
        let n = self.size;
        let mut i = 0usize;
        let mut right = n as i32 - 1;
        while right >= 1 {
            if right == 6 {
                right = 5;
            }
            for vert in 0..n {
                for j in 0..2 {
                    let x = (right - j) as usize;
                    let upward = ((right + 1) & 2) == 0;
                    let y = if upward { n - 1 - vert } else { vert };
                    let idx = y * n + x;
                    if !self.func[idx] && i < data.len() * 8 {
                        self.dark[idx] = (data[i >> 3] >> (7 - (i & 7))) & 1 == 1;
                        i += 1;
                    }
                }
            }
            right -= 2;
        }
    }

    fn apply_mask(&mut self, mask: u32) {
        let n = self.size;
        for y in 0..n {
            for x in 0..n {
                let (xu, yu) = (x as u32, y as u32);
                let invert = match mask {
                    0 => (xu + yu) % 2 == 0,
                    1 => yu % 2 == 0,
                    2 => xu % 3 == 0,
                    3 => (xu + yu) % 3 == 0,
                    4 => (xu / 3 + yu / 2) % 2 == 0,
                    5 => xu * yu % 2 + xu * yu % 3 == 0,
                    6 => (xu * yu % 2 + xu * yu % 3) % 2 == 0,
                    _ => ((xu + yu) % 2 + xu * yu % 3) % 2 == 0,
                };
                let idx = y * n + x;
                if !self.func[idx] && invert {
                    self.dark[idx] = !self.dark[idx];
                }
            }
        }
    }

    fn penalty(&self) -> u32 {
        const A: [bool; 11] = [
            true, false, true, true, true, false, true, false, false, false, false,
        ];
        const B: [bool; 11] = [
            false, false, false, false, true, false, true, true, true, false, true,
        ];
        let n = self.size;
        let g = |x: usize, y: usize| self.dark[y * n + x];
        let mut p = 0u32;
        for dir in 0..2 {
            for a in 0..n {
                let line: Vec<bool> = (0..n)
                    .map(|b| if dir == 0 { g(b, a) } else { g(a, b) })
                    .collect();
                let mut run = 1u32;
                for i in 1..n {
                    if line[i] == line[i - 1] {
                        run += 1;
                    } else {
                        if run >= 5 {
                            p += 3 + (run - 5);
                        }
                        run = 1;
                    }
                }
                if run >= 5 {
                    p += 3 + (run - 5);
                }
                for i in 0..n.saturating_sub(10) {
                    let w = &line[i..i + 11];
                    if w == A.as_slice() {
                        p += 40;
                    }
                    if w == B.as_slice() {
                        p += 40;
                    }
                }
            }
        }
        for y in 0..n - 1 {
            for x in 0..n - 1 {
                let c = g(x, y);
                if c == g(x + 1, y) && c == g(x, y + 1) && c == g(x + 1, y + 1) {
                    p += 3;
                }
            }
        }
        let dark = self.dark.iter().filter(|&&d| d).count();
        let pct = (dark * 100 / (n * n)) as i32;
        let prev = pct / 5 * 5;
        let k = (prev - 50).abs().min((prev + 5 - 50).abs()) / 5;
        p + (k * 10) as u32
    }
}

/// Encodes `data` as a QR Code (byte mode, level M). `None` when it does not fit version 10.
pub(crate) fn encode(data: &[u8]) -> Option<Qr> {
    let mut version = 0usize;
    for v in 1..=10usize {
        let (raw, blocks, ec) = EC_M[v - 1];
        let cap_bits = (raw - blocks * ec) * 8;
        let cc_bits = if v < 10 { 8 } else { 16 };
        if 4 + cc_bits + data.len() * 8 <= cap_bits {
            version = v;
            break;
        }
    }
    if version == 0 {
        return None;
    }
    let (raw, nblocks, ecc_len) = EC_M[version - 1];
    let cap = (raw - nblocks * ecc_len) * 8;

    let mut bits: Vec<bool> = Vec::new();
    push_bits(&mut bits, 0b0100, 4);
    push_bits(
        &mut bits,
        data.len() as u32,
        if version < 10 { 8 } else { 16 },
    );
    for &b in data {
        push_bits(&mut bits, b as u32, 8);
    }
    let term = (cap - bits.len()).min(4);
    push_bits(&mut bits, 0, term);
    while !bits.len().is_multiple_of(8) {
        bits.push(false);
    }
    let mut pad = 0xECu32;
    while bits.len() < cap {
        push_bits(&mut bits, pad, 8);
        pad ^= 0xEC ^ 0x11;
    }
    let cw: Vec<u8> = bits
        .chunks(8)
        .map(|c| c.iter().fold(0u8, |a, &b| (a << 1) | b as u8))
        .collect();

    let short_len = raw / nblocks;
    let num_short = nblocks - raw % nblocks;
    let divisor = rs_divisor(ecc_len);
    let mut blocks: Vec<Vec<u8>> = Vec::new();
    let mut k = 0usize;
    for i in 0..nblocks {
        let take = short_len - ecc_len + if i < num_short { 0 } else { 1 };
        let mut dat = cw[k..k + take].to_vec();
        k += take;
        let ecc = rs_remainder(&dat, &divisor);
        if i < num_short {
            dat.push(0);
        }
        dat.extend(ecc);
        blocks.push(dat);
    }
    let mut out: Vec<u8> = Vec::with_capacity(raw);
    for i in 0..blocks[0].len() {
        for (j, b) in blocks.iter().enumerate() {
            if i != short_len - ecc_len || j >= num_short {
                out.push(b[i]);
            }
        }
    }

    let size = 17 + 4 * version;
    let mut grid = Grid {
        size,
        dark: vec![false; size * size],
        func: vec![false; size * size],
    };
    grid.draw_function_patterns(version);
    grid.place_codewords(&out);
    let mut best = 0u32;
    let mut best_pen = u32::MAX;
    for mask in 0..8u32 {
        grid.apply_mask(mask);
        grid.draw_format(mask);
        let p = grid.penalty();
        if p < best_pen {
            best_pen = p;
            best = mask;
        }
        grid.apply_mask(mask);
    }
    grid.apply_mask(best);
    grid.draw_format(best);
    Some(Qr {
        size,
        modules: grid.dark,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_capacity() {
        assert_eq!(encode(b"A").unwrap().size, 21);
        assert_eq!(encode(b"https://example.com/").unwrap().size, 25);
        assert!(encode(&[b'a'; 213]).is_some());
        assert!(encode(&[b'a'; 214]).is_none());
    }

    #[test]
    fn finder_patterns_present() {
        let q = encode(b"https://rail.example/").unwrap();
        let n = q.size;
        for (ox, oy) in [(0, 0), (n - 7, 0), (0, n - 7)] {
            assert!(q.get(ox, oy) && q.get(ox + 6, oy + 6) && q.get(ox + 3, oy + 3));
            assert!(!q.get(ox + 1, oy + 1) && !q.get(ox + 5, oy + 5));
        }
    }

    /// Writes matrices for an external decoder when QR_DUMP=<dir> is set.
    #[test]
    fn dump_for_external_decoder() {
        let Ok(dir) = std::env::var("QR_DUMP") else {
            return;
        };
        // One sample at the capacity limit of every version 1..=10, plus a shorter one for each.
        const CAPS: [usize; 10] = [14, 26, 42, 62, 84, 106, 122, 152, 180, 213];
        let alphabet = b"abcXYZ019/:?=&-_.~%";
        let mut samples: Vec<Vec<u8>> = Vec::new();
        for &n in CAPS.iter() {
            for len in [n, n - 5] {
                samples.push(
                    (0..len)
                        .map(|i| alphabet[(i * 7 + 3) % alphabet.len()])
                        .collect(),
                );
            }
        }
        for (i, s) in samples.iter().enumerate() {
            let q = encode(s).unwrap();
            let mut txt = String::new();
            for y in 0..q.size {
                for x in 0..q.size {
                    txt.push(if q.get(x, y) { '1' } else { '0' });
                }
                txt.push('\n');
            }
            std::fs::write(format!("{dir}/qr{i}.txt"), txt).unwrap();
            std::fs::write(format!("{dir}/qr{i}.data"), s).unwrap();
        }
    }
}
