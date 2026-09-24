use crate::card::{Card, Rank, Suit};
use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use std::collections::HashSet;

// ── Constants ────────────────────────────────────────────────────────────────

const TABLE_FELT: Color32 = Color32::from_rgb(35, 90, 45);
const TABLE_BORDER: Color32 = Color32::from_rgb(90, 60, 20);
const CARD_EMPTY: Color32 = Color32::from_rgb(55, 55, 55);
const CARD_FULL: Color32 = Color32::WHITE;
const MAX_PLAYERS: usize = 9;

// Table geometry (screen-space, fixed)
const TABLE_RY: f32 = 108.0; // oval vertical radius
const PAINTER_H: f32 = 450.0; // vertical space allocated to the table painter
// PLAYER_RY = TABLE_RY + gap; kept just outside the table border (14px)
const PLAYER_GAP: f32 = 48.0; // gap between table border and player center

// Card sizes
const P_CARD_W: f32 = 30.0; // player hole cards (table view)
const P_CARD_H: f32 = 44.0;
const C_CARD_W: f32 = 46.0; // community cards
const C_CARD_H: f32 = 69.0;
const C_GAP: f32 = 6.0;

// ── Data ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum CardSlot {
    Player(usize, usize), // (player_idx, card_idx)
    Flop(usize),
    Turn,
    River,
}

#[derive(Clone, Default)]
struct Player {
    cards: [Option<Card>; 2],
}

#[derive(Default)]
struct Board {
    flop: [Option<Card>; 3],
    turn: Option<Card>,
    river: Option<Card>,
}

type HandOddsExtractor = fn(&HandOdds) -> Option<f32>;

// Per-player probabilities — None means not yet calculated
#[derive(Clone, Default)]
pub struct HandOdds {
    pub win: Option<f32>,
    pub high_card: Option<f32>,
    pub pair: Option<f32>,
    pub two_pair: Option<f32>,
    pub three_kind: Option<f32>,
    pub straight: Option<f32>,
    pub flush: Option<f32>,
    pub full_house: Option<f32>,
    pub four_kind: Option<f32>,
    pub str_flush: Option<f32>,
    pub roy_flush: Option<f32>,
}

pub struct PokerApp {
    players: Vec<Player>,
    board: Board,
    picking: Option<CardSlot>,
    odds: Vec<HandOdds>,
}

