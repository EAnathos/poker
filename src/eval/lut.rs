//! LUT (Lookup Table) evaluator — tables pré-calculées au premier appel.
//!
//! `eval7` construit une freq-table puis exécute ~80 instructions de tri +
//! branches par main. Ici deux tables (cold path `OnceLock`) :
//!   - `flush_table[mask_13bits]`  → u32  (8 192 entrées, 32 Ko)
//!   - `non_flush` open-addressing → u32  (131 072 slots, ~1.5 Mo)
//!
//! Le hot path se réduit à :
//!   1. Boucle 7 cartes → freq + suit_cnt : ~15 instr.
//!   2. Clé (base-5 ou bitmask) : ~10 instr.
//!   3. 1–2 accès mémoire (L3 chaud après warmup).
//!
//! Hypothèse : 1 accès L3 (~40 cycles) + ~25 instr. < ~80 instr. d'eval7
//! une fois la table chaude (~milliers d'itérations).
//!
//! Vérification : `just bench sc6 eval7 lut`
#![allow(dead_code)]

use std::sync::OnceLock;

use super::{Condition, Evaluator, SimOdds};
use crate::card::{Card, Rank, Suit};

const MAX_PLAYERS: usize = 9;

const HIGH_CARD: u8 = 0;
const PAIR: u8 = 1;
const TWO_PAIR: u8 = 2;
const THREE_KIND: u8 = 3;
const STRAIGHT: u8 = 4;
const FLUSH: u8 = 5;
const FULL_HOUSE: u8 = 6;
const FOUR_KIND: u8 = 7;
const STR_FLUSH: u8 = 8;
const ROYAL_FLUSH: u8 = 9;

#[inline(always)]
fn rank_idx(r: Rank) -> u8 {
    r as u8 + 2
}

#[inline(always)]
fn pack(cat: u8, t0: u8, t1: u8, t2: u8, t3: u8, t4: u8) -> u32 {
    ((cat as u32) << 20)
        | ((t0 as u32) << 16)
        | ((t1 as u32) << 12)
        | ((t2 as u32) << 8)
        | ((t3 as u32) << 4)
        | (t4 as u32)
}

// ── Table non-flush : open addressing (2^17 slots) ───────────────────────────

const TBL_BITS: usize = 17; // 131 072 slots ≥ 2× les ~50 K patterns valides
const TBL_SIZE: usize = 1 << TBL_BITS;
const TBL_MASK: usize = TBL_SIZE - 1;

#[inline(always)]
fn ht_slot(key: u64) -> usize {
    let h = key.wrapping_mul(0x9e3779b97f4a7c15);
    ((h ^ (h >> 32)) as usize) & TBL_MASK
}

struct NonFlushTable {
    keys: Vec<u64>,
    vals: Vec<u32>,
}

impl NonFlushTable {
    fn new() -> Self {
        NonFlushTable {
            keys: vec![u64::MAX; TBL_SIZE],
            vals: vec![0u32; TBL_SIZE],
        }
    }

    fn insert(&mut self, key: u64, val: u32) {
        let mut idx = ht_slot(key);
        while self.keys[idx] != u64::MAX {
            idx = (idx + 1) & TBL_MASK;
        }
        self.keys[idx] = key;
        self.vals[idx] = val;
    }

    #[inline(always)]
    fn get(&self, key: u64) -> u32 {
        let mut idx = ht_slot(key);
        loop {
            if self.keys[idx] == key {
                return self.vals[idx];
            }
            idx = (idx + 1) & TBL_MASK;
        }
    }
}

// ── Clé base-5 : bijection freq[2..=14] → u64 ───────────────────────────────

#[inline(always)]
fn freq_key(freq: &[u8; 15]) -> u64 {
    let mut key = 0u64;
    for r in (2usize..=14).rev() {
        key = key * 5 + freq[r] as u64;
    }
    key
}

// ── Évaluation non-flush (cold path, build uniquement) ───────────────────────

