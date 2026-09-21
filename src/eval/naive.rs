use super::{Condition, Evaluator, SimOdds};
use crate::card::{Card, Rank, Suit};
use std::collections::HashSet;

// ── Rank value ────────────────────────────────────────────────────────────────

fn rank_val(r: Rank) -> u8 {
    match r {
        Rank::Two => 2,
        Rank::Three => 3,
        Rank::Four => 4,
        Rank::Five => 5,
        Rank::Six => 6,
        Rank::Seven => 7,
        Rank::Eight => 8,
        Rank::Nine => 9,
        Rank::Ten => 10,
        Rank::Jack => 11,
        Rank::Queen => 12,
        Rank::King => 13,
        Rank::Ace => 14,
    }
}

// ── Hand category (ordered by strength) ──────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum HandCategory {
    HighCard,
    Pair,
    TwoPair,
    ThreeKind,
    Straight,
    Flush,
    FullHouse,
    FourKind,
    StraightFlush,
    RoyalFlush,
}

// ── Hand value (fully orderable for comparison) ───────────────────────────────

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct HandValue {
    category: HandCategory,
    // Ranks in comparison priority order (high→low); compared lexicographically.
    tiebreak: Vec<u8>,
}

// ── 5-card evaluator ──────────────────────────────────────────────────────────

fn evaluate_five(cards: [Card; 5]) -> HandValue {
    let mut vals: Vec<u8> = cards.iter().map(|c| rank_val(c.rank)).collect();
    vals.sort_unstable_by(|a, b| b.cmp(a)); // descending

    let is_flush = cards.iter().all(|c| c.suit == cards[0].suit);
    let is_straight = vals.windows(2).all(|w| w[0] == w[1] + 1);
    // A-2-3-4-5 wheel: ace acts as 1
    let is_wheel = vals == [14, 5, 4, 3, 2];

    // Rank frequency table, sorted by (count desc, rank desc)
    let mut freq = [0u8; 15];
    for &v in &vals {
        freq[v as usize] += 1;
    }
    let mut counts: Vec<(u8, u8)> = (2u8..=14)
        .filter(|&r| freq[r as usize] > 0)
        .map(|r| (r, freq[r as usize]))
        .collect();
    counts.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));
    let tb_ranks: Vec<u8> = counts.iter().map(|&(r, _)| r).collect();

    // Straight flush / royal flush
    if is_flush && (is_straight || is_wheel) {
        return if vals[0] == 14 && !is_wheel {
            HandValue {
                category: HandCategory::RoyalFlush,
                tiebreak: vec![],
            }
        } else {
            let top = if is_wheel { 5 } else { vals[0] };
            HandValue {
                category: HandCategory::StraightFlush,
                tiebreak: vec![top],
            }
        };
    }

    if counts[0].1 == 4 {
        return HandValue {
            category: HandCategory::FourKind,
            tiebreak: tb_ranks,
        };
    }
    if counts[0].1 == 3 && counts[1].1 == 2 {
        return HandValue {
            category: HandCategory::FullHouse,
            tiebreak: tb_ranks,
        };
    }
    if is_flush {
        return HandValue {
            category: HandCategory::Flush,
            tiebreak: vals,
        };
    }
    if is_straight {
        return HandValue {
            category: HandCategory::Straight,
            tiebreak: vec![vals[0]],
        };
    }
    if is_wheel {
        return HandValue {
            category: HandCategory::Straight,
            tiebreak: vec![5],
        };
    }
    if counts[0].1 == 3 {
        return HandValue {
            category: HandCategory::ThreeKind,
            tiebreak: tb_ranks,
        };
    }
    if counts[0].1 == 2 && counts[1].1 == 2 {
        return HandValue {
            category: HandCategory::TwoPair,
            tiebreak: tb_ranks,
        };
    }
    if counts[0].1 == 2 {
        return HandValue {
            category: HandCategory::Pair,
            tiebreak: tb_ranks,
        };
    }
    HandValue {
        category: HandCategory::HighCard,
        tiebreak: vals,
    }
}

// ── Best 5-card hand from 5–7 cards (all C(n,5) combos) ──────────────────────

fn best_hand(cards: &[Card]) -> HandValue {
    let n = cards.len();
    debug_assert!(n >= 5 && n <= 7);
    let mut best: Option<HandValue> = None;

    for i0 in 0..n - 4 {
        for i1 in i0 + 1..n - 3 {
            for i2 in i1 + 1..n - 2 {
                for i3 in i2 + 1..n - 1 {
                    for i4 in i3 + 1..n {
                        let val =
                            evaluate_five([cards[i0], cards[i1], cards[i2], cards[i3], cards[i4]]);
                        if best.as_ref().map_or(true, |b| val > *b) {
                            best = Some(val);
                        }
                    }
                }
            }
        }
    }

    best.unwrap()
}

