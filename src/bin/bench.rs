use poker::card::{Card, Rank, Suit};
use poker::eval::{Condition, Evaluator, naive::NaiveEvaluator};
use std::time::Instant;

fn main() {
    let ak_spades = [
        Some(Card {
            rank: Rank::Ace,
            suit: Suit::Spades,
        }),
        Some(Card {
            rank: Rank::King,
            suit: Suit::Spades,
        }),
    ];
    let sevtwo = [
        Some(Card {
            rank: Rank::Seven,
            suit: Suit::Hearts,
        }),
        Some(Card {
            rank: Rank::Two,
            suit: Suit::Diamonds,
        }),
    ];
    let flop = [
        Some(Card {
            rank: Rank::Ace,
            suit: Suit::Hearts,
        }),
        Some(Card {
            rank: Rank::King,
            suit: Suit::Diamonds,
        }),
        Some(Card {
            rank: Rank::Two,
            suit: Suit::Clubs,
        }),
        None,
        None,
    ];

    let scenarios: &[(&str, Condition)] = &[
        (
            "2 joueurs  | board vide | 100k iters",
            Condition {
                players: vec![[None; 2]; 2],
                board: [None; 5],
                iterations: 100_000,
            },
        ),
        (
            "6 joueurs  | board vide | 100k iters",
            Condition {
                players: vec![[None; 2]; 6],
                board: [None; 5],
                iterations: 100_000,
            },
        ),
        (
            "As-K♠ vs 7-2  | flop AK2 | 100k iters",
            Condition {
                players: vec![ak_spades, sevtwo],
                board: flop,
                iterations: 100_000,
            },
        ),
        (
            "2 joueurs  | board vide | 1M iters",
            Condition {
                players: vec![[None; 2]; 2],
                board: [None; 5],
                iterations: 1_000_000,
            },
        ),
    ];

    let evaluator = NaiveEvaluator;

    for (name, cond) in scenarios {
        let t = Instant::now();
        let results = evaluator.run(cond);
        let elapsed = t.elapsed();
        let iters_per_sec = cond.iterations as f64 / elapsed.as_secs_f64();

        println!("=== {} ===", name);
        println!("  durée    : {:>8.2?}", elapsed);
        println!("  iters/s  : {:>10.0}", iters_per_sec);
        for (i, o) in results.iter().enumerate() {
            println!(
                "  J{}  win={:5.1}%  pair={:5.1}%  deux-paires={:5.1}%  brelan={:5.1}%  flush={:5.1}%  sf={:5.1}%",
                i + 1,
                o.win * 100.0,
                o.pair * 100.0,
                o.two_pair * 100.0,
                o.three_kind * 100.0,
                o.flush * 100.0,
                o.str_flush * 100.0,
            );
        }
        println!();
    }
}
