use readme_tui::sources::{Raw, assemble};
use readme_tui::ui::{Tab, draw};

fn main() {
    let read = |name: &str| std::fs::read_to_string(format!("tests/fixtures/{name}")).ok();
    let profile = assemble(&Raw {
        stats: read("stats.svg"),
        top_langs: read("top-langs.svg"),
        pin_spotatui: read("pin-spotatui.svg"),
        pin_steamie: read("pin-steamie.svg"),
        pin_steam_cm_protocol: None,
        followers: read("followers.json"),
        stars: read("stars.json"),
        calendar: read("calendar.json"),
        views: read("komarev.svg"),
        lanyard: read("lanyard.json"),
        now_playing: Some(
            r#"{"state":"playing","title":"Sample Track","subtitle":"Sample Artist · Sample Album","progressMs":83000,"durationMs":225000}"#.into(),
        ),
    });
    std::fs::create_dir_all("preview").unwrap();
    for tab in Tab::ALL {
        let path = format!("preview/{}.svg", tab.slug());
        std::fs::write(&path, readme_tui::svg::render(&draw(&profile, tab))).unwrap();
        println!("wrote {path}");
    }
}
