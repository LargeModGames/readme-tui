use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Tabs, Widget};

pub const WIDTH: u16 = 96;
pub const HEIGHT: u16 = 51;
/// Cover art width in cells; with 6 rows of 18 px it is close to square.
const ART_WIDTH: u16 = 13;

const ACCENT: Color = Color::Rgb(0xF7, 0x93, 0x1E);
const MUTED: Color = Color::DarkGray;
const TRACK: Color = Color::Rgb(0x3E, 0x3D, 0x32);
const HEAT: [Color; 5] = [
    Color::Rgb(0x3E, 0x3D, 0x32),
    Color::Rgb(0x6B, 0x45, 0x1A),
    Color::Rgb(0x9C, 0x5E, 0x1A),
    Color::Rgb(0xCC, 0x7A, 0x1C),
    ACCENT,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    WhatIDo,
    Spotatui,
    Steamie,
    Tech,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::WhatIDo, Tab::Spotatui, Tab::Steamie, Tab::Tech];

    pub fn title(self) -> &'static str {
        match self {
            Tab::WhatIDo => "What I Do",
            Tab::Spotatui => "spotatui",
            Tab::Steamie => "steamie",
            Tab::Tech => "Tech I Use",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Tab::WhatIDo => "what-i-do",
            Tab::Spotatui => "spotatui",
            Tab::Steamie => "steamie",
            Tab::Tech => "tech",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Tab> {
        Tab::ALL.into_iter().find(|t| t.slug() == slug)
    }
}

pub struct Track {
    /// `playing`, `paused` or `live`, as spotatui.com reports it.
    pub state: String,
    pub title: String,
    pub artists: String,
    pub album: String,
    pub progress_ms: u64,
    pub duration_ms: u64,
    pub image_url: Option<String>,
}

pub struct Presence {
    pub username: String,
    pub tag: Option<String>,
    pub status: String,
}

pub struct RepoCard {
    pub name: String,
    pub description: Option<String>,
    pub stars: String,
    pub forks: String,
    pub language: Option<(String, Color)>,
}

pub struct Stats {
    pub stars: String,
    pub commits: String,
    pub prs: String,
    pub issues: String,
    pub contributed_to: String,
    pub rank: String,
}

pub struct Language {
    pub name: String,
    pub percent: f64,
    pub color: Color,
}

pub struct Profile {
    pub now_playing: Option<Track>,
    /// The cover as a `data:` URI, inlined because GitHub's image proxy blocks external images.
    pub cover_art: Option<String>,
    pub discord: Option<Presence>,
    pub views: String,
    pub followers: String,
    pub stars: String,
    pub spotatui: RepoCard,
    pub steamie: RepoCard,
    pub steam_cm_protocol: RepoCard,
    pub stats: Stats,
    pub languages: Vec<Language>,
    /// Contribution levels 0..=4, one per day, oldest first, starting on a Sunday.
    pub contributions: Vec<u8>,
    pub total_contributions: u64,
}

/// Draws the frame and returns it with the cell area reserved for the cover art.
pub fn draw(profile: &Profile, tab: Tab) -> (Buffer, Rect) {
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut buf = Buffer::empty(area);

    let frame = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(Line::from(" LargeModGames ").bold().fg(ACCENT));
    let inner = frame.inner(area);
    frame.render(area, &mut buf);

    let [header_area, tabs, rule, body, strip, footer] = Layout::vertical([
        Constraint::Length(14),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(12),
        Constraint::Length(1),
    ])
    .horizontal_margin(1)
    .areas(inner);

    let art = header(profile, header_area, &mut buf);

    Tabs::new(Tab::ALL.map(|t| t.title()))
        .select(Tab::ALL.iter().position(|t| *t == tab))
        .style(Style::new().fg(MUTED))
        .highlight_style(Style::new().fg(Color::Black).bg(ACCENT).bold())
        .padding(" ", " ")
        .divider(" ")
        .render(tabs, &mut buf);
    Line::from(symbols::line::HORIZONTAL.repeat(rule.width as usize))
        .fg(MUTED)
        .render(rule, &mut buf);

    let body = Rect {
        y: body.y + 1,
        height: body.height.saturating_sub(1),
        ..body
    };
    match tab {
        Tab::WhatIDo => what_i_do(body, &mut buf),
        Tab::Spotatui => spotatui(profile, body, &mut buf),
        Tab::Steamie => steamie(profile, body, &mut buf),
        Tab::Tech => tech(body, &mut buf),
    }

    github_strip(profile, strip, &mut buf);

    Line::from(vec![
        Span::from("▼ ").fg(ACCENT),
        Span::from("pick a tab below this window to switch it for everyone"),
    ])
    .fg(MUTED)
    .render(footer, &mut buf);

    (buf, art)
}

