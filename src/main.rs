mod app;
mod card;
mod eval;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("Poker Texas Hold'em")
            .with_maximized(true),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Poker Texas Hold'em",
        options,
        Box::new(|_cc| Ok(Box::new(app::PokerApp::default()))),
    );
}
