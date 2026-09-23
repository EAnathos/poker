use poker::card::{Card, Rank, Suit};
use poker::eval::{
    Condition, Evaluator, eval7::Eval7Evaluator, fisher::FisherEvaluator, lut::LutEvaluator,
    naive::NaiveEvaluator, sort_free::SortFreeEvaluator, zero_alloc::ZeroAllocEvaluator,
};
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

    // ── Parsing des arguments ─────────────────────────────────────────────────
    // Usage : bench [sc1..sc6] [naive] [zero_alloc] [sort_free]  (ordre libre, tout optionnel)
    // Exemples :
    //   bench sc6 zero_alloc sort_free   → compare les deux sur sc6
    //   bench sort_free                  → tous les scénarios, sort_free uniquement
    //   bench                            → tous les scénarios, les trois évaluateurs
    let mut sc_filter: Option<usize> = None;
    let mut evals: Vec<&'static str> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "sc1" => sc_filter = Some(0),
            "sc2" => sc_filter = Some(1),
            "sc3" => sc_filter = Some(2),
            "sc4" => sc_filter = Some(3),
            "sc5" => sc_filter = Some(4),
            "sc6" => sc_filter = Some(5),
            "naive" => {
                if !evals.contains(&"naive") {
                    evals.push("naive");
                }
            }
            "zero_alloc" => {
                if !evals.contains(&"zero_alloc") {
                    evals.push("zero_alloc");
                }
            }
            "sort_free" => {
                if !evals.contains(&"sort_free") {
                    evals.push("sort_free");
                }
            }
            "fisher" => {
                if !evals.contains(&"fisher") {
                    evals.push("fisher");
                }
            }
            "eval7" => {
                if !evals.contains(&"eval7") {
                    evals.push("eval7");
                }
            }
            "lut" => {
                if !evals.contains(&"lut") {
                    evals.push("lut");
                }
            }
            other => eprintln!("argument inconnu ignoré : {other}"),
        }
    }
    if evals.is_empty() {
        evals.push("naive");
        evals.push("zero_alloc");
        evals.push("fisher");
        evals.push("eval7");
        evals.push("lut");
    }

    let to_run: Vec<usize> = match sc_filter {
        Some(i) => vec![i],
        None => (0..scenarios.len()).collect(),
    };

    for &i in &to_run {
        let (name, cond) = &scenarios[i];
        println!("=== {} ===", name);

        // Exécute chaque évaluateur demandé et collecte (durée, ips, résultats)
        let mut runs: Vec<(&str, std::time::Duration, f64, Vec<poker::eval::SimOdds>)> = Vec::new();
        for &eval_name in &evals {
            let ev: Box<dyn Evaluator> = match eval_name {
                "zero_alloc" => Box::new(ZeroAllocEvaluator),
                "sort_free" => Box::new(SortFreeEvaluator),
                "fisher" => Box::new(FisherEvaluator),
                "eval7" => Box::new(Eval7Evaluator),
                "lut" => Box::new(LutEvaluator),
                _ => Box::new(NaiveEvaluator),
            };
            let t = Instant::now();
            let results = ev.run(cond);
            let elapsed = t.elapsed();
            let ips = cond.iterations as f64 / elapsed.as_secs_f64();
            runs.push((eval_name, elapsed, ips, results));
        }

        // Affichage des timings (avec speedup si plusieurs évaluateurs)
        let base_ips = runs[0].2;
        for (k, (eval_name, elapsed, ips, _)) in runs.iter().enumerate() {
            if runs.len() == 1 {
                println!("  durée    : {:>8.2?}", elapsed);
                println!("  iters/s  : {:>10.0}", ips);
            } else {
                let speedup = if k == 0 {
                    String::new()
                } else {
                    format!("   ×{:.2} vs {}", ips / base_ips, runs[0].0)
                };
                println!(
                    "  [{eval_name:<10}]  {:>8.2?}   {:>10.0} iters/s{speedup}",
                    elapsed, ips
                );
            }
        }

        // Statistiques joueurs depuis le dernier évaluateur lancé
        for (j, o) in runs.last().unwrap().3.iter().enumerate() {
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