fn header(p: &Profile, area: Rect, buf: &mut Buffer) -> Rect {
    let mut lines = vec![
        Line::default(),
        Line::from(
            "Open-source developer and maintainer, building terminal-native software in Rust",
        )
        .bold()
        .fg(ACCENT),
    ];
    lines.extend(wrap(
        &[
            ("Maintainer of ", Style::new()),
            ("spotatui", Style::new().bold().fg(ACCENT)),
            (
                ". I work on media playback, network protocols, Linux tooling, and cross-platform applications.",
                Style::new(),
            ),
        ],
        area.width as usize,
        "",
    ));
    Paragraph::new(lines).render(area, buf);

    let [player, side] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(26)])
        .spacing(2)
        .areas(Rect {
            y: area.y + 5,
            height: 8,
            ..area
        });
    side_column(p, side, buf);
    now_playing_panel(p, player, buf)
}

/// Draws the player panel and returns the cell area reserved for the cover art.
fn now_playing_panel(p: &Profile, area: Rect, buf: &mut Buffer) -> Rect {
    let block = panel(" ♪ Now playing on spotatui ").title_bottom(
        Line::from(vec![
            Span::from(" ↑ live from "),
            Span::from("spotatui").bold(),
            Span::from(", the terminal music player I maintain "),
        ])
        .fg(MUTED),
    );
    let inner = block.inner(area);
    block.render(area, buf);
    let [art, _, text] = Layout::horizontal([
        Constraint::Length(ART_WIDTH),
        Constraint::Length(2),
        Constraint::Fill(1),
    ])
    .areas(inner);

    // Placeholder; the SVG renderer draws the cover image over it when there is one.
    for y in art.top()..art.bottom() {
        for x in art.left()..art.right() {
            buf[(x, y)].set_symbol(" ").set_bg(TRACK);
        }
    }
    Line::from("♪").fg(MUTED).centered().render(
        Rect {
            y: art.y + art.height / 2,
            height: 1,
            ..art
        },
        buf,
    );

    let width = text.width as usize;
    let lines = match &p.now_playing {
        Some(t) => vec![
            Line::default(),
            Line::from(truncate(&t.title, width)).bold(),
            Line::from(truncate(&t.artists, width)),
            Line::from(truncate(&t.album, width)).fg(MUTED),
            Line::default(),
            progress(t, width),
        ],
        None => vec![
            Line::default(),
            Line::from("Not listening right now").fg(MUTED),
            Line::from("spotatui.com").fg(MUTED),
        ],
    };
    Paragraph::new(lines).render(text, buf);
    art
}

/// `▶ 1:29 ━━━━━━──── 3:56`, with ⏸ when paused and a LIVE tag for streams.
fn progress(t: &Track, width: usize) -> Line<'static> {
    let mmss = |ms: u64| format!("{}:{:02}", ms / 60_000, ms / 1000 % 60);
    let (icon, color) = match t.state.as_str() {
        "paused" => ("⏸", MUTED),
        "live" => {
            return Line::from(vec![
                Span::from(" LIVE ").fg(Color::Black).bg(Color::Red).bold(),
                Span::from(format!("  {}", mmss(t.progress_ms))).fg(MUTED),
            ]);
        }
        _ => ("▶", ACCENT),
    };
    let (elapsed, total) = (mmss(t.progress_ms), mmss(t.duration_ms));
    let bar = width.saturating_sub(elapsed.len() + total.len() + 5);
    let filled = if t.duration_ms > 0 {
        ((t.progress_ms as f64 / t.duration_ms as f64).clamp(0.0, 1.0) * bar as f64).round()
            as usize
    } else {
        0
    };
    Line::from(vec![
        Span::from(format!("{icon} ")).fg(color),
        Span::from(format!("{elapsed} ")).fg(MUTED),
        Span::from(symbols::line::THICK_HORIZONTAL.repeat(filled)).fg(color),
        Span::from(symbols::line::HORIZONTAL.repeat(bar - filled)).fg(TRACK),
        Span::from(format!(" {total}")).fg(MUTED),
    ])
}