fn eval_non_flush(freq: &[u8; 15]) -> u32 {
    let mut straight_top = 0u8;
    for h in (6u8..=14).rev() {
        if freq[h as usize] > 0
            && freq[(h - 1) as usize] > 0
            && freq[(h - 2) as usize] > 0
            && freq[(h - 3) as usize] > 0
            && freq[(h - 4) as usize] > 0
        {
            straight_top = h;
            break;
        }
    }
    let is_wheel = straight_top == 0
        && freq[14] > 0
        && freq[5] > 0
        && freq[4] > 0
        && freq[3] > 0
        && freq[2] > 0;

    let mut cnt = [(0u8, 0u8); 7];
    let mut nc = 0usize;
    for r in (2u8..=14).rev() {
        if freq[r as usize] > 0 {
            cnt[nc] = (r, freq[r as usize]);
            nc += 1;
        }
    }
    cnt[..nc].sort_unstable_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));

    let f0 = cnt[0].1;
    if f0 == 4 {
        return pack(FOUR_KIND, cnt[0].0, cnt[1].0, 0, 0, 0);
    }
    if f0 == 3 && cnt[1].1 >= 2 {
        return pack(FULL_HOUSE, cnt[0].0, cnt[1].0, 0, 0, 0);
    }
    if straight_top > 0 {
        return pack(STRAIGHT, straight_top, 0, 0, 0, 0);
    }
    if is_wheel {
        return pack(STRAIGHT, 5, 0, 0, 0, 0);
    }
    if f0 == 3 {
        return pack(THREE_KIND, cnt[0].0, cnt[1].0, cnt[2].0, 0, 0);
    }
    if f0 == 2 && cnt[1].1 == 2 {
        return pack(TWO_PAIR, cnt[0].0, cnt[1].0, cnt[2].0, 0, 0);
    }
    if f0 == 2 {
        return pack(PAIR, cnt[0].0, cnt[1].0, cnt[2].0, cnt[3].0, 0);
    }
    pack(HIGH_CARD, cnt[0].0, cnt[1].0, cnt[2].0, cnt[3].0, cnt[4].0)
}

// ── Évaluation flush (cold path, build uniquement) ───────────────────────────

fn eval_flush_ranks(ranks: &[u8]) -> u32 {
    let nf = ranks.len(); // 5..=7, trié décroissant
    let mut sf_top = 0u8;
    for i in 0..=(nf - 5) {
        if ranks[i] >= 5
            && ranks[i] == ranks[i + 1] + 1
            && ranks[i + 1] == ranks[i + 2] + 1
            && ranks[i + 2] == ranks[i + 3] + 1
            && ranks[i + 3] == ranks[i + 4] + 1
        {
            sf_top = ranks[i];
            break;
        }
    }
    let is_wheel = sf_top == 0
        && ranks.contains(&14)
        && ranks.contains(&5)
        && ranks.contains(&4)
        && ranks.contains(&3)
        && ranks.contains(&2);

    if sf_top == 14 {
        return pack(ROYAL_FLUSH, 0, 0, 0, 0, 0);
    }
    if sf_top > 0 {
        return pack(STR_FLUSH, sf_top, 0, 0, 0, 0);
    }
    if is_wheel {
        return pack(STR_FLUSH, 5, 0, 0, 0, 0);
    }
    pack(FLUSH, ranks[0], ranks[1], ranks[2], ranks[3], ranks[4])
}

// ── Construction des tables (cold path) ──────────────────────────────────────

fn build_flush_table() -> Vec<u32> {
    let mut table = vec![0u32; 8192];
    for mask in 0u16..8192 {
        let count = mask.count_ones();
        if !(5..=7).contains(&count) {
            continue;
        }
        let mut ranks = [0u8; 7];
        let mut nr = 0usize;
        for bit in (0..13usize).rev() {
            if mask & (1 << bit) != 0 {
                ranks[nr] = bit as u8 + 2;
                nr += 1;
            }
        }
        table[mask as usize] = eval_flush_ranks(&ranks[..nr]);
    }
    table
}

fn build_non_flush_table() -> NonFlushTable {
    let mut table = NonFlushTable::new();
    let mut freq = [0u8; 15];
    enumerate_freq(14, 7, &mut freq, &mut table);
    table
}

// Enumère récursivement tous les vecteurs freq valides (Σ=7, chaque ≤ 4).
fn enumerate_freq(rank: usize, remaining: u8, freq: &mut [u8; 15], table: &mut NonFlushTable) {
    if remaining == 0 {
        let key = freq_key(freq);
        let val = eval_non_flush(freq);
        table.insert(key, val);
        return;
    }
    if rank < 2 {
        return;
    }
    let max_f = remaining.min(4);
    for f in 0..=max_f {
        freq[rank] = f;
        enumerate_freq(rank - 1, remaining - f, freq, table);
    }
    freq[rank] = 0;
}

// ── Données LUT globales ──────────────────────────────────────────────────────

pub(crate) struct LutData {
    flush: Vec<u32>,          // 8 192 entrées, 32 Ko
    non_flush: NonFlushTable, // 131 072 slots, ~1.5 Mo
}

static LUT: OnceLock<LutData> = OnceLock::new();

pub(crate) fn get_lut() -> &'static LutData {
    LUT.get_or_init(|| LutData {
        flush: build_flush_table(),
        non_flush: build_non_flush_table(),
    })
}

// ── Seuil de rentabilité ─────────────────────────────────────────────────────
//
// En dessous de LUT_THRESHOLD itérations, la table non-flush (~1.5 Mo) n'est
// pas encore chaude en L3 : les accès mémoire coûtent plus que les ~80 instr.
// d'eval7. Au-delà, la table est stable en L3 et le gain se matérialise.
pub(crate) const LUT_THRESHOLD: u32 = 200_000;

