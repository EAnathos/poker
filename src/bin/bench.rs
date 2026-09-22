use poker::card::{Card, Rank, Suit};
use poker::eval::{Condition, Evaluator, naive::NaiveEvaluator};
use std::time::Instant;

fn main() {
    let scenarios: &[(&str, Condition)] = &[
        (
            "AA vs KK vs QJs | flop 9s-Ts-2d | 50k iters",
            Condition {
                players: vec![
                    [
                        Some(Card {
                            rank: Rank::Ace,
                            suit: Suit::Spades,
                        }),
                        Some(Card {
                            rank: Rank::Ace,
                            suit: Suit::Clubs,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::King,
                            suit: Suit::Spades,
                        }),
                        Some(Card {
                            rank: Rank::King,
                            suit: Suit::Clubs,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::Queen,
                            suit: Suit::Spades,
                        }),
                        Some(Card {
                            rank: Rank::Jack,
                            suit: Suit::Spades,
                        }),
                    ],
                ],
                board: [
                    Some(Card {
                        rank: Rank::Nine,
                        suit: Suit::Spades,
                    }),
                    Some(Card {
                        rank: Rank::Ten,
                        suit: Suit::Spades,
                    }),
                    Some(Card {
                        rank: Rank::Two,
                        suit: Suit::Diamonds,
                    }),
                    None,
                    None,
                ],
                iterations: 50_000,
            },
        ),
        (
            "AhKh vs 9c9d vs JcTc | turn 9h-8h-2s-3h | 75k iters",
            Condition {
                players: vec![
                    [
                        Some(Card {
                            rank: Rank::Ace,
                            suit: Suit::Hearts,
                        }),
                        Some(Card {
                            rank: Rank::King,
                            suit: Suit::Hearts,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::Nine,
                            suit: Suit::Clubs,
                        }),
                        Some(Card {
                            rank: Rank::Nine,
                            suit: Suit::Diamonds,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::Jack,
                            suit: Suit::Clubs,
                        }),
                        Some(Card {
                            rank: Rank::Ten,
                            suit: Suit::Clubs,
                        }),
                    ],
                ],
                board: [
                    Some(Card {
                        rank: Rank::Nine,
                        suit: Suit::Hearts,
                    }),
                    Some(Card {
                        rank: Rank::Eight,
                        suit: Suit::Hearts,
                    }),
                    Some(Card {
                        rank: Rank::Two,
                        suit: Suit::Spades,
                    }),
                    Some(Card {
                        rank: Rank::Three,
                        suit: Suit::Hearts,
                    }),
                    None,
                ],
                iterations: 75_000,
            },
        ),
        (
            "7s6s vs AdKd vs QcQh | flop 8s-9d-2s | 50k iters",
            Condition {
                players: vec![
                    [
                        Some(Card {
                            rank: Rank::Seven,
                            suit: Suit::Spades,
                        }),
                        Some(Card {
                            rank: Rank::Six,
                            suit: Suit::Spades,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::Ace,
                            suit: Suit::Diamonds,
                        }),
                        Some(Card {
                            rank: Rank::King,
                            suit: Suit::Diamonds,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::Queen,
                            suit: Suit::Clubs,
                        }),
                        Some(Card {
                            rank: Rank::Queen,
                            suit: Suit::Hearts,
                        }),
                    ],
                ],
                board: [
                    Some(Card {
                        rank: Rank::Eight,
                        suit: Suit::Spades,
                    }),
                    Some(Card {
                        rank: Rank::Nine,
                        suit: Suit::Diamonds,
                    }),
                    Some(Card {
                        rank: Rank::Two,
                        suit: Suit::Spades,
                    }),
                    None,
                    None,
                ],
                iterations: 50_000,
            },
        ),
        (
            "As5s vs KdKc vs QhJh vs joueur inconnu | flop 2s-3d-8s | 30k iters",
            Condition {
                players: vec![
                    [
                        Some(Card {
                            rank: Rank::Ace,
                            suit: Suit::Spades,
                        }),
                        Some(Card {
                            rank: Rank::Five,
                            suit: Suit::Spades,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::King,
                            suit: Suit::Diamonds,
                        }),
                        Some(Card {
                            rank: Rank::King,
                            suit: Suit::Clubs,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::Queen,
                            suit: Suit::Hearts,
                        }),
                        Some(Card {
                            rank: Rank::Jack,
                            suit: Suit::Hearts,
                        }),
                    ],
                    [None; 2],
                ],
                board: [
                    Some(Card {
                        rank: Rank::Two,
                        suit: Suit::Spades,
                    }),
                    Some(Card {
                        rank: Rank::Three,
                        suit: Suit::Diamonds,
                    }),
                    Some(Card {
                        rank: Rank::Eight,
                        suit: Suit::Spades,
                    }),
                    None,
                    None,
                ],
                iterations: 50_000,
            },
        ),
        (
            "Ah5h vs JdJc vs 7c6c | flop 4h-5s-6h | 50k iters",
            Condition {
                players: vec![
                    [
                        Some(Card {
                            rank: Rank::Ace,
                            suit: Suit::Hearts,
                        }),
                        Some(Card {
                            rank: Rank::Five,
                            suit: Suit::Hearts,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::Jack,
                            suit: Suit::Diamonds,
                        }),
                        Some(Card {
                            rank: Rank::Jack,
                            suit: Suit::Clubs,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::Seven,
                            suit: Suit::Clubs,
                        }),
                        Some(Card {
                            rank: Rank::Six,
                            suit: Suit::Clubs,
                        }),
                    ],
                ],
                board: [
                    Some(Card {
                        rank: Rank::Four,
                        suit: Suit::Hearts,
                    }),
                    Some(Card {
                        rank: Rank::Five,
                        suit: Suit::Spades,
                    }),
                    Some(Card {
                        rank: Rank::Six,
                        suit: Suit::Hearts,
                    }),
                    None,
                    None,
                ],
                iterations: 30_000,
            },
        ),
        (
            "AhKh vs QsQd | flop Jh-Th-2c | 500k iters",
            Condition {
                players: vec![
                    [
                        Some(Card {
                            rank: Rank::Ace,
                            suit: Suit::Hearts,
                        }),
                        Some(Card {
                            rank: Rank::King,
                            suit: Suit::Hearts,
                        }),
                    ],
                    [
                        Some(Card {
                            rank: Rank::Queen,
                            suit: Suit::Spades,
                        }),
                        Some(Card {
                            rank: Rank::Queen,
                            suit: Suit::Diamonds,
                        }),
                    ],
                ],
                board: [
                    Some(Card {
                        rank: Rank::Jack,
                        suit: Suit::Hearts,
                    }),
                    Some(Card {
                        rank: Rank::Ten,
                        suit: Suit::Hearts,
                    }),
                    Some(Card {
                        rank: Rank::Two,
                        suit: Suit::Clubs,
                    }),
                    None,
                    None,
                ],
                iterations: 500_000,
            },
        ),
    ];

    let to_run: &[usize] = match std::env::args().nth(1).as_deref() {
        Some("sc1") => &[0],
        Some("sc2") => &[1],
        Some("sc3") => &[2],
        Some("sc4") => &[3],
        Some("sc5") => &[4],
        Some("sc6") => &[5],
        _ => &[0, 1, 2, 3, 4, 5],
    };

    let evaluator = NaiveEvaluator;

    for &i in to_run {
        let (name, cond) = &scenarios[i];
        let t = Instant::now();
        let results = evaluator.run(cond);
        let elapsed = t.elapsed();
        let iters_per_sec = cond.iterations as f64 / elapsed.as_secs_f64();

        println!("=== {} ===", name);
        println!("  durée    : {:>8.2?}", elapsed);
        println!("  iters/s  : {:>10.0}", iters_per_sec);
        for (j, o) in results.iter().enumerate() {
            println!(
                "  J{}  win={:5.1}%  pair={:5.1}%  deux-paires={:5.1}%  brelan={:5.1}%  flush={:5.1}%  sf={:5.1}%",
                j + 1,
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
