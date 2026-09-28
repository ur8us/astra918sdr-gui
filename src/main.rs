mod app;
mod worker;
use clap::Parser;
#[derive(Parser)]
struct Args {
    #[cfg(debug_assertions)]
    #[arg(long)]
    simulator: Option<String>,
    #[arg(long)]
    smoke_test: bool,
}
fn main() -> eframe::Result {
    let args = Args::parse();
    #[cfg(debug_assertions)]
    let simulator = args.simulator;
    #[cfg(not(debug_assertions))]
    let simulator = None;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([660., 180.])
            .with_min_inner_size([480., 160.]),
        #[cfg(windows)]
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    #[cfg(windows)]
    let mut options = options;
    #[cfg(windows)]
    if std::env::var_os("WINELOADER").is_some() && std::env::var_os("WGPU_BACKEND").is_none() {
        // Wine may expose a DirectX adapter that fails device creation.
        // Prefer its Vulkan path, retaining GLES for systems without Vulkan.
        if let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup
        {
            setup.instance_descriptor.backends = wgpu::Backends::VULKAN | wgpu::Backends::GL;
        }
    }
    eframe::run_native(
        "Astra918",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, simulator, args.smoke_test)))),
    )
}
