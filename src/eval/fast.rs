//! Les items de ce module sont utilisés depuis `bin/bench.rs`, pas depuis
//! `bin/poker`. Sans cet attribut, le lint dead_code du binaire principal
//! les signale comme morts même si la lib les exporte correctement.
#![allow(dead_code)]

use super::{Condition, Evaluator, SimOdds};
use crate::card::{Card, Rank, Suit};
use std::collections::HashSet;

const MAX_PLAYERS: usize = 9;

// Catégories ordonnées par force (identiques à HandCategory dans naive)
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

// Two=2, Three=3, ..., King=13, Ace=14 (variants 0-indexed, +2 offset)
#[inline(always)]
fn rank_idx(r: Rank) -> u8 {
    r as u8 + 2
}

/// Encode une main en u32 directement comparable (plus grand = meilleur) :
///   bits [23:20] = catégorie (0..9)
///   bits [19:0]  = 5 nibbles de tiebreak, tb[0] = nibble le plus significatif
#[inline(always)]
fn pack(cat: u8, t0: u8, t1: u8, t2: u8, t3: u8, t4: u8) -> u32 {
    ((cat as u32) << 20)
        | ((t0 as u32) << 16)
        | ((t1 as u32) << 12)
        | ((t2 as u32) << 8)
        | ((t3 as u32) << 4)
        | (t4 as u32)
}

/// Évalue exactement 5 cartes — zéro allocation heap.
#[inline(always)]
fn eval5(cards: [Card; 5]) -> u32 {
    let mut v = [0u8; 5];
    for (i, c) in cards.iter().enumerate() {
        v[i] = rank_idx(c.rank);
    }
    v.sort_unstable_by(|a, b| b.cmp(a)); // décroissant

    let is_flush = cards[0].suit == cards[1].suit
        && cards[1].suit == cards[2].suit
        && cards[2].suit == cards[3].suit
        && cards[3].suit == cards[4].suit;

    let is_straight = v[0] == v[1] + 1 && v[1] == v[2] + 1 && v[2] == v[3] + 1 && v[3] == v[4] + 1;
    let is_wheel = v[0] == 14 && v[1] == 5 && v[2] == 4 && v[3] == 3 && v[4] == 2;

    if is_flush && (is_straight || is_wheel) {
        return if v[0] == 14 && !is_wheel {
            pack(ROYAL_FLUSH, 0, 0, 0, 0, 0)
        } else {
            let top = if is_wheel { 5 } else { v[0] };
            pack(STR_FLUSH, top, 0, 0, 0, 0)
        };
    }

    // Table de fréquences (stack, 15 octets)
    let mut freq = [0u8; 15];
    for &r in &v {
        freq[r as usize] += 1;
    }

    // Groupes (rank, freq) triés par (freq desc, rank desc) — 5 entrées max
    let mut cnt = [(0u8, 0u8); 5];
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
    if f0 == 3 && cnt[1].1 == 2 {
        return pack(FULL_HOUSE, cnt[0].0, cnt[1].0, 0, 0, 0);
    }
    if is_flush {
        return pack(FLUSH, v[0], v[1], v[2], v[3], v[4]);
    }
    if is_straight {
        return pack(STRAIGHT, v[0], 0, 0, 0, 0);
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
    pack(HIGH_CARD, v[0], v[1], v[2], v[3], v[4])
}

/// Meilleure main sur exactement 7 cartes (C(7,5)=21 combos) — zéro allocation.
#[inline(always)]
fn best7(seven: &[Card; 7]) -> u32 {
    let mut best = 0u32;
    for i0 in 0..3usize {
        for i1 in i0 + 1..4 {
            for i2 in i1 + 1..5 {
                for i3 in i2 + 1..6 {
                    for i4 in i3 + 1..7 {
                        let v = eval5([seven[i0], seven[i1], seven[i2], seven[i3], seven[i4]]);
                        if v > best {
                            best = v;
                        }
                    }
                }
            }
        }
    }
    best
}

// ── XorShift64 RNG ────────────────────────────────────────────────────────────

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

    fn shuffle(&mut self, v: &mut [Card]) {
        for i in (1..v.len()).rev() {
            let j = (self.next() as usize) % (i + 1);
            v.swap(i, j);
        }
    }
}

// ── FastEvaluator ─────────────────────────────────────────────────────────────

pub struct FastEvaluator;

impl Evaluator for FastEvaluator {
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

    // ── Cold path : initialisation unique ─────────────────────────────────────
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

    // Stats accumulées (allouées une fois)
    let mut win_score = vec![0.0f32; n];
    let mut cat_counts = [[0u32; 10]; MAX_PLAYERS]; // [joueur][catégorie 0..9]

    // ── Hot path : buffers réutilisés sans allocation ─────────────────────────
    let dummy = Card {
        rank: Rank::Two,
        suit: Suit::Spades,
    };
    let mut hole = [[dummy; 2]; MAX_PLAYERS];
    let mut seven = [dummy; 7]; // [hole0, hole1, board0..4]
    let mut ranks = [0u32; MAX_PLAYERS];

    let mut rng = Rng::new();

    for _ in 0..iterations {
        rng.shuffle(&mut base_deck);
        let mut cur = 0usize;

        // Cartes privées
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

        // Board commun (positions 2..7 du buffer seven)
        for k in 0..5 {
            seven[k + 2] = board[k].unwrap_or_else(|| {
                let c = base_deck[cur];
                cur += 1;
                c
            });
        }

        // Évaluation — seules les positions 0-1 changent par joueur
        for i in 0..n {
            seven[0] = hole[i][0];
            seven[1] = hole[i][1];
            ranks[i] = best7(&seven);
        }

        // Gagnant(s) — pas de Vec, deux passes O(n)
        let best = ranks[..n].iter().copied().max().unwrap_or(0);
        let nw = ranks[..n].iter().filter(|&&r| r == best).count() as u32;

        let share = 1.0 / nw as f32;
        for i in 0..n {
            if ranks[i] == best {
                win_score[i] += share;
            }
        }

        // Compteurs de catégorie : extraits des bits [23:20] du rang encodé
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
