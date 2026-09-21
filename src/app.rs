use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use std::collections::HashSet;
use crate::card::{Card, Rank, Suit};

const TABLE_FELT:   Color32 = Color32::from_rgb(35, 90, 45);
const TABLE_BORDER: Color32 = Color32::from_rgb(90, 60, 20);
const CARD_EMPTY:   Color32 = Color32::from_rgb(55, 55, 55);
const CARD_FULL:    Color32 = Color32::WHITE;
const MAX_PLAYERS:  usize   = 9;

// Identifies which card slot is being edited
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
    flop:  [Option<Card>; 3],
    turn:  Option<Card>,
    river: Option<Card>,
}

// Probabilités pour un joueur — None = pas encore calculé
#[derive(Clone, Default)]
pub struct HandOdds {
    pub win:        Option<f32>,
    pub pair:       Option<f32>,
    pub two_pair:   Option<f32>,
    pub three_kind: Option<f32>,
    pub straight:   Option<f32>,
    pub flush:      Option<f32>,
    pub full_house: Option<f32>,
    pub four_kind:  Option<f32>,
    pub str_flush:  Option<f32>,
    pub roy_flush:  Option<f32>,
}

pub struct PokerApp {
    players: Vec<Player>,
    board:   Board,
    dealer:  usize,
    picking: Option<CardSlot>,
    odds:    Vec<HandOdds>,
}

impl Default for PokerApp {
    fn default() -> Self {
        Self {
            players: vec![Player::default(); 2],
            board:   Board::default(),
            dealer:  0,
            picking: None,
            odds:    vec![HandOdds::default(); 2],
        }
    }
}

impl PokerApp {
    // All currently assigned cards, excluding the slot being edited (so it can be re-picked)
    fn used_cards(&self) -> HashSet<Card> {
        let p = self.picking;
        let mut set = HashSet::new();
        for (pi, player) in self.players.iter().enumerate() {
            for (ci, c) in player.cards.iter().enumerate() {
                if p != Some(CardSlot::Player(pi, ci)) {
                    if let Some(c) = c { set.insert(*c); }
                }
            }
        }
        for (i, c) in self.board.flop.iter().enumerate() {
            if p != Some(CardSlot::Flop(i)) {
                if let Some(c) = c { set.insert(*c); }
            }
        }
        if p != Some(CardSlot::Turn)  { if let Some(c) = self.board.turn  { set.insert(c); } }
        if p != Some(CardSlot::River) { if let Some(c) = self.board.river { set.insert(c); } }
        set
    }

    fn get_card(&self, slot: CardSlot) -> Option<Card> {
        match slot {
            CardSlot::Player(p, c) => self.players[p].cards[c],
            CardSlot::Flop(i)      => self.board.flop[i],
            CardSlot::Turn         => self.board.turn,
            CardSlot::River        => self.board.river,
        }
    }

    fn set_card(&mut self, slot: CardSlot, card: Option<Card>) {
        match slot {
            CardSlot::Player(p, c) => self.players[p].cards[c] = card,
            CardSlot::Flop(i)      => self.board.flop[i] = card,
            CardSlot::Turn         => self.board.turn = card,
            CardSlot::River        => self.board.river = card,
        }
    }

    fn toggle_picking(&mut self, slot: CardSlot) {
        self.picking = if self.picking == Some(slot) { None } else { Some(slot) };
    }
}

// ── Drawing helpers ─────────────────────────────────────────────────────────

impl PokerApp {
    fn paint_ellipse(painter: &egui::Painter, center: Pos2, rx: f32, ry: f32, color: Color32) {
        let n = 80usize;
        let pts: Vec<Pos2> = (0..n).map(|i| {
            let t = i as f32 * std::f32::consts::TAU / n as f32;
            Pos2::new(center.x + rx * t.cos(), center.y + ry * t.sin())
        }).collect();
        painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
    }

