use egui::Color32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Rank {
    Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten,
    Jack, Queen, King, Ace,
}

impl Rank {
    pub const ALL: [Rank; 13] = [
        Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six,
        Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten,
        Rank::Jack, Rank::Queen, Rank::King, Rank::Ace,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Rank::Two => "2",   Rank::Three => "3", Rank::Four => "4",
            Rank::Five => "5",  Rank::Six => "6",   Rank::Seven => "7",
            Rank::Eight => "8", Rank::Nine => "9",  Rank::Ten => "T",
            Rank::Jack => "J",  Rank::Queen => "Q", Rank::King => "K",
            Rank::Ace => "A",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Suit {
    Spades, Hearts, Diamonds, Clubs,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];

    pub fn symbol(self) -> &'static str {
        match self {
            Suit::Spades => "♠", Suit::Hearts => "♥",
            Suit::Diamonds => "♦", Suit::Clubs => "♣",
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Suit::Hearts | Suit::Diamonds => Color32::from_rgb(220, 50, 50),
            Suit::Spades | Suit::Clubs    => Color32::from_rgb(20, 20, 20),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    pub fn label(self) -> String {
        format!("{}{}", self.rank.label(), self.suit.symbol())
    }
}