impl Default for PokerApp {
    fn default() -> Self {
        Self {
            players: vec![Player::default(); 2],
            board: Board::default(),
            picking: None,
            odds: vec![HandOdds::default(); 2],
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

impl PokerApp {
    fn used_cards(&self) -> HashSet<Card> {
        let p = self.picking;
        let mut set = HashSet::new();
        for (pi, player) in self.players.iter().enumerate() {
            for (ci, c) in player.cards.iter().enumerate() {
                if p != Some(CardSlot::Player(pi, ci))
                    && let Some(c) = c
                {
                    set.insert(*c);
                }
            }
        }
        for (i, c) in self.board.flop.iter().enumerate() {
            if p != Some(CardSlot::Flop(i))
                && let Some(c) = c
            {
                set.insert(*c);
            }
        }
        if p != Some(CardSlot::Turn)
            && let Some(c) = self.board.turn
        {
            set.insert(c);
        }
        if p != Some(CardSlot::River)
            && let Some(c) = self.board.river
        {
            set.insert(c);
        }
        set
    }

    fn get_card(&self, slot: CardSlot) -> Option<Card> {
        match slot {
            CardSlot::Player(p, c) => self.players[p].cards[c],
            CardSlot::Flop(i) => self.board.flop[i],
            CardSlot::Turn => self.board.turn,
            CardSlot::River => self.board.river,
        }
    }

    fn set_card(&mut self, slot: CardSlot, card: Option<Card>) {
        match slot {
            CardSlot::Player(p, c) => self.players[p].cards[c] = card,
            CardSlot::Flop(i) => self.board.flop[i] = card,
            CardSlot::Turn => self.board.turn = card,
            CardSlot::River => self.board.river = card,
        }
    }

    fn toggle_picking(&mut self, slot: CardSlot) {
        self.picking = if self.picking == Some(slot) {
            None
        } else {
            Some(slot)
        };
    }

    fn next_slot(&self, slot: CardSlot) -> CardSlot {
        let n = self.players.len();
        match slot {
            CardSlot::Player(p, 0) => CardSlot::Player(p, 1),
            CardSlot::Player(p, 1) => CardSlot::Player((p + 1) % n, 0),
            CardSlot::Flop(0) => CardSlot::Flop(1),
            CardSlot::Flop(1) => CardSlot::Flop(2),
            CardSlot::Flop(2) => CardSlot::Turn,
            CardSlot::Turn => CardSlot::River,
            CardSlot::River => CardSlot::River,
            _ => slot,
        }
    }
}

// ── Painter helpers ──────────────────────────────────────────────────────────

fn paint_ellipse(painter: &egui::Painter, center: Pos2, rx: f32, ry: f32, color: Color32) {
    const N: usize = 80;
    let pts: Vec<Pos2> = (0..N)
        .map(|i| {
            let t = i as f32 * std::f32::consts::TAU / N as f32;
            Pos2::new(center.x + rx * t.cos(), center.y + ry * t.sin())
        })
        .collect();
    painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
}

fn paint_card(painter: &egui::Painter, rect: Rect, card: Option<Card>, highlighted: bool) {
    let rounding = CornerRadius::same(3);
    let accent = Color32::from_rgb(255, 200, 0);
    if let Some(c) = card {
        let bg = if highlighted {
            Color32::from_rgb(255, 250, 180)
        } else {
            CARD_FULL
        };
        painter.rect_filled(rect, rounding, bg);
        if highlighted {
            painter.add(egui::Shape::rect_stroke(
                rect,
                rounding,
                Stroke::new(2.0, accent),
                StrokeKind::Inside,
            ));
        }
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            c.label(),
            FontId::proportional(rect.height() * 0.30),
            c.suit.color(),
        );
    } else {
        let border = if highlighted {
            accent
        } else {
            Color32::from_gray(90)
        };
        painter.rect_filled(rect, rounding, CARD_EMPTY);
        painter.add(egui::Shape::rect_stroke(
            rect,
            rounding,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        ));
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "+",
            FontId::proportional(rect.height() * 0.42),
            border,
        );
    }
}

// ── Table view ───────────────────────────────────────────────────────────────

impl PokerApp {
    fn show_table(&mut self, ui: &mut egui::Ui) {
        let avail_w = ui.available_width();
        let (resp, painter) = ui.allocate_painter(Vec2::new(avail_w, PAINTER_H), Sense::hover());
        let r = resp.rect;
        let center = r.center();

        // Table rx scales with window width but is capped so players stay in the painter area
        let table_rx = (avail_w * 0.22).clamp(160.0, 260.0);
        let player_rx = table_rx + 14.0 + PLAYER_GAP; // just outside the border
        let player_ry = TABLE_RY + 14.0 + PLAYER_GAP;

        // Draw table (border then felt)
        paint_ellipse(
            &painter,
            center,
            table_rx + 14.0,
            TABLE_RY + 14.0,
            TABLE_BORDER,
        );
        paint_ellipse(&painter, center, table_rx, TABLE_RY, TABLE_FELT);

        // ── Community cards ────────────────────────────────────────────────
        let comm_w = 5.0 * C_CARD_W + 4.0 * C_GAP;
        let cx0 = center.x - comm_w / 2.0;
        let cy = center.y - C_CARD_H / 2.0;

        painter.text(
            Pos2::new(center.x, cy - 14.0),
            egui::Align2::CENTER_CENTER,
            "Community",
            FontId::proportional(11.0),
            Color32::from_gray(155),
        );

        let mut clicked: Option<CardSlot> = None;

        for i in 0..5usize {
            let slot = match i {
                0..=2 => CardSlot::Flop(i),
                3 => CardSlot::Turn,
                _ => CardSlot::River,
            };
            let rect = Rect::from_min_size(
                Pos2::new(cx0 + i as f32 * (C_CARD_W + C_GAP), cy),
                Vec2::new(C_CARD_W, C_CARD_H),
            );
            paint_card(
                &painter,
                rect,
                self.get_card(slot),
                self.picking == Some(slot),
            );
            if ui
                .interact(rect, ui.id().with(("comm", i)), Sense::click())
                .clicked()
            {
                clicked = Some(slot);
            }
        }

        // ── Players evenly distributed around the oval ────────────────────
        let n = self.players.len();
        for idx in 0..n {
            let t = std::f32::consts::FRAC_PI_2 + idx as f32 * std::f32::consts::TAU / n as f32;
            let px = center.x + player_rx * t.cos();
            let py = center.y + player_ry * t.sin();

            // Player label
            let label_pos = Pos2::new(px, py - P_CARD_H / 2.0 - 14.0);
            painter.text(
                label_pos,
                egui::Align2::CENTER_CENTER,
                format!("P{}", idx + 1),
                FontId::proportional(13.0),
                Color32::WHITE,
            );

            // 2 hole cards
            for c in 0..2usize {
                let slot = CardSlot::Player(idx, c);
                let card = self.players[idx].cards[c];
                let cx = px - P_CARD_W - 2.0 + c as f32 * (P_CARD_W + 4.0);
                let crect = Rect::from_min_size(
                    Pos2::new(cx, py - P_CARD_H / 2.0),
                    Vec2::new(P_CARD_W, P_CARD_H),
                );
                paint_card(&painter, crect, card, self.picking == Some(slot));
                if ui
                    .interact(crect, ui.id().with(("pcard", idx, c)), Sense::click())
                    .clicked()
                {
                    clicked = Some(slot);
                }
            }
        }

        if let Some(slot) = clicked {
            self.toggle_picking(slot);
        }
    }
}

// ── Player panels ─────────────────────────────────────────────────────────────

impl PokerApp {
    fn show_player_panels(&mut self, ui: &mut egui::Ui) {
        let mut clicked: Option<CardSlot> = None;
        let mut add_player = false;

        egui::ScrollArea::horizontal()
            .id_salt("panels_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let n = self.players.len();
                    for idx in 0..n {
                        let cards = self.players[idx].cards;
                        let picking = self.picking;

                        ui.group(|ui| {
                            ui.set_min_width(140.0);
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(format!("P{}", idx + 1)).strong());

                                ui.horizontal(|ui| {
                                    for (c, &card_opt) in cards.iter().enumerate() {
                                        let slot = CardSlot::Player(idx, c);
                                        let is_active = picking == Some(slot);
                                        let (label, text_color) = match card_opt {
                                            Some(card) => (
                                                card.label(),
                                                match card.suit {
                                                    Suit::Hearts | Suit::Diamonds => {
                                                        Color32::from_rgb(220, 50, 50)
                                                    }
                                                    _ => Color32::from_rgb(15, 15, 15),
                                                },
                                            ),
                                            None => ("?".to_string(), Color32::DARK_GRAY),
                                        };

                                        let bg = if is_active {
                                            Color32::from_rgb(255, 220, 50)
                                        } else if card_opt.is_some() {
                                            Color32::WHITE
                                        } else {
                                            Color32::from_gray(45)
                                        };

                                        let btn = egui::Button::new(
                                            egui::RichText::new(&label)
                                                .size(18.0)
                                                .color(text_color),
                                        )
                                        .min_size(Vec2::new(48.0, 48.0))
                                        .fill(bg);

                                        if ui.add(btn).clicked() {
                                            clicked = Some(slot);
                                        }
                                    }
                                });
                            });
                        });
                    }