// ── Fallback eval7 direct (hot path, table froide) ───────────────────────────

#[inline(always)]
pub(crate) fn eval7_inline(seven: &[Card; 7]) -> u32 {
    let mut freq = [0u8; 15];
    let mut suit_cnt = [0u8; 4];
    for c in seven {
        freq[rank_idx(c.rank) as usize] += 1;
        suit_cnt[c.suit as usize] += 1;
    }

    if let Some(fs_idx) = suit_cnt.iter().position(|&c| c >= 5) {
        let fs = Suit::ALL[fs_idx];
        let mut fv = [0u8; 7];
        let mut nf = 0usize;
        for c in seven {
            if c.suit == fs {
                fv[nf] = rank_idx(c.rank);
                nf += 1;
            }
        }
        fv[..nf].sort_unstable_by(|a, b| b.cmp(a));
        let mut sf_top = 0u8;
        for i in 0..=(nf - 5) {
            if fv[i] >= 5
                && fv[i] == fv[i + 1] + 1
                && fv[i + 1] == fv[i + 2] + 1
                && fv[i + 2] == fv[i + 3] + 1
                && fv[i + 3] == fv[i + 4] + 1
            {
                sf_top = fv[i];
                break;
            }
        }
        let is_wheel_flush = sf_top == 0
            && fv[..nf].contains(&14)
            && fv[..nf].contains(&5)
            && fv[..nf].contains(&4)
            && fv[..nf].contains(&3)
            && fv[..nf].contains(&2);
        if sf_top == 14 {
            return pack(ROYAL_FLUSH, 0, 0, 0, 0, 0);
        }
        if sf_top > 0 {
            return pack(STR_FLUSH, sf_top, 0, 0, 0, 0);
        }
        if is_wheel_flush {
            return pack(STR_FLUSH, 5, 0, 0, 0, 0);
        }
        return pack(FLUSH, fv[0], fv[1], fv[2], fv[3], fv[4]);
    }

    let mut cnt = [(0u8, 0u8); 7];
    let mut nc = 0usize;
    for r in 2u8..=14 {
        if freq[r as usize] > 0 {
            cnt[nc] = (r, freq[r as usize]);
            nc += 1;
        }
    }
    cnt[..nc].sort_unstable_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));

    let mut straight_top = 0u8;
    for h in (6u8..=14).rev() {
        if freq[h as usize] > 0
            && freq[(h - 1) as usize] > 0
            && freq[(h - 2) as usize] > 0
            && freq[(h - 3) as usize] > 0
            && freq[(h - 4) as usize] > 0
        {
            straight_top = h;
            break;
        }
    }
    let is_wheel = straight_top == 0
        && freq[14] > 0
        && freq[5] > 0
        && freq[4] > 0
        && freq[3] > 0
        && freq[2] > 0;

    let f0 = cnt[0].1;
    if f0 == 4 {
        return pack(FOUR_KIND, cnt[0].0, cnt[1].0, 0, 0, 0);
    }
    if f0 == 3 && cnt[1].1 >= 2 {
        return pack(FULL_HOUSE, cnt[0].0, cnt[1].0, 0, 0, 0);
    }
    if straight_top > 0 {
        return pack(STRAIGHT, straight_top, 0, 0, 0, 0);
    }
    if is_wheel {
        return pack(STRAIGHT, 5, 0, 0, 0, 0);
    }
    if f0 == 3 {
        return pack(THREE_KIND, cnt[0].0, cnt[1].0, cnt[2].0, 0, 0);
    }
    if f0 == 2 && cnt[1].1 == 2 {
        return pack(TWO_PAIR, cnt[0].0, cnt[1].0, cnt[2].0, 0, 0);
    }
    if f0 == 2 {
        return pack(PAIR, cnt[0].0, cnt[1].0, cnt[2].0, cnt[3].0, 0);
    }
    pack(HIGH_CARD, cnt[0].0, cnt[1].0, cnt[2].0, cnt[3].0, cnt[4].0)
}

// ── Évaluation hot path (LUT) ────────────────────────────────────────────────

#[inline(always)]
pub(crate) fn eval_lut(seven: &[Card; 7], lut: &LutData) -> u32 {
    let mut freq = [0u8; 15];
    let mut suit_cnt = [0u8; 4];
    for c in seven {
        freq[rank_idx(c.rank) as usize] += 1;
        suit_cnt[c.suit as usize] += 1;
    }

    // Flush ?
    if let Some(fs_idx) = suit_cnt.iter().position(|&c| c >= 5) {
        let fs = Suit::ALL[fs_idx];
        let mut flush_mask = 0u16;
        for c in seven {
            if c.suit == fs {
                // bit k = rang k+2 → bit = rank as u8 = rank_idx - 2
                flush_mask |= 1 << (rank_idx(c.rank) - 2);
            }
        }
        return lut.flush[flush_mask as usize];
    }

    lut.non_flush.get(freq_key(&freq))
}

