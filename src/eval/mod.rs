pub mod eval7;
pub mod fisher;
pub mod lut;
pub mod naive;
pub mod sort_free;
pub mod zero_alloc;

use crate::card::Card;

// ── Condition de simulation ───────────────────────────────────────────────────

/// État de jeu à évaluer : utilisé comme entrée standard pour tous les évaluateurs,
/// ce qui permet de lancer plusieurs algos sur la même condition et de les comparer.
pub struct Condition {
    pub players: Vec<[Option<Card>; 2]>,
    pub board: [Option<Card>; 5],
    pub iterations: u32,
}

// ── Résultats par joueur ──────────────────────────────────────────────────────

pub struct SimOdds {
    pub win: f32,
    pub high_card: f32,
    pub pair: f32,
    pub two_pair: f32,
    pub three_kind: f32,
    pub straight: f32,
    pub flush: f32,
    pub full_house: f32,
    pub four_kind: f32,
    pub str_flush: f32,
    pub roy_flush: f32,
}

// ── Trait commun ──────────────────────────────────────────────────────────────

pub trait Evaluator {
    fn run(&self, condition: &Condition) -> Vec<SimOdds>;
}