                    if n < MAX_PLAYERS
                        && ui
                            .button(egui::RichText::new("+").size(22.0))
                            .on_hover_text("Ajouter un joueur")
                            .clicked()
                    {
                        add_player = true;
                    }
                });
            });

        if let Some(slot) = clicked {
            self.toggle_picking(slot);
        }
        if add_player {
            self.players.push(Player::default());
            self.odds.push(HandOdds::default());
            self.picking = None;
        }
    }
}

// ── Equity section ─────────────────────────────────────────────────────────

impl PokerApp {
    fn show_equity_section(&self, ui: &mut egui::Ui) {
        ui.separator();
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Équités").strong().size(14.0));
        ui.add_space(8.0);

        let hand_rows: &[(&str, HandOddsExtractor)] = &[
            ("Carte haute", |o: &HandOdds| o.high_card),
            ("Paire", |o: &HandOdds| o.pair),
            ("Double paire", |o: &HandOdds| o.two_pair),
            ("Brelan", |o: &HandOdds| o.three_kind),
            ("Quinte", |o: &HandOdds| o.straight),
            ("Couleur", |o: &HandOdds| o.flush),
            ("Full", |o: &HandOdds| o.full_house),
            ("Carré", |o: &HandOdds| o.four_kind),
            ("Quinte flush", |o: &HandOdds| o.str_flush),
            ("Q. flush royale", |o: &HandOdds| o.roy_flush),
        ];

        egui::ScrollArea::horizontal()
            .id_salt("equity_scroll")
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    for (idx, odds) in self.odds.iter().enumerate() {
                        ui.group(|ui| {
                            ui.set_min_width(270.0);
                            ui.set_max_width(270.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Joueur {}", idx + 1)).strong(),
                                );
                                ui.add_space(6.0);

                                // Win probability — prominent green bar
                                equity_row(
                                    ui,
                                    "Victoire",
                                    odds.win,
                                    Color32::from_rgb(45, 170, 75),
                                    true,
                                );

                                ui.add_space(6.0);
                                ui.separator();
                                ui.add_space(4.0);

                                // Hand probabilities
                                for (name, getter) in hand_rows {
                                    equity_row(
                                        ui,
                                        name,
                                        getter(odds),
                                        Color32::from_rgb(55, 90, 170),
                                        false,
                                    );
                                    ui.add_space(2.0);
                                }
                            });
                        });
                        ui.add_space(6.0);
                    }
                });
            });
    }
}

