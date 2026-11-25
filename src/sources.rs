//! Parsers for the services the README already embeds, so the TUI shows the same values.

use ratatui::style::Color;
use serde_json::Value;

use crate::ui::{Language, Presence, Profile, RepoCard, Stats, Track};

/// Raw response bodies, each `None` when its fetch failed.
#[derive(Default)]
pub struct Raw {
    pub stats: Option<String>,
    pub top_langs: Option<String>,
    pub pin_spotatui: Option<String>,
    pub pin_steamie: Option<String>,
    pub pin_steam_cm_protocol: Option<String>,
    pub followers: Option<String>,
    pub stars: Option<String>,
    pub calendar: Option<String>,
    pub views: Option<String>,
    pub lanyard: Option<String>,
    pub now_playing: Option<String>,
}

/// Builds the profile, showing `?` for any value whose source failed.
pub fn assemble(raw: &Raw) -> Profile {
    let unknown = || "?".to_owned();
    let pin = |body: &Option<String>, name: &str| {
        body.as_deref().and_then(pin_card).unwrap_or(RepoCard {
            name: name.to_owned(),
            description: None,
            stars: unknown(),
            forks: unknown(),
            language: None,
        })
    };
    let (contributions, total_contributions) = raw
        .calendar
        .as_deref()
        .and_then(calendar)
        .unwrap_or_default();
    Profile {
        now_playing: raw.now_playing.as_deref().and_then(now_playing),
        cover_art: None,
        discord: raw.lanyard.as_deref().and_then(lanyard),
        views: raw
            .views
            .as_deref()
            .and_then(komarev_badge)
            .unwrap_or_else(unknown),
        followers: raw
            .followers
            .as_deref()
            .and_then(shields)
            .unwrap_or_else(unknown),
        stars: raw
            .stars
            .as_deref()
            .and_then(shields)
            .unwrap_or_else(unknown),
        spotatui: pin(&raw.pin_spotatui, "spotatui"),
        steamie: pin(&raw.pin_steamie, "steamie"),
        steam_cm_protocol: pin(&raw.pin_steam_cm_protocol, "steam-cm-protocol"),
        stats: raw.stats.as_deref().and_then(stats_card).unwrap_or(Stats {
            stars: unknown(),
            commits: unknown(),
            prs: unknown(),
            issues: unknown(),
            contributed_to: unknown(),
            rank: unknown(),
        }),
        languages: raw
            .top_langs
            .as_deref()
            .and_then(top_langs_card)
            .unwrap_or_default(),
        contributions,
        total_contributions,
    }
}

/// The trimmed text content of the first element after `marker`.
fn text_after<'a>(svg: &'a str, marker: &str) -> Option<&'a str> {
    let rest = &svg[svg.find(marker)? + marker.len()..];
    let rest = &rest[rest.find('>')? + 1..];
    Some(rest[..rest.find('<')?].trim())
}

fn color(hex: &str) -> Option<Color> {
    let v = u32::from_str_radix(hex.strip_prefix('#')?, 16).ok()?;
    let [_, r, g, b] = v.to_be_bytes();
    Some(Color::Rgb(r, g, b))
}

