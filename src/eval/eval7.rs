//! `zero_alloc` (et `partial`) appellent `best7` qui énumère C(7,5) = 21 combos
//! et appelle `eval5` sur chacun : 21 × (~60 instructions) = ~1260 instr. par joueur.
//!
//! Ici une seule passe sur les 7 cartes construit une freq-table et une
//! suit-table, puis dérive directement la meilleure main : ~80 instructions.
//!
//! Hypothèse matérielle : remplacer 21 appels à `eval5` par 1 appel à `eval7`
//! réduit la pression sur i-cache et branch predictor — l'overhead des 2 sorts
//! sur ≤7 éléments est négligeable (insertion-sort inlined par LLVM).
//!
//! Vérification : `just bench sc1 fisher eval7`
#![allow(dead_code)]

use super::{Condition, Evaluator, SimOdds};
use crate::card::{Card, Rank, Suit};
use std::collections::HashSet;

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

/// Évalue directement 7 cartes — remplace best7 + 21 × eval5.
///
/// Style zero_alloc : sorts `sort_unstable_by` maintenus (pas de bucket scan
/// sort_free). Les deux sorts portent sur ≤7 éléments → inlinés par LLVM comme
/// insertion sort, coût négligeable vs le gain sur la suppression des 21 combos.
#[inline(always)]
fn eval7(seven: &[Card; 7]) -> u32 {
    // ── 1. Tables de fréquences ───────────────────────────────────────────────
    let mut freq = [0u8; 15]; // freq[rank 2..=14]
    let mut suit_cnt = [0u8; 4]; // Spades=0, Hearts=1, Diamonds=2, Clubs=3
    for c in seven {
        freq[rank_idx(c.rank) as usize] += 1;
        suit_cnt[c.suit as usize] += 1;
    }

    // ── 2. Flush ? ────────────────────────────────────────────────────────────
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
        // Style zero_alloc : tri sort_unstable_by sur ≤7 éléments
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

    // ── 3. Cnt array trié par (freq desc, rank desc) — style zero_alloc ──────
    let mut cnt = [(0u8, 0u8); 7]; // max 7 rangs distincts sur 7 cartes
    let mut nc = 0usize;
    for r in 2u8..=14 {
        if freq[r as usize] > 0 {
            cnt[nc] = (r, freq[r as usize]);
            nc += 1;
        }
    }
    cnt[..nc].sort_unstable_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));

    // ── 4. Quinte — scan freq table (obligatoire sur 7 cartes) ───────────────
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

    // ── 5. Meilleure main ─────────────────────────────────────────────────────
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

pub struct Eval7Evaluator;

impl Evaluator for Eval7Evaluator {
    fn run(&self, condition: &Condition) -> Vec<SimOdds> {
        simulate(&condition.players, &condition.board, condition.iterations)
    }
}

fn simulate(
    players: &[[Option<Card>; 2]],
    board: &[Option<Card>; 5],
    iterations: u32,
) -> Vec<SimOdds> {
    let n = players.len();
    if n == 0 {
        return vec![];
    }
    debug_assert!(n <= MAX_PLAYERS);

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
            ranks[i] = eval7(&seven);
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
