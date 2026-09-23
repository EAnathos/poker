//! LutParEvaluator — version parallèle de LutEvaluator via rayon.
//!
//! La simulation Monte Carlo est embarrassingly parallel : chaque itération est
//! indépendante. Les itérations sont distribuées uniformément sur N threads rayon.
//! La LUT est partagée en lecture pure (&'static LutData → pas de coherency traffic).
//! Chaque thread possède son propre Rng (seeds distincts = tid × Fibonacci constant).
//!
//! Hypothèse : scaling quasi-linéaire sur les cœurs physiques disponibles.
//! Vérification : `just bench sc6 lut lut_par`
#![allow(dead_code)]

use rayon::prelude::*;

use super::lut::{LUT_THRESHOLD, LutData, eval_lut, eval7_inline, get_lut};
use super::{Condition, Evaluator, SimOdds};
use crate::card::{Card, Rank, Suit};

const MAX_PLAYERS: usize = 9;

// ── RNG (XorShift64 + Lemire — même implémentation que lut.rs) ───────────────

struct Rng(u64);

impl Rng {
    fn new_seeded(tid: usize) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let base = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xdeadbeef_cafebabe);
        // Fibonacci hashing : garantit des seeds distincts et bien distribués par tid.
        let seed = base ^ (tid as u64).wrapping_mul(0x9e3779b97f4a7c15);
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

pub struct LutParEvaluator;

impl Evaluator for LutParEvaluator {
    fn run(&self, condition: &Condition) -> Vec<SimOdds> {
        simulate_par(&condition.players, &condition.board, condition.iterations)
    }
}

fn simulate_par(
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

    // Résolu une seule fois : tous les threads partagent le même choix LUT/inline.
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

    let base_deck: Vec<Card> = Suit::ALL
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

    let n_threads = rayon::current_num_threads();
    let base_iters = iterations / n_threads as u32;
    let remainder = (iterations % n_threads as u32) as usize;

    // Chaque thread retourne (win_scores, cat_counts) sur ses itérations locales.
    let chunks: Vec<(Vec<f32>, Vec<[u32; 10]>)> = (0..n_threads)
        .into_par_iter()
        .map(|tid| {
            let chunk_iters = base_iters + if tid < remainder { 1 } else { 0 };
            let dummy = Card {
                rank: Rank::Two,
                suit: Suit::Spades,
            };
            let mut deck = base_deck.clone();
            let mut hole = [[dummy; 2]; MAX_PLAYERS];
            let mut seven = [dummy; 7];
            let mut ranks = [0u32; MAX_PLAYERS];
            let mut win_score = vec![0.0f32; n];
            let mut cat_counts = vec![[0u32; 10]; n];
            let mut rng = Rng::new_seeded(tid);

            for _ in 0..chunk_iters {
                rng.partial_shuffle(&mut deck, n_needed);
                let mut cur = 0usize;

                for i in 0..n {
                    hole[i][0] = players[i][0].unwrap_or_else(|| {
                        let c = deck[cur];
                        cur += 1;
                        c
                    });
                    hole[i][1] = players[i][1].unwrap_or_else(|| {
                        let c = deck[cur];
                        cur += 1;
                        c
                    });
                }
                for k in 0..5 {
                    seven[k + 2] = board[k].unwrap_or_else(|| {
                        let c = deck[cur];
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

            (win_score, cat_counts)
        })
        .collect();

    // Merge : somme des résultats partiels de chaque thread.
    let mut win_score = vec![0.0f32; n];
    let mut cat_counts = vec![[0u32; 10]; n];
    for (ws, cc) in chunks {
        for i in 0..n {
            win_score[i] += ws[i];
            for c in 0..10 {
                cat_counts[i][c] += cc[i][c];
            }
        }
    }

    let total = iterations as f32;
    (0..n)
        .map(|i| {
            let cc = &cat_counts[i];
            SimOdds {
                win: win_score[i] / total,
                high_card: cc[0] as f32 / total,
                pair: cc[1] as f32 / total,
                two_pair: cc[2] as f32 / total,
                three_kind: cc[3] as f32 / total,
                straight: cc[4] as f32 / total,
                flush: cc[5] as f32 / total,
                full_house: cc[6] as f32 / total,
                four_kind: cc[7] as f32 / total,
                str_flush: cc[8] as f32 / total,
                roy_flush: cc[9] as f32 / total,
            }
        })
        .collect()
}