    // Paint a card slot: filled with the card value, or an empty "+" placeholder
    fn paint_card(painter: &egui::Painter, rect: Rect, card: Option<Card>, highlighted: bool) {
        let rounding = CornerRadius::same(3);
        let accent   = Color32::from_rgb(255, 200, 0);

        if let Some(c) = card {
            let bg = if highlighted { Color32::from_rgb(255, 250, 180) } else { CARD_FULL };
            painter.rect_filled(rect, rounding, bg);
            if highlighted {
                painter.add(egui::Shape::rect_stroke(
                    rect, rounding, Stroke::new(2.0, accent), StrokeKind::Inside,
                ));
            }
            painter.text(rect.center(), egui::Align2::CENTER_CENTER,
                c.label(), FontId::proportional(rect.height() * 0.33), c.suit.color());
        } else {
            let border = if highlighted { accent } else { Color32::from_gray(90) };
            painter.rect_filled(rect, rounding, CARD_EMPTY);
            painter.add(egui::Shape::rect_stroke(
                rect, rounding, Stroke::new(1.0, border), StrokeKind::Inside,
            ));
            painter.text(rect.center(), egui::Align2::CENTER_CENTER,
                "+", FontId::proportional(rect.height() * 0.45), border);
        }
    }
}

// ── Table view ──────────────────────────────────────────────────────────────

impl PokerApp {
    fn show_table(&mut self, ui: &mut egui::Ui) {
        let size = Vec2::new(ui.available_width(), 300.0);
        let (resp, painter) = ui.allocate_painter(size, Sense::hover());
        let r      = resp.rect;
        let center = Pos2::new(r.center().x, r.center().y);
        let (rx, ry) = (r.width() * 0.28, r.height() * 0.40);

        Self::paint_ellipse(&painter, center, rx + 14.0, ry + 14.0, TABLE_BORDER);
        Self::paint_ellipse(&painter, center, rx, ry, TABLE_FELT);

        // ── Community cards ────────────────────────────────────────────────
        let cw    = rx * 0.14;
        let ch    = cw * 1.55;
        let gap   = cw * 0.18;
        let total = 5.0 * cw + 4.0 * gap;
        let cx0   = center.x - total / 2.0;
        let cy    = center.y - ch / 2.0;

        painter.text(Pos2::new(center.x, cy - 12.0), egui::Align2::CENTER_CENTER,
            "Community", FontId::proportional(11.0), Color32::from_gray(160));

        let mut clicked: Option<CardSlot> = None;

        for i in 0..5usize {
            let slot = match i { 0..=2 => CardSlot::Flop(i), 3 => CardSlot::Turn, _ => CardSlot::River };
            let rect = Rect::from_min_size(
                Pos2::new(cx0 + i as f32 * (cw + gap), cy),
                Vec2::new(cw, ch),
            );
            Self::paint_card(&painter, rect, self.get_card(slot), self.picking == Some(slot));
            if ui.interact(rect, ui.id().with(("comm", i)), Sense::click()).clicked() {
                clicked = Some(slot);
            }
        }

        // ── Players around the table ───────────────────────────────────────
        let n = self.players.len();
        for idx in 0..n {
            let t  = std::f32::consts::FRAC_PI_2 + idx as f32 * std::f32::consts::TAU / n as f32;
            let px = center.x + rx * 1.22 * t.cos();
            let py = center.y + ry * 1.35 * t.sin();

            // Dealer button (white circle with "D")
            if idx == self.dealer {
                let dp = Pos2::new(px + 18.0, py - 18.0);
                painter.circle_filled(dp, 9.0, Color32::WHITE);
                painter.text(dp, egui::Align2::CENTER_CENTER, "D",
                    FontId::proportional(10.0), Color32::BLACK);
            }

            // Player label — click to assign dealer here
            let label_pos  = Pos2::new(px, py - 22.0);
            let label_rect = Rect::from_center_size(label_pos, Vec2::new(28.0, 16.0));
            painter.text(label_pos, egui::Align2::CENTER_CENTER,
                format!("P{}", idx + 1), FontId::proportional(13.0), Color32::WHITE);
            if ui.interact(label_rect, ui.id().with(("set_dealer", idx)), Sense::click()).clicked() {
                self.dealer = idx;
            }

            // 2 hole card slots
            let pw = cw * 0.85;
            let ph = pw * 1.5;
            for c in 0..2usize {
                let slot   = CardSlot::Player(idx, c);
                let card   = self.players[idx].cards[c];
                let cx     = px - pw - 1.0 + c as f32 * (pw + 2.0);
                let crect  = Rect::from_min_size(Pos2::new(cx, py - ph / 2.0), Vec2::new(pw, ph));
                Self::paint_card(&painter, crect, card, self.picking == Some(slot));
                if ui.interact(crect, ui.id().with(("pcard", idx, c)), Sense::click()).clicked() {
                    clicked = Some(slot);
                }
            }
        }

        if let Some(slot) = clicked {
            self.toggle_picking(slot);
        }
    }
}