// ── XorShift64 RNG (no external deps) ────────────────────────────────────────

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

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn shuffle(&mut self, v: &mut Vec<Card>) {
        for i in (1..v.len()).rev() {
            let j = (self.next() as usize) % (i + 1);
            v.swap(i, j);
        }
    }
}

// ── NaiveEvaluator ───────────────────────────────────────────────────────────

pub struct NaiveEvaluator;

impl Evaluator for NaiveEvaluator {
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

    let known: HashSet<Card> = players
        .iter()
        .flat_map(|p| p.iter())
        .chain(board.iter())
        .filter_map(|c| *c)
        .collect();

    let mut base_deck: Vec<Card> = Suit::ALL
        .iter()
        .flat_map(|&suit| Rank::ALL.iter().map(move |&rank| Card { rank, suit }))
        .filter(|c| !known.contains(c))
        .collect();

    let mut win_score = vec![0.0f32; n];
    let mut high_card_count = vec![0u32; n];
    let mut pair_count = vec![0u32; n];
    let mut two_pair_count = vec![0u32; n];
    let mut three_kind_count = vec![0u32; n];
    let mut straight_count = vec![0u32; n];
    let mut flush_count = vec![0u32; n];
    let mut full_house_count = vec![0u32; n];
    let mut four_kind_count = vec![0u32; n];
    let mut str_flush_count = vec![0u32; n];
    let mut roy_flush_count = vec![0u32; n];

    let mut rng = Rng::new();

    for _ in 0..iterations {
        rng.shuffle(&mut base_deck);
        let mut cur = 0usize;

        let hole_cards: Vec<[Card; 2]> = players
            .iter()
            .map(|p| {
                let h0 = p[0].unwrap_or_else(|| {
                    let c = base_deck[cur];
                    cur += 1;
                    c
                });
                let h1 = p[1].unwrap_or_else(|| {
                    let c = base_deck[cur];
                    cur += 1;
                    c
                });
                [h0, h1]
            })
            .collect();

        // Board partagé entre tous les joueurs — complété une seule fois
        let mut filled_board = Vec::with_capacity(5);
        for slot in board {
            filled_board.push(slot.unwrap_or_else(|| {
                let c = base_deck[cur];
                cur += 1;
                c
            }));
        }

        let hand_values: Vec<HandValue> = hole_cards
            .iter()
            .map(|h| {
                let mut cards = vec![h[0], h[1]];
                cards.extend_from_slice(&filled_board);
                best_hand(&cards)
            })
            .collect();

        let best = hand_values.iter().max().unwrap();
        let winners: Vec<usize> = hand_values
            .iter()
            .enumerate()
            .filter(|(_, v)| *v == best)
            .map(|(i, _)| i)
            .collect();
        let share = 1.0 / winners.len() as f32;
        for &w in &winners {
            win_score[w] += share;
        }

        for (i, hv) in hand_values.iter().enumerate() {
            match hv.category {
                HandCategory::HighCard => high_card_count[i] += 1,
                HandCategory::Pair => pair_count[i] += 1,
                HandCategory::TwoPair => two_pair_count[i] += 1,
                HandCategory::ThreeKind => three_kind_count[i] += 1,
                HandCategory::Straight => straight_count[i] += 1,
                HandCategory::Flush => flush_count[i] += 1,
                HandCategory::FullHouse => full_house_count[i] += 1,
                HandCategory::FourKind => four_kind_count[i] += 1,
                HandCategory::StraightFlush => str_flush_count[i] += 1,
                HandCategory::RoyalFlush => roy_flush_count[i] += 1,
            }
        }
    }

    let total = iterations as f32;
    (0..n)
        .map(|i| SimOdds {
            win: win_score[i] / total,
            high_card: high_card_count[i] as f32 / total,
            pair: pair_count[i] as f32 / total,
            two_pair: two_pair_count[i] as f32 / total,
            three_kind: three_kind_count[i] as f32 / total,
            straight: straight_count[i] as f32 / total,
            flush: flush_count[i] as f32 / total,
            full_house: full_house_count[i] as f32 / total,
            four_kind: four_kind_count[i] as f32 / total,
            str_flush: str_flush_count[i] as f32 / total,
            roy_flush: roy_flush_count[i] as f32 / total,
        })
        .collect()
}
