use eden_settings::Settings;
use eden_utils::build;

pub mod logging;

pub fn print_launch(settings: &Settings) {
    use nu_ansi_term::{Color, Style};
    if eden_utils::build::PROFILE != "release" {
        return;
    }

    let ascii_art = r"
d88888b d8888b. d88888b d8b   db 
88'     88  `8D 88'     888o  88 
88ooooo 88   88 88ooooo 88V8o 88 
88~~~~~ 88   88 88~~~~~ 88 V8o88 
88.     88  .8D 88.     88  V888 
Y88888P Y8888D' Y88888P VP   V8P
";

    let header = Style::new().bold();
    let ascii_art = Style::new().fg(Color::Green).paint(ascii_art);

    eprintln!("{ascii_art}");
    eprintln!(
        "{}:\t{} ({})",
        header.paint("Version"),
        env!("CARGO_PKG_VERSION"),
        build::COMMIT_BRANCH,
    );
    eprintln!("{}:\t{}", header.paint("Commit hash"), build::COMMIT_HASH);
    eprintln!("{}:\t{}", header.paint("Commit date"), &*build::COMMIT_DATE);
    eprintln!();

    if let Some(path) = settings.path() {
        eprintln!("{}:\t{}", header.paint("Settings file"), path.display());
    } else {
        eprintln!("{}:\t<none>", header.paint("Settings file"));
    }
    eprintln!("{}:\t{}", header.paint("Threads"), settings.threads());

    eprintln!();
}
