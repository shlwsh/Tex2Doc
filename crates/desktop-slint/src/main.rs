use slint::include_modules;

include_modules!();

fn main() {
    let ui = MainWindow::new().unwrap();
    ui.run().unwrap();
}