fn side_column(p: &Profile, area: Rect, buf: &mut Buffer) {
    let [discord, _, badges] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Length(3),
    ])
    .areas(area);

    let block = panel(" Discord ");
    let inner = block.inner(discord);
    block.render(discord, buf);
    let lines = match &p.discord {
        Some(d) => {
            let (dot, label) = match d.status.as_str() {
                "online" => (Color::Green, "online"),
                "idle" => (Color::Yellow, "idle"),
                "dnd" => (Color::Red, "do not disturb"),
                _ => (MUTED, "offline"),
            };
            let mut name = vec![
                Span::from(" ● ").fg(dot),
                Span::from(d.username.as_str()).bold(),
            ];
            if let Some(tag) = &d.tag {
                name.push(Span::from(" "));
                name.push(
                    Span::from(format!(" {tag} "))
                        .fg(Color::Black)
                        .bg(Color::Gray),
                );
            }
            vec![
                Line::from(name),
                Line::from(format!("   {label}")).fg(MUTED),
            ]
        }
        None => vec![Line::from(" ● offline").fg(MUTED)],
    };
    Paragraph::new(lines).render(inner, buf);

    Paragraph::new(vec![
        Line::from(badge("PROFILE VIEWS", &p.views)),
        Line::from(badge("FOLLOWERS", &p.followers)),
        Line::from(badge("STARS", &p.stars)),
    ])
    .right_aligned()
    .render(badges, buf);
}

fn what_i_do(area: Rect, buf: &mut Buffer) {
    let width = area.width as usize;
    let mut lines = Vec::new();
    for (head, rest) in [
        (
            "Maintain open-source projects that people actually run.",
            " Releases, packaging, issue triage, and the long tail of cross-platform bugs.",
        ),
        (
            "Write Rust",
            " for my open-source work. C, C++, and Python when they suit the problem better.",
        ),
        (
            "Build with teams.",
            " TypeScript and Python at mAIdribusiness, and C++ game development at Greybeard Game Studios.",
        ),
        (
            "Build for the terminal.",
            " Fast, keyboard-driven tools that stay out of your way.",
        ),
        (
            "Enjoy the awkward parts.",
            " Audio pipelines, undocumented network protocols, Linux internals, and bugs that only reproduce on someone else's machine.",
        ),
    ] {
        let mut bullet = wrap(
            &[(head, Style::new().bold()), (rest, Style::new())],
            width - 2,
            "  ",
        );
        bullet[0].spans.insert(0, Span::from("› ").fg(ACCENT));
        lines.extend(bullet);
        lines.push(Line::default());
    }
    Paragraph::new(lines).render(area, buf);
}

fn spotatui(p: &Profile, area: Rect, buf: &mut Buffer) {
    let width = area.width as usize;
    let mut lines = vec![Line::from(vec![
        Span::from("spotatui").bold().fg(ACCENT),
        Span::from("  flagship   ").fg(MUTED),
    ])];
    let mut card_lines = card(&p.spotatui, width).into_iter();
    if let Some(stats) = card_lines.next() {
        lines[0].spans.extend(stats.spans);
    }
    lines.extend(card_lines);
    lines.push(Line::default());
    lines.extend(wrap(
        &[(
            "A fast music player for the terminal, with a desktop app in preview, written in Rust on Ratatui. \
             Spotify streams through its own built-in client, with no official app, no developer key, and no \
             spotifyd in the loop. Spotify is also just one of seven sources: local files, Subsonic/Navidrome, \
             internet radio, YouTube, Qobuz, and Apple Music on macOS all work on their own.",
            Style::new(),
        )],
        width,
        "",
    ));
    lines.push(Line::default());
    lines.extend(wrap(
        &[(
            "Synced lyrics, a real-time audio visualizer, a queue that mixes sources, playlist sync across \
             services, Lua plugins, an AI DJ (or your coding agent as the DJ over MCP), a local listening history \
             with a shareable recap, and a scriptable CLI. Runs on Linux, macOS, and Windows.",
            Style::new(),
        )],
        width,
        "",
    ));
    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::from("$ ").fg(MUTED),
        Span::from("curl -fsSL https://spotatui.com/install.sh | bash").fg(Color::Green),
        Span::from("  or  ").fg(MUTED),
        Span::from("winget install spotatui").fg(Color::Green),
    ]));
    lines.push(Line::from(links(&[
        "spotatui.com",
        "Discord",
        "crates.io",
        "source",
    ])));
    Paragraph::new(lines).render(area, buf);
}