/// The github-readme-stats stats card.
pub fn stats_card(svg: &str) -> Option<Stats> {
    let stat = |id: &str| text_after(svg, &format!(r#"data-testid="{id}""#)).map(str::to_owned);
    Some(Stats {
        stars: stat("stars")?,
        commits: stat("commits")?,
        prs: stat("prs")?,
        issues: stat("issues")?,
        contributed_to: stat("contribs")?,
        rank: stat("level-rank-icon")?,
    })
}

/// The github-readme-stats compact top-languages card: one coloured circle per `lang-name` entry.
pub fn top_langs_card(svg: &str) -> Option<Vec<Language>> {
    let colors = svg
        .split("<circle")
        .skip(1)
        .filter_map(|c| color(c.split(r#"fill=""#).nth(1)?.split('"').next()?));
    let names = svg
        .split(r#"data-testid="lang-name""#)
        .skip(1)
        .filter_map(|s| text_after(s, ""));
    let langs: Vec<Language> = names
        .zip(colors)
        .filter_map(|(text, color)| {
            let (name, pct) = text.rsplit_once(' ')?;
            Some(Language {
                name: name.to_owned(),
                percent: pct.strip_suffix('%')?.parse().ok()?,
                color,
            })
        })
        .collect();
    (!langs.is_empty()).then_some(langs)
}

/// The github-readme-stats repository pin card.
pub fn pin_card(svg: &str) -> Option<RepoCard> {
    let field = |id: &str| text_after(svg, &format!(r#"data-testid="{id}""#)).map(str::to_owned);
    let description: Vec<&str> = svg
        .split(r#"class="description""#)
        .nth(1)?
        .split("</text>")
        .next()?
        .split("<tspan")
        .skip(1)
        .filter_map(|t| text_after(t, ""))
        .collect();
    let description = description.join(" ");
    let language = field("lang-name").zip(
        svg.split(r#"data-testid="lang-color""#)
            .nth(1)
            .and_then(|c| color(c.split(r#"fill=""#).nth(1)?.split('"').next()?)),
    );
    Some(RepoCard {
        name: field("header")?,
        description: (!description.is_empty() && description != "No description provided")
            .then_some(description),
        stars: field("stargazers")?,
        forks: field("forkcount")?,
        language,
    })
}

/// The komarev profile-view badge; its last text node is the count.
pub fn komarev_badge(svg: &str) -> Option<String> {
    let last = svg.rsplit("<text").next()?;
    text_after(last, "").map(str::to_owned)
}

/// A shields.io `.json` badge.
pub fn shields(json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(json).ok()?;
    Some(v["message"].as_str()?.to_owned())
}

/// Lanyard's `/v1/users/<id>` response.
pub fn lanyard(json: &str) -> Option<Presence> {
    let v: Value = serde_json::from_str(json).ok()?;
    let data = &v["data"];
    Some(Presence {
        username: data["discord_user"]["username"].as_str()?.to_owned(),
        tag: data["discord_user"]["primary_guild"]["tag"]
            .as_str()
            .map(str::to_owned),
        status: data["discord_status"].as_str()?.to_owned(),
    })
}

/// GitHub's contribution calendar: levels 0..=4 per day, oldest first, plus the yearly total.
pub fn calendar(json: &str) -> Option<(Vec<u8>, u64)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let cal = &v["data"]["user"]["contributionsCollection"]["contributionCalendar"];
    let levels = cal["weeks"]
        .as_array()?
        .iter()
        .flat_map(|w| w["contributionDays"].as_array().into_iter().flatten())
        .map(|d| match d["contributionLevel"].as_str() {
            Some("FIRST_QUARTILE") => 1,
            Some("SECOND_QUARTILE") => 2,
            Some("THIRD_QUARTILE") => 3,
            Some("FOURTH_QUARTILE") => 4,
            _ => 0,
        })
        .collect();
    Some((levels, cal["totalContributions"].as_u64()?))
}

/// Spotify's 300 px cover instead of the 640 px one; the panel shows it at about 110 px.
pub fn smaller_cover(url: &str) -> String {
    match url.strip_prefix("https://i.scdn.co/image/ab67616d0000b273") {
        Some(id) => format!("https://i.scdn.co/image/ab67616d00001e02{id}"),
        None => url.to_owned(),
    }
}

/// spotatui.com's `/widget/<username>.json`; `None` when nothing is playing.
pub fn now_playing(json: &str) -> Option<Track> {
    let v: Value = serde_json::from_str(json).ok()?;
    let state = v["state"].as_str()?;
    if state == "offline" {
        return None;
    }
    let text = |key: &str| v[key].as_str().unwrap_or_default().to_owned();
    Some(Track {
        state: state.to_owned(),
        title: v["title"].as_str()?.to_owned(),
        // Older responses only had the combined subtitle.
        artists: v["artists"]
            .as_str()
            .map_or_else(|| text("subtitle"), str::to_owned),
        album: text("album"),
        progress_ms: v["progressMs"].as_u64().unwrap_or(0),
        duration_ms: v["durationMs"].as_u64().unwrap_or(0),
        image_url: v["imageUrl"].as_str().map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn failed_sources_show_question_marks() {
        let p = assemble(&Raw {
            followers: Some(fixture("followers.json")),
            ..Raw::default()
        });
        assert_eq!((p.followers.as_str(), p.views.as_str()), ("51", "?"));
        assert_eq!(
            (p.stats.rank.as_str(), p.steamie.name.as_str()),
            ("?", "steamie")
        );
        assert!(p.now_playing.is_none() && p.discord.is_none() && p.languages.is_empty());
    }

    #[test]
    fn picks_the_300px_spotify_cover() {
        assert_eq!(
            smaller_cover("https://i.scdn.co/image/ab67616d0000b273ce38"),
            "https://i.scdn.co/image/ab67616d00001e02ce38"
        );
        assert_eq!(
            smaller_cover("https://i.ytimg.com/vi/x/hq.jpg"),
            "https://i.ytimg.com/vi/x/hq.jpg"
        );
    }

    #[test]
    fn reads_the_stats_card() {
        let s = stats_card(&fixture("stats.svg")).unwrap();
        assert_eq!(
            [
                s.stars,
                s.commits,
                s.prs,
                s.issues,
                s.contributed_to,
                s.rank
            ],
            ["1.9k", "2.5k", "559", "463", "13", "A+"]
        );
    }

    #[test]
    fn reads_the_top_languages_card() {
        let langs = top_langs_card(&fixture("top-langs.svg")).unwrap();
        let got: Vec<(&str, f64)> = langs.iter().map(|l| (l.name.as_str(), l.percent)).collect();
        assert_eq!(
            got,
            [
                ("TypeScript", 24.29),
                ("C++", 22.69),
                ("Python", 19.98),
                ("Rust", 15.72),
                ("C", 9.57),
                ("C#", 7.76)
            ]
        );
        assert_eq!(langs[0].color, Color::Rgb(0x31, 0x78, 0xC6));
    }

    #[test]
    fn reads_pin_cards_and_drops_the_placeholder_description() {
        let steamie = pin_card(&fixture("pin-steamie.svg")).unwrap();
        assert_eq!(
            (
                steamie.name.as_str(),
                steamie.stars.as_str(),
                steamie.forks.as_str()
            ),
            ("steamie", "12", "2")
        );
        assert_eq!(steamie.description, None);
        assert_eq!(steamie.language.unwrap().0, "Rust");

        let spotatui = pin_card(&fixture("pin-spotatui.svg")).unwrap();
        assert!(
            spotatui
                .description
                .unwrap()
                .starts_with("Open-source music player for the desktop and the terminal.")
        );
    }

    #[test]
    fn reads_badges_and_presence() {
        assert_eq!(komarev_badge(&fixture("komarev.svg")).unwrap(), "2,764");
        assert_eq!(shields(&fixture("followers.json")).unwrap(), "51");
        assert_eq!(shields(&fixture("stars.json")).unwrap(), "1.9k");
        let p = lanyard(&fixture("lanyard.json")).unwrap();
        assert_eq!(
            (p.username.as_str(), p.tag.as_deref(), p.status.as_str()),
            ("largemodgames", Some("GBGS"), "online")
        );
    }

    #[test]
    fn reads_the_contribution_calendar() {
        let (levels, total) = calendar(&fixture("calendar.json")).unwrap();
        assert_eq!(total, 5697);
        assert!(levels.len() > 360 && levels.iter().all(|l| *l <= 4));
    }

    #[test]
    fn reads_now_playing_and_treats_offline_as_nothing() {
        let t = now_playing(
            r#"{"state":"playing","title":"T","artists":"A","album":"B","progressMs":1000,"durationMs":2000,"imageUrl":"https://i.scdn.co/image/x"}"#,
        )
        .unwrap();
        assert_eq!(
            (
                t.state.as_str(),
                t.title.as_str(),
                t.artists.as_str(),
                t.album.as_str()
            ),
            ("playing", "T", "A", "B")
        );
        assert_eq!(t.image_url.as_deref(), Some("https://i.scdn.co/image/x"));
        let old = now_playing(r#"{"state":"paused","title":"T","subtitle":"A · B"}"#).unwrap();
        assert_eq!((old.artists.as_str(), old.album.as_str()), ("A · B", ""));
        assert!(now_playing(r#"{"state":"offline","title":"Not listening right now"}"#).is_none());
    }
}
