//! ## Partial Fisher-Yates (hot path)
//!
//! `zero_alloc` appelle `rng.shuffle(&mut base_deck)` qui randomise la totalité
//! du deck résiduel (≈ 40-48 cartes) à chaque itération : O(|deck|) swaps + appels RNG.
//!
//! Ici on ne randomise que les `n_needed` premières positions (≤ 6 cartes dans
//! nos scénarios) : O(n_needed) swaps + appels RNG.
//! `n_needed` est calculé **une seule fois** avant la boucle (cold path).
//!
//! Hypothèse matérielle : réduire de 40 à ≤6 les appels à `xorshift64` et swaps
//! diminue la pression sur le pipeline d'exécution et le bus mémoire.
//!
//! Vérification : `just bench sc1 zero_alloc fisher`
#![allow(dead_code)]

use super::{Condition, Evaluator, SimOdds};
use crate::card::{Card, Rank, Suit};
use std::collections::HashSet;

const MAX_PLAYERS: usize = 9;

const HIGH_CARD: u8   = 0;
const PAIR: u8        = 1;
const TWO_PAIR: u8    = 2;
const THREE_KIND: u8  = 3;
const STRAIGHT: u8    = 4;
const FLUSH: u8       = 5;
const FULL_HOUSE: u8  = 6;
const FOUR_KIND: u8   = 7;
const STR_FLUSH: u8   = 8;
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

/// Identique à zero_alloc::eval5 — sorts conservés (aucune modification).
#[inline(always)]
fn eval5(cards: [Card; 5]) -> u32 {
    let mut v = [0u8; 5];
    for (i, c) in cards.iter().enumerate() {
        v[i] = rank_idx(c.rank);
    }
    v.sort_unstable_by(|a, b| b.cmp(a));

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

    let mut freq = [0u8; 15];
    for &r in &v {
        freq[r as usize] += 1;
    }

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
    if f0 == 4 { return pack(FOUR_KIND, cnt[0].0, cnt[1].0, 0, 0, 0); }
    if f0 == 3 && cnt[1].1 == 2 { return pack(FULL_HOUSE, cnt[0].0, cnt[1].0, 0, 0, 0); }
    if is_flush { return pack(FLUSH, v[0], v[1], v[2], v[3], v[4]); }
    if is_straight { return pack(STRAIGHT, v[0], 0, 0, 0, 0); }
    if is_wheel { return pack(STRAIGHT, 5, 0, 0, 0, 0); }
    if f0 == 3 { return pack(THREE_KIND, cnt[0].0, cnt[1].0, cnt[2].0, 0, 0); }
    if f0 == 2 && cnt[1].1 == 2 { return pack(TWO_PAIR, cnt[0].0, cnt[1].0, cnt[2].0, 0, 0); }
    if f0 == 2 { return pack(PAIR, cnt[0].0, cnt[1].0, cnt[2].0, cnt[3].0, 0); }
    pack(HIGH_CARD, v[0], v[1], v[2], v[3], v[4])
}

/// Identique à zero_alloc::best7 — 21 combos C(7,5), aucune modification.
#[inline(always)]
fn best7(seven: &[Card; 7]) -> u32 {
    let mut best = 0u32;
    for i0 in 0..3usize {
        for i1 in i0 + 1..4 {
            for i2 in i1 + 1..5 {
                for i3 in i2 + 1..6 {
                    for i4 in i3 + 1..7 {
                        let v = eval5([seven[i0], seven[i1], seven[i2], seven[i3], seven[i4]]);
                        if v > best { best = v; }
                    }
                }
            }
        }
    }
    best
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

    /// Partial Fisher-Yates : seules les `k` premières positions sont randomisées.
    #[inline(always)]
    fn partial_shuffle(&mut self, v: &mut [Card], k: usize) {
        let n = v.len();
        for i in 0..k {
            let j = i + (self.next() as usize) % (n - i);
            v.swap(i, j);
        }
    }
}

pub struct FisherEvaluator;

impl Evaluator for FisherEvaluator {
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
    if n == 0 { return vec![]; }
    debug_assert!(n <= MAX_PLAYERS);

    let known: HashSet<Card> = players
        .iter().flat_map(|p| p.iter())
        .chain(board.iter())
        .filter_map(|c| *c)
        .collect();

    let mut base_deck: Vec<Card> = Suit::ALL
        .iter()
        .flat_map(|&s| Rank::ALL.iter().map(move |&r| Card { rank: r, suit: s }))
        .filter(|c| !known.contains(c))
        .collect();

    // Cold path : calculé une seule fois.
    let n_needed: usize = board.iter().filter(|c| c.is_none()).count()
        + players.iter().flat_map(|p| p.iter()).filter(|c| c.is_none()).count();

    let mut win_score  = vec![0.0f32; n];
    let mut cat_counts = [[0u32; 10]; MAX_PLAYERS];

    let dummy = Card { rank: Rank::Two, suit: Suit::Spades };
    let mut hole  = [[dummy; 2]; MAX_PLAYERS];
    let mut seven = [dummy; 7];
    let mut ranks = [0u32; MAX_PLAYERS];
    let mut rng   = Rng::new();

    for _ in 0..iterations {
        rng.partial_shuffle(&mut base_deck, n_needed); // ← seule différence vs zero_alloc
        let mut cur = 0usize;

        for i in 0..n {
            hole[i][0] = players[i][0].unwrap_or_else(|| { let c = base_deck[cur]; cur += 1; c });
            hole[i][1] = players[i][1].unwrap_or_else(|| { let c = base_deck[cur]; cur += 1; c });
        }
        for k in 0..5 {
            seven[k + 2] = board[k].unwrap_or_else(|| { let c = base_deck[cur]; cur += 1; c });
        }

        for i in 0..n {
            seven[0] = hole[i][0];
            seven[1] = hole[i][1];
            ranks[i] = best7(&seven);
        }

        let best = ranks[..n].iter().copied().max().unwrap_or(0);
        let nw   = ranks[..n].iter().filter(|&&r| r == best).count() as u32;
        let share = 1.0 / nw as f32;
        for i in 0..n {
            if ranks[i] == best { win_score[i] += share; }
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
                win:        win_score[i] / total,
                high_card:  cc[HIGH_CARD   as usize] as f32 / total,
                pair:       cc[PAIR        as usize] as f32 / total,
                two_pair:   cc[TWO_PAIR    as usize] as f32 / total,
                three_kind: cc[THREE_KIND  as usize] as f32 / total,
                straight:   cc[STRAIGHT    as usize] as f32 / total,
                flush:      cc[FLUSH       as usize] as f32 / total,
                full_house: cc[FULL_HOUSE  as usize] as f32 / total,
                four_kind:  cc[FOUR_KIND   as usize] as f32 / total,
                str_flush:  cc[STR_FLUSH   as usize] as f32 / total,
                roy_flush:  cc[ROYAL_FLUSH as usize] as f32 / total,
            }
        })
        .collect()
}