fn steamie(p: &Profile, area: Rect, buf: &mut Buffer) {
    let width = area.width as usize;
    let mut lines = Vec::new();
    for (repo, text) in [
        (
            &p.steamie,
            "A terminal-native Steam client. Sign in with a QR code, browse your library, chat with friends in \
             real time, check playtime and achievements, and launch games without a Steam window in the way. \
             It speaks Steam's own connection-manager protocol, so there is no Web API key and no scraping.",
        ),
        (
            &p.steam_cm_protocol,
            "The crate underneath steamie, and usable on its own: a Rust implementation of Steam's \
             connection-manager protocol over WebSocket. Authentication (QR, password, Steam Guard, refresh \
             tokens), friends and presence, live chat, library and achievement data. Async, on Tokio.",
        ),
    ] {
        lines.push(Line::from(vec![
            Span::from(repo.name.as_str()).bold().fg(ACCENT),
            Span::from("   "),
        ]));
        if let Some(stats) = card(repo, width).into_iter().next() {
            lines.last_mut().unwrap().spans.extend(stats.spans);
        }
        lines.extend(wrap(&[(text, Style::new())], width, ""));
        lines.push(Line::default());
    }
    Paragraph::new(lines).render(area, buf);
}

fn tech(area: Rect, buf: &mut Buffer) {
    let width = area.width as usize;
    let mut lines: Vec<Line> = [
        "█▀▀▄ █  █ ▄▀▀▀ ▀▀█▀▀",
        "█▄▄▀ █  █  ▀▀▄   █  ",
        "█  █ ▀▄▄▀ ▄▄▄▀   █  ",
    ]
    .into_iter()
    .map(|l| Line::from(l).fg(Color::Rgb(0xDE, 0xA5, 0x84)))
    .collect();
    lines.push(Line::from(vec![
        Span::from("Rust").bold(),
        Span::from(" is where most of my open-source time goes"),
    ]));
    lines.push(Line::default());
    lines.push(Line::from("Languages").fg(MUTED));
    lines.extend(dots(
        &[
            ("C", 0x555555),
            ("C++", 0xF34B7D),
            ("C#", 0x178600),
            ("TypeScript", 0x3178C6),
            ("Kotlin", 0xA97BFF),
            ("Lua", 0x000080),
            ("Python", 0x3572A5),
            ("Haskell", 0x5E5086),
            ("Erlang", 0xB83998),
            ("Bash", 0x89E051),
            ("PowerShell", 0x012456),
        ],
        width,
    ));
    lines.push(Line::default());
    lines.push(Line::from("Tools").fg(MUTED));
    lines.extend(dots(
        &[
            ("React", 0x61DAFB),
            ("Next.js", 0xF8F8F2),
            ("SQLite", 0x0F80CC),
            ("Linux", 0xFCC624),
            ("Arch Linux", 0x1793D1),
            ("Neovim", 0x57A143),
            ("Git", 0xF05032),
            ("GitHub Actions", 0x2088FF),
            ("Docker", 0x2496ED),
            ("Cloudflare", 0xF38020),
        ],
        width,
    ));
    Paragraph::new(lines).render(area, buf);
}