// ── Player panels ───────────────────────────────────────────────────────────

impl PokerApp {
    fn show_player_panels(&mut self, ui: &mut egui::Ui) {
        let mut clicked:    Option<CardSlot> = None;
        let mut set_dealer: Option<usize>    = None;
        let mut add_player = false;

        ui.horizontal(|ui| {
            let n = self.players.len();
            for idx in 0..n {
                let is_dealer = self.dealer == idx;
                let cards     = self.players[idx].cards;
                let picking   = self.picking;

                ui.group(|ui| {
                    ui.set_min_width(140.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("P{}", idx + 1)).strong());
                            if is_dealer {
                                ui.label(egui::RichText::new("BTN").color(Color32::YELLOW).small());
                            } else if ui.small_button("Set BTN").clicked() {
                                set_dealer = Some(idx);
                            }
                        });

                        ui.horizontal(|ui| {
                            for c in 0..2usize {
                                let slot      = CardSlot::Player(idx, c);
                                let is_active = picking == Some(slot);
                                let (label, text_color) = match cards[c] {
                                    Some(card) => (
                                        card.label(),
                                        match card.suit {
                                            Suit::Hearts | Suit::Diamonds => Color32::from_rgb(220, 50, 50),
                                            _                             => Color32::from_rgb(15, 15, 15),
                                        },
                                    ),
                                    None => ("?".to_string(), Color32::DARK_GRAY),
                                };

                                let bg = if is_active            { Color32::from_rgb(60, 55, 20) }
                                         else if cards[c].is_some() { Color32::WHITE }
                                         else                       { Color32::from_gray(45) };

                                let btn = egui::Button::new(
                                    egui::RichText::new(&label).size(18.0).color(text_color)
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

            if n < MAX_PLAYERS {
                if ui.button(egui::RichText::new("+").size(22.0))
                    .on_hover_text("Ajouter un joueur")
                    .clicked()
                {
                    add_player = true;
                }
            }
        });

        if let Some(slot) = clicked    { self.toggle_picking(slot); }
        if let Some(idx)  = set_dealer { self.dealer = idx; }
        if add_player {
            self.players.push(Player::default());
            self.odds.push(HandOdds::default());
            self.picking = None;
        }
    }
}

// ── Equity section ──────────────────────────────────────────────────────────

impl PokerApp {
    fn show_equity_section(&self, ui: &mut egui::Ui) {
        ui.separator();
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Équités").strong().size(14.0));
        ui.add_space(6.0);

        // (label, field accessor)
        let hands: &[(&str, fn(&HandOdds) -> Option<f32>)] = &[
            ("Paire",           |o| o.pair),
            ("Double paire",    |o| o.two_pair),
            ("Brelan",          |o| o.three_kind),
            ("Quinte",          |o| o.straight),
            ("Couleur",         |o| o.flush),
            ("Full",            |o| o.full_house),
            ("Carré",           |o| o.four_kind),
            ("Quinte flush",    |o| o.str_flush),
            ("Q. flush royale", |o| o.roy_flush),
        ];

        egui::ScrollArea::horizontal().id_salt("equity_scroll").show(ui, |ui| {
            ui.horizontal_top(|ui| {
                for (idx, odds) in self.odds.iter().enumerate() {
                    ui.group(|ui| {
                        ui.set_min_width(200.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(format!("Joueur {}", idx + 1)).strong());
                            ui.add_space(4.0);

                            // Win probability — prominent green bar
                            let win_val  = odds.win.unwrap_or(0.0);
                            let win_text = fmt_pct("Victoire", odds.win);
                            ui.add(egui::ProgressBar::new(win_val)
                                .text(win_text)
                                .fill(Color32::from_rgb(45, 170, 75))
                                .desired_width(190.0));

                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // Hand probabilities
                            for (name, getter) in hands {
                                let val  = getter(odds).unwrap_or(0.0);
                                let text = fmt_pct(name, getter(odds));
                                ui.add(egui::ProgressBar::new(val)
                                    .text(text)
                                    .fill(Color32::from_rgb(50, 90, 160))
                                    .desired_width(190.0));
                                ui.add_space(2.0);
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
            });
        });
    }
}

fn fmt_pct(label: &str, v: Option<f32>) -> String {
    match v {
        Some(p) => format!("{}: {:.1}%", label, p * 100.0),
        None    => format!("{}: —", label),
    }
}

// ── Card picker popup ────────────────────────────────────────────────────────

impl PokerApp {
    fn show_card_picker(&mut self, ctx: &egui::Context) {
        let picking = match self.picking { Some(p) => p, None => return };
        let used    = self.used_cards();
        let current = self.get_card(picking);

        let mut selected: Option<Card> = None;
        let mut remove = false;
        let mut close  = false;

        egui::Window::new("Choisir une carte")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Grid::new("picker")
                    .spacing(Vec2::new(3.0, 3.0))
                    .show(ui, |ui| {
                        ui.label("");
                        for suit in Suit::ALL {
                            ui.colored_label(suit.color(), suit.symbol());
                        }
                        ui.end_row();

                        for rank in Rank::ALL {
                            ui.label(egui::RichText::new(rank.label()).strong());
                            for suit in Suit::ALL {
                                let card    = Card { rank, suit };
                                let is_used = used.contains(&card);
                                let is_cur  = current == Some(card);

                                let text = egui::RichText::new(card.label())
                                    .size(13.0)
                                    .color(if is_used { Color32::DARK_GRAY } else { suit.color() });

                                let btn = egui::Button::new(text)
                                    .min_size(Vec2::new(38.0, 26.0))
                                    .fill(if is_cur { Color32::from_rgb(70, 60, 10) } else { Color32::from_gray(38) });

                                if ui.add_enabled(!is_used, btn).clicked() {
                                    selected = Some(card);
                                }
                            }
                            ui.end_row();
                        }
                    });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Fermer").clicked() { close = true; }
                    if current.is_some() {
                        if ui.button(egui::RichText::new("Supprimer").color(Color32::LIGHT_RED)).clicked() {
                            remove = true;
                        }
                    }
                });
            });

        if let Some(card) = selected {
            self.set_card(picking, Some(card));
            self.picking = None;
        } else if remove {
            self.set_card(picking, None);
            self.picking = None;
        } else if close {
            self.picking = None;
        }
    }
}

// ── eframe::App ─────────────────────────────────────────────────────────────

impl eframe::App for PokerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().set_visuals(egui::Visuals::dark());
        self.show_table(ui);
        ui.add_space(16.0);
        self.show_player_panels(ui);

        ui.add_space(12.0);
        self.show_equity_section(ui);

        if self.picking.is_some() {
            let ctx = ui.ctx().clone();
            self.show_card_picker(&ctx);
        }
    }
}