fn equity_row(ui: &mut egui::Ui, name: &str, val: Option<f32>, color: Color32, prominent: bool) {
    let h = if prominent { 22.0 } else { 16.0 };
    let font_sz = if prominent { 13.0 } else { 12.0 };
    let pct = val.unwrap_or(0.0);
    let pct_text = match val {
        Some(v) => format!("{:.1}%", v * 100.0),
        None => "—".to_string(),
    };

    ui.horizontal(|ui| {
        // Fixed-width label
        ui.add_sized(
            [108.0, h],
            egui::Label::new(egui::RichText::new(name).size(font_sz)),
        );
        // Fixed-width progress bar
        ui.add_sized([110.0, h], egui::ProgressBar::new(pct).fill(color));
        // Percentage label
        ui.add_sized(
            [42.0, h],
            egui::Label::new(egui::RichText::new(pct_text).size(font_sz)),
        );
    });
}

// ── Card picker popup ─────────────────────────────────────────────────────────

impl PokerApp {
    fn show_card_picker(&mut self, ctx: &egui::Context) {
        let picking = match self.picking {
            Some(p) => p,
            None => return,
        };
        let used = self.used_cards();
        let current = self.get_card(picking);

        let mut selected: Option<Card> = None;
        let mut remove = false;
        let mut close = false;
        let mut pick_random = false;
        let shift_held = ctx.input(|i| i.modifiers.shift);

        egui::Window::new("Choisir une carte")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Grid::new("picker")
                    .spacing(Vec2::new(3.0, 3.0))
                    .show(ui, |ui| {
                        for rank in Rank::ALL {
                            ui.label(egui::RichText::new(rank.label()).strong());
                            for suit in Suit::ALL {
                                let card = Card { rank, suit };
                                let is_used = used.contains(&card);
                                let is_cur = current == Some(card);

                                let text = egui::RichText::new(card.label()).size(13.0).color(
                                    if is_used {
                                        Color32::DARK_GRAY
                                    } else {
                                        suit.color()
                                    },
                                );

                                let btn = egui::Button::new(text)
                                    .min_size(Vec2::new(38.0, 26.0))
                                    .fill(if is_cur {
                                        Color32::from_rgb(255, 220, 50)
                                    } else if is_used {
                                        Color32::from_gray(210)
                                    } else {
                                        Color32::WHITE
                                    });

                                if ui.add_enabled(!is_used, btn).clicked() {
                                    selected = Some(card);
                                }
                            }
                            ui.end_row();
                        }
                    });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Fermer").clicked() {
                        close = true;
                    }
                    if ui
                        .button(
                            egui::RichText::new("Aléatoire").color(Color32::from_rgb(80, 160, 220)),
                        )
                        .clicked()
                    {
                        pick_random = true;
                    }
                    if current.is_some()
                        && ui
                            .button(egui::RichText::new("Supprimer").color(Color32::LIGHT_RED))
                            .clicked()
                    {
                        remove = true;
                    }
                });
            });

        if pick_random {
            let available: Vec<Card> = Rank::ALL
                .iter()
                .flat_map(|&r| Suit::ALL.iter().map(move |&s| Card { rank: r, suit: s }))
                .filter(|c| !used.contains(c))
                .collect();
            if !available.is_empty() {
                use rand::seq::IndexedRandom;
                let card = *available.choose(&mut rand::rng()).unwrap();
                self.set_card(picking, Some(card));
                if shift_held {
                    self.picking = Some(self.next_slot(picking));
                } else {
                    self.picking = None;
                }
            }
        } else if let Some(card) = selected {
            self.set_card(picking, Some(card));
            if shift_held {
                self.picking = Some(self.next_slot(picking));
            } else {
                self.picking = None;
            }
        } else if remove {
            self.set_card(picking, None);
            self.picking = None;
        } else if close {
            self.picking = None;
        }
    }
}

