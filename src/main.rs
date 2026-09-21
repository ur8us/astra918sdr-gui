mod app;
mod worker;
use clap::Parser;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    simulator: Option<String>,
    #[arg(long)]
    smoke_test: bool,
}
fn main() -> eframe::Result {
    let args = Args::parse();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([660., 740.])
            .with_min_inner_size([480., 380.]),
        ..Default::default()
    };
    eframe::run_native(
        "Astra918",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, args.simulator, args.smoke_test)))),
    )
}
