//! Les items de ce module sont utilisés depuis `bin/bench.rs`, pas depuis
//! `bin/poker`. Sans cet attribut, le lint dead_code du binaire principal
//! les signale comme morts même si la lib les exporte correctement.
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

/// Évalue exactement 5 cartes — zéro sort, zéro allocation heap.
/// Optimisation 2 par rapport à zero_alloc :
///   - sort_unstable_by([u8;5]) supprimé : quinte détectée par scan direct de
///     la freq table (O(9) itérations, aucun closure/comparateur générique).
///   - sort_unstable_by([(u8,u8)]) supprimé : buckets quad/trip/pairs/singles
///     remplis de rang 14 vers 2, donc déjà en ordre décroissant.
#[inline(always)]
fn eval5(cards: [Card; 5]) -> u32 {
    // Table de fréquences indexée par rang (2..=14)
    let mut freq = [0u8; 15];
    for c in &cards {
        freq[rank_idx(c.rank) as usize] += 1;
    }

    let is_flush = cards[0].suit == cards[1].suit
        && cards[1].suit == cards[2].suit
        && cards[2].suit == cards[3].suit
        && cards[3].suit == cards[4].suit;

    // Détection de quinte : scan descendant, O(9) max, pas de closure
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
    let is_straight = straight_top > 0;
    // Wheel A-2-3-4-5 (As compte comme 1)
    let is_wheel =
        !is_straight && freq[14] > 0 && freq[5] > 0 && freq[4] > 0 && freq[3] > 0 && freq[2] > 0;

    if is_flush && (is_straight || is_wheel) {
        return if is_straight && straight_top == 14 {
            pack(ROYAL_FLUSH, 0, 0, 0, 0, 0)
        } else {
            let top = if is_wheel { 5 } else { straight_top };
            pack(STR_FLUSH, top, 0, 0, 0, 0)
        };
    }

    // Buckets remplis du rang le plus haut vers le plus bas — aucun tri
    // Invariant : pairs[0] >= pairs[1], singles[0] >= singles[1] >= ...
    let mut quad = 0u8;
    let mut trip = 0u8;
    let mut pairs = [0u8; 2];
    let mut np = 0usize;
    let mut singles = [0u8; 5];
    let mut ns = 0usize;
    for r in (2u8..=14).rev() {
        match freq[r as usize] {
            4 => quad = r,
            3 => trip = r,
            2 => {
                pairs[np] = r;
                np += 1;
            }
            1 => {
                singles[ns] = r;
                ns += 1;
            }
            _ => {}
        }
    }

    if quad > 0 {
        return pack(FOUR_KIND, quad, singles[0], 0, 0, 0);
    }
    if trip > 0 && np > 0 {
        return pack(FULL_HOUSE, trip, pairs[0], 0, 0, 0);
    }
    if is_flush {
        // Flush : tous les rangs distincts (garantie du jeu), freq[r] ∈ {0,1}
        let mut fv = [0u8; 5];
        let mut fi = 0usize;
        for r in (2u8..=14).rev() {
            if freq[r as usize] > 0 {
                fv[fi] = r;
                fi += 1;
            }
        }
        return pack(FLUSH, fv[0], fv[1], fv[2], fv[3], fv[4]);
    }
    if is_straight {
        return pack(STRAIGHT, straight_top, 0, 0, 0, 0);
    }
    if is_wheel {
        return pack(STRAIGHT, 5, 0, 0, 0, 0);
    }
    if trip > 0 {
        return pack(THREE_KIND, trip, singles[0], singles[1], 0, 0);
    }
    if np == 2 {
        return pack(TWO_PAIR, pairs[0], pairs[1], singles[0], 0, 0);
    }
    if np == 1 {
        return pack(PAIR, pairs[0], singles[0], singles[1], singles[2], 0);
    }
    pack(
        HIGH_CARD, singles[0], singles[1], singles[2], singles[3], singles[4],
    )
}

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

pub struct SortFreeEvaluator;

impl Evaluator for SortFreeEvaluator {
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
        rng.shuffle(&mut base_deck);
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
            ranks[i] = best7(&seven);
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