// ── Reset ─────────────────────────────────────────────────────────────────────

impl PokerApp {
    fn reset(&mut self) {
        let n = self.players.len();
        self.players = vec![Player::default(); n];
        self.board = Board::default();
        self.odds = vec![HandOdds::default(); n];
        self.picking = None;
    }
}

// ── Simulation ────────────────────────────────────────────────────────────────

impl PokerApp {
    fn run_simulation(&mut self) {
        use crate::eval::{Condition, Evaluator, lut_par::LutParEvaluator};
        let condition = Condition {
            players: self.players.iter().map(|p| p.cards).collect(),
            board: [
                self.board.flop[0],
                self.board.flop[1],
                self.board.flop[2],
                self.board.turn,
                self.board.river,
            ],
            iterations: 100_000,
        };
        let results = LutParEvaluator.run(&condition);
        self.odds = results
            .into_iter()
            .map(|r| HandOdds {
                win: Some(r.win),
                high_card: Some(r.high_card),
                pair: Some(r.pair),
                two_pair: Some(r.two_pair),
                three_kind: Some(r.three_kind),
                straight: Some(r.straight),
                flush: Some(r.flush),
                full_house: Some(r.full_house),
                four_kind: Some(r.four_kind),
                str_flush: Some(r.str_flush),
                roy_flush: Some(r.roy_flush),
            })
            .collect();
    }
}

// ── eframe::App ───────────────────────────────────────────────────────────────

impl eframe::App for PokerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().set_visuals(egui::Visuals::dark());

        self.show_table(ui);

        egui::ScrollArea::vertical()
            .id_salt("bottom_scroll")
            .show(ui, |ui| {
                ui.add_space(8.0);
                self.show_player_panels(ui);
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui
                        .button(egui::RichText::new("Calculer les équités").size(13.0))
                        .clicked()
                    {
                        self.run_simulation();
                    }
                    if ui
                        .button(
                            egui::RichText::new("Reset cartes")
                                .size(13.0)
                                .color(Color32::from_rgb(220, 80, 60)),
                        )
                        .clicked()
                    {
                        self.reset();
                    }
                });

                ui.add_space(4.0);
                self.show_equity_section(ui);
                ui.add_space(8.0);
            });

        if self.picking.is_some() {
            let ctx = ui.ctx().clone();
            self.show_card_picker(&ctx);
        }
    }
}