// ── RNG (identique à eval7 + Lemire) ─────────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xdeadbeef_cafebabe);
        Self(if seed == 0 { 1 } else { seed })
    }

    #[inline(always)]
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    #[inline(always)]
    fn partial_shuffle(&mut self, v: &mut [Card], k: usize) {
        let n = v.len();
        for i in 0..k {
            let range = (n - i) as u64;
            let j = i + ((self.next() as u128 * range as u128) >> 64) as usize;
            v.swap(i, j);
        }
    }
}

// ── Évaluateur ───────────────────────────────────────────────────────────────

pub struct LutEvaluator;

impl Evaluator for LutEvaluator {
    fn run(&self, condition: &Condition) -> Vec<SimOdds> {
        simulate(&condition.players, &condition.board, condition.iterations)
    }
}

fn simulate(
    players: &[[Option<Card>; 2]],
    board: &[Option<Card>; 5],
    iterations: u32,
) -> Vec<SimOdds> {
    use std::collections::HashSet;
    let n = players.len();
    if n == 0 {
        return vec![];
    }
    debug_assert!(n <= MAX_PLAYERS);

    // Résolution unique avant la boucle : LLVM hisse la branche hors du hot loop.
    let lut_opt: Option<&'static LutData> = if iterations >= LUT_THRESHOLD {
        Some(get_lut())
    } else {
        None
    };

    let known: HashSet<Card> = players
        .iter()
        .flat_map(|p| p.iter())
        .chain(board.iter())
        .filter_map(|c| *c)
        .collect();

    let mut base_deck: Vec<Card> = Suit::ALL
        .iter()
        .flat_map(|&s| Rank::ALL.iter().map(move |&r| Card { rank: r, suit: s }))
        .filter(|c| !known.contains(c))
        .collect();

    let n_needed: usize = board.iter().filter(|c| c.is_none()).count()
        + players
            .iter()
            .flat_map(|p| p.iter())
            .filter(|c| c.is_none())
            .count();

    let mut win_score = vec![0.0f32; n];
    let mut cat_counts = [[0u32; 10]; MAX_PLAYERS];

    let dummy = Card {
        rank: Rank::Two,
        suit: Suit::Spades,
    };
    let mut hole = [[dummy; 2]; MAX_PLAYERS];
    let mut seven = [dummy; 7];
    let mut ranks = [0u32; MAX_PLAYERS];
    let mut rng = Rng::new();

    for _ in 0..iterations {
        rng.partial_shuffle(&mut base_deck, n_needed);
        let mut cur = 0usize;

        for i in 0..n {
            hole[i][0] = players[i][0].unwrap_or_else(|| {
                let c = base_deck[cur];
                cur += 1;
                c
            });
            hole[i][1] = players[i][1].unwrap_or_else(|| {
                let c = base_deck[cur];
                cur += 1;
                c
            });
        }
        for k in 0..5 {
            seven[k + 2] = board[k].unwrap_or_else(|| {
                let c = base_deck[cur];
                cur += 1;
                c
            });
        }

        for i in 0..n {
            seven[0] = hole[i][0];
            seven[1] = hole[i][1];
            ranks[i] = match lut_opt {
                Some(lut) => eval_lut(&seven, lut),
                None => eval7_inline(&seven),
            };
        }

        let best = ranks[..n].iter().copied().max().unwrap_or(0);
        let nw = ranks[..n].iter().filter(|&&r| r == best).count() as u32;
        let share = 1.0 / nw as f32;
        for i in 0..n {
            if ranks[i] == best {
                win_score[i] += share;
            }
        }
        for i in 0..n {
            cat_counts[i][(ranks[i] >> 20) as usize] += 1;
        }
    }

    let total = iterations as f32;
    (0..n)
        .map(|i| {
            let cc = &cat_counts[i];
            SimOdds {
                win: win_score[i] / total,
                high_card: cc[HIGH_CARD as usize] as f32 / total,
                pair: cc[PAIR as usize] as f32 / total,
                two_pair: cc[TWO_PAIR as usize] as f32 / total,
                three_kind: cc[THREE_KIND as usize] as f32 / total,
                straight: cc[STRAIGHT as usize] as f32 / total,
                flush: cc[FLUSH as usize] as f32 / total,
                full_house: cc[FULL_HOUSE as usize] as f32 / total,
                four_kind: cc[FOUR_KIND as usize] as f32 / total,
                str_flush: cc[STR_FLUSH as usize] as f32 / total,
                roy_flush: cc[ROYAL_FLUSH as usize] as f32 / total,
            }
        })
        .collect()
}