fn github_strip(p: &Profile, area: Rect, buf: &mut Buffer) {
    let [stats_area, right] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(55)])
        .spacing(1)
        .areas(area);
    let [langs_area, heat_area] =
        Layout::vertical([Constraint::Length(6), Constraint::Length(6)]).areas(right);

    let stats_block = panel(" GitHub stats ");
    let stats_inner = stats_block.inner(stats_area);
    stats_block.render(stats_area, buf);
    let s = &p.stats;
    let mut lines: Vec<Line> = [
        ("Total Stars Earned", &s.stars),
        ("Total Commits (last year)", &s.commits),
        ("Total PRs", &s.prs),
        ("Total Issues", &s.issues),
        ("Contributed to (last year)", &s.contributed_to),
    ]
    .into_iter()
    .map(|(label, value)| {
        let value = value.clone();
        let gap = (stats_inner.width as usize).saturating_sub(label.len() + value.len() + 2);
        Line::from(vec![
            Span::from(format!(" {label}")),
            Span::from(" ".repeat(gap)),
            Span::from(value).bold().fg(ACCENT),
        ])
    })
    .collect();
    lines.insert(0, Line::default());
    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::from(" Rank  ").fg(MUTED),
        Span::from(format!(" {} ", s.rank))
            .bold()
            .fg(Color::Black)
            .bg(ACCENT),
    ]));
    Paragraph::new(lines).render(stats_inner, buf);

    let langs_block = panel(" Most Used Languages ");
    let langs_inner = langs_block.inner(langs_area);
    langs_block.render(langs_area, buf);
    language_bar(&p.languages, langs_inner, buf);

    let heat_block = panel(format!(
        " {} contributions in the last year ",
        if p.contributions.is_empty() {
            "?".to_owned()
        } else {
            grouped(p.total_contributions)
        }
    ));
    let heat_inner = heat_block.inner(heat_area);
    heat_block.render(heat_area, buf);
    contribution_grid(&p.contributions, heat_inner, buf);
}

/// One segmented bar sized by percentage, then a two-column legend.
fn language_bar(langs: &[Language], area: Rect, buf: &mut Buffer) {
    if langs.is_empty() {
        Line::from(" ?").render(area, buf);
        return;
    }
    let width = area.width.saturating_sub(2) as f64;
    let total: f64 = langs.iter().map(|l| l.percent).sum();
    let mut x: f64 = 0.0;
    let mut spans = vec![Span::from(" ")];
    for lang in langs {
        let start = x.round() as usize;
        x += lang.percent / total * width;
        let len = x.round() as usize - start;
        spans.push(Span::from("▀".repeat(len)).fg(lang.color));
    }
    Line::from(spans).render(area, buf);

    let half = langs.len().div_ceil(2);
    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(Rect {
            y: area.y + 1,
            height: area.height.saturating_sub(1),
            ..area
        });
    for (column, chunk) in [left, right].into_iter().zip(langs.chunks(half)) {
        let lines: Vec<Line> = chunk
            .iter()
            .map(|l| {
                Line::from(vec![
                    Span::from(" ● ").fg(l.color),
                    Span::from(format!("{} {:.2}%", l.name, l.percent)),
                ])
            })
            .collect();
        Paragraph::new(lines).render(column, buf);
    }
}

/// Two days per cell: the upper half block takes one day's colour as foreground, the next as background.
fn contribution_grid(levels: &[u8], area: Rect, buf: &mut Buffer) {
    let weeks = levels.len().div_ceil(7).min(area.width as usize);
    let skip = levels.len().div_ceil(7) - weeks;
    let x0 = area.x + (area.width - weeks as u16) / 2;
    for week in 0..weeks {
        for pair in 0..4u16 {
            let day = |d: usize| {
                levels
                    .get((skip + week) * 7 + d)
                    .map(|l| HEAT[(*l).min(4) as usize])
            };
            let (top, bottom) = (
                day(pair as usize * 2),
                if pair < 3 {
                    day(pair as usize * 2 + 1)
                } else {
                    None
                },
            );
            let Some(cell) = buf.cell_mut((x0 + week as u16, area.y + pair)) else {
                continue;
            };
            match (top, bottom) {
                (Some(top), bottom) => {
                    cell.set_symbol("▀")
                        .set_fg(top)
                        .set_bg(bottom.unwrap_or(Color::Reset));
                }
                (None, Some(bottom)) => {
                    cell.set_symbol("▄").set_fg(bottom);
                }
                (None, None) => {}
            }
        }
    }
}

fn panel(title: impl Into<String>) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(MUTED))
        .title(Line::from(title.into()).fg(ACCENT))
}

fn card(repo: &RepoCard, width: usize) -> Vec<Line<'_>> {
    let mut spans = vec![
        Span::from(format!("★ {}", repo.stars)).fg(Color::Yellow),
        Span::from("   "),
        Span::from(format!("⑂ {}", repo.forks)).fg(MUTED),
    ];
    if let Some((name, color)) = &repo.language {
        spans.push(Span::from("   ● ").fg(*color));
        spans.push(Span::from(name.clone()).fg(MUTED));
    }
    let mut lines = vec![Line::from(spans)];
    if let Some(d) = &repo.description {
        lines.extend(wrap(&[(d, Style::new().fg(MUTED))], width, ""));
    }
    lines
}

fn links(names: &[&'static str]) -> Span<'static> {
    Span::from(names.join("  ·  ")).fg(Color::Cyan)
}

fn dots(items: &[(&'static str, u32)], width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut used = 0;
    for (name, rgb) in items {
        let len = name.chars().count() + 5;
        if used + len > width && !spans.is_empty() {
            lines.push(Line::from(std::mem::take(&mut spans)));
            used = 0;
        }
        let [_, r, g, b] = rgb.to_be_bytes();
        spans.push(Span::from("● ").fg(Color::Rgb(r, g, b)));
        spans.push(Span::from(*name));
        spans.push(Span::from("   "));
        used += len;
    }
    lines.push(Line::from(spans));
    lines
}

fn badge(label: &'static str, value: &str) -> Vec<Span<'static>> {
    vec![
        Span::from(format!(" {label} "))
            .fg(Color::White)
            .bg(Color::Rgb(0x55, 0x55, 0x55))
            .bold(),
        Span::from(format!(" {value} "))
            .fg(Color::Black)
            .bg(ACCENT)
            .bold(),
    ]
}

/// Greedy word wrap over styled segments; continuation lines start with `indent`.
fn wrap<'a>(segments: &[(&'a str, Style)], width: usize, indent: &'static str) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut used = 0;
    for (text, style) in segments {
        for word in text.split_inclusive(' ') {
            let len = word.trim_end().chars().count();
            if used > 0 && used + len > width {
                if let Some(last) = spans.last_mut() {
                    last.content = last.content.trim_end().to_string().into();
                }
                lines.push(Line::from(std::mem::take(&mut spans)));
                spans.push(Span::from(indent));
                used = indent.len();
            }
            spans.push(Span::styled(word, *style));
            used += word.chars().count();
        }
    }
    lines.push(Line::from(spans));
    lines
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn grouped(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_playing_shows_track_and_progress() {
        let profile = crate::sources::assemble(&crate::sources::Raw {
            now_playing: Some(
                r#"{"state":"playing","title":"Used To Love (with Dean Lewis)","artists":"Martin Garrix, Dean Lewis","album":"Used To Love (with Dean Lewis)","subtitle":"x","progressMs":89770,"durationMs":236764,"imageUrl":null}"#.into(),
            ),
            ..Default::default()
        });
        let (buf, art) = draw(&profile, Tab::Spotatui);
        let text: String = (0..HEIGHT)
            .map(|y| (0..WIDTH).map(|x| buf[(x, y)].symbol()).collect::<String>() + "\n")
            .collect();
        assert!(text.contains("Martin Garrix, Dean Lewis"), "{text}");
        assert!(text.contains("▶ 1:29 ━"), "{text}");
        assert!(text.contains("─ 3:56"), "{text}");
        assert_eq!((art.width, art.height), (ART_WIDTH, 6));
    }

    #[test]
    fn renders_with_every_source_missing() {
        let profile = crate::sources::assemble(&crate::sources::Raw::default());
        for tab in Tab::ALL {
            draw(&profile, tab);
        }
    }

    #[test]
    fn truncates_with_ellipsis() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("abc", 4), "abc");
    }

    #[test]
    fn groups_thousands() {
        assert_eq!(grouped(1864), "1,864");
        assert_eq!(grouped(51), "51");
        assert_eq!(grouped(1234567), "1,234,567");
    }

    #[test]
    fn wraps_with_hanging_indent() {
        let lines = wrap(&[("aaa bbb ccc", Style::new())], 7, "  ");
        let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        assert_eq!(text, ["aaa bbb", "  ccc"]);
    }

    #[test]
    fn tab_slugs_round_trip() {
        for tab in Tab::ALL {
            assert_eq!(Tab::from_slug(tab.slug()), Some(tab));
        }
    }
}
