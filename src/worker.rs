//! Cloudflare Worker: serves the TUI as a live SVG and records tab clicks.

use futures::join;
use worker::*;

use crate::sources::{self, Raw, assemble};
use crate::ui::{Tab, draw};

const STATS: &str = "https://github-readme-stats-zeta-eight-88.vercel.app";
const PROFILE: &str = "https://github.com/LargeModGames";
/// How long a slow source (cards, badges, calendar) is reused before it is refetched.
const SLOW_TTL_MS: u64 = 30 * 60 * 1000;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .get_async("/tui.svg", |_, ctx| async move { tui(&ctx.env).await })
        .get_async("/tab/:slug", |_, ctx| async move {
            if let Some(tab) = ctx.param("slug").and_then(|s| Tab::from_slug(s)) {
                // A failed write (e.g. the daily KV quota) still sends the visitor back.
                if let Ok(put) = ctx.env.kv("STATE")?.put("tab", tab.slug()) {
                    let _ = put.execute().await;
                }
            }
            Response::redirect(Url::parse(PROFILE)?)
        })
        .run(req, env)
        .await
}

async fn tui(env: &Env) -> Result<Response> {
    let kv = env.kv("STATE")?;
    let tab = kv
        .get("tab")
        .text()
        .await
        .ok()
        .flatten()
        .and_then(|s| Tab::from_slug(&s))
        .unwrap_or(Tab::Spotatui);
    let token = env.secret("GITHUB_TOKEN").ok().map(|s| s.to_string());
    let card = |query: &str| format!("{STATS}/api{query}&theme=monokai&hide_border=true");
    let pin = |repo: &str| card(&format!("/pin/?username=LargeModGames&repo={repo}"));

    let (
        stats,
        top_langs,
        pin_spotatui,
        pin_steamie,
        pin_steam_cm_protocol,
        followers,
        stars,
        calendar,
        views,
        lanyard,
        now_playing,
    ) =
        join!(
        cached(
            &kv,
            "stats",
            get(card("?username=LargeModGames&show_icons=true")),
            |s| sources::stats_card(s).is_some()
        ),
        cached(
            &kv,
            "top-langs",
            get(card(
                "/top-langs/?username=LargeModGames&orgs=mAIdribusiness,Greybeard-Game-Studios\
                 &size_weight=0.5&count_weight=0.5&hide=makefile&layout=compact"
            )),
            |s| sources::top_langs_card(s).is_some()
        ),
        cached(&kv, "pin-spotatui", get(pin("spotatui")), is_pin),
        cached(&kv, "pin-steamie", get(pin("steamie")), is_pin),
        cached(
            &kv,
            "pin-steam-cm-protocol",
            get(pin("steam-cm-protocol")),
            is_pin
        ),
        cached(
            &kv,
            "followers",
            get("https://img.shields.io/github/followers/LargeModGames.json".into()),
            is_shields
        ),
        cached(
            &kv,
            "stars",
            get("https://img.shields.io/github/stars/LargeModGames.json".into()),
            is_shields
        ),
        cached(&kv, "calendar", contribution_calendar(token.as_deref()), |s| {
            sources::calendar(s).is_some()
        }),
        // Fetched on every render so each view still counts, as the old badge did.
        get(
            "https://komarev.com/ghpvc/?username=LargeModGames&style=for-the-badge&color=orange"
                .into()
        ),
        get("https://api.lanyard.rest/v1/users/358199352717475840".into()),
        get("https://spotatui.com/widget/largemodgames.json".into()),
    );

    let profile = assemble(&Raw {
        stats,
        top_langs,
        pin_spotatui,
        pin_steamie,
        pin_steam_cm_protocol,
        followers,
        stars,
        calendar,
        views,
        lanyard,
        now_playing,
    });
    let headers = Headers::new();
    headers.set("Content-Type", "image/svg+xml; charset=utf-8")?;
    // GitHub's image proxy honours this, so every page view fetches a fresh frame.
    headers.set(
        "Cache-Control",
        "no-cache, no-store, must-revalidate, max-age=0",
    )?;
    Ok(Response::ok(crate::svg::render(&draw(&profile, tab)))?.with_headers(headers))
}

fn is_pin(s: &str) -> bool {
    sources::pin_card(s).is_some()
}

fn is_shields(s: &str) -> bool {
    sources::shields(s).is_some()
}

/// Reuses the body stored under `key` while it is fresh. Only bodies `valid` accepts are stored,
/// and a failed refresh keeps serving the stale one.
async fn cached(
    kv: &KvStore,
    key: &str,
    fetch: impl Future<Output = Option<String>>,
    valid: fn(&str) -> bool,
) -> Option<String> {
    let now = Date::now().as_millis();
    let stored = kv.get(key).text().await.ok().flatten();
    let (stamp, body) = stored
        .as_deref()
        .and_then(|s| s.split_once('\n'))
        .and_then(|(t, b)| Some((t.parse::<u64>().ok()?, b.to_owned())))
        .unzip();
    if stamp.is_some_and(|t| now.saturating_sub(t) < SLOW_TTL_MS) {
        return body;
    }
    match fetch.await.filter(|b| valid(b)) {
        Some(fresh) => {
            if let Ok(put) = kv.put(key, format!("{now}\n{fresh}")) {
                let _ = put.execute().await;
            }
            Some(fresh)
        }
        None => body,
    }
}

async fn get(url: String) -> Option<String> {
    let headers = Headers::new();
    headers.set("User-Agent", "readme-tui").ok()?;
    let mut init = RequestInit::new();
    init.with_headers(headers);
    let mut res = Fetch::Request(Request::new_with_init(&url, &init).ok()?)
        .send()
        .await
        .ok()?;
    if res.status_code() != 200 {
        return None;
    }
    res.text().await.ok()
}

async fn contribution_calendar(token: Option<&str>) -> Option<String> {
    let token = token?;
    let body = serde_json::json!({
        "query": "{ user(login: \"LargeModGames\") { contributionsCollection { contributionCalendar \
                  { totalContributions weeks { contributionDays { contributionLevel } } } } } }"
    });
    let headers = Headers::new();
    headers.set("User-Agent", "readme-tui").ok()?;
    headers
        .set("Authorization", &format!("bearer {token}"))
        .ok()?;
    headers.set("Content-Type", "application/json").ok()?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body.to_string().into()));
    let mut res =
        Fetch::Request(Request::new_with_init("https://api.github.com/graphql", &init).ok()?)
            .send()
            .await
            .ok()?;
    res.text().await.ok()
}
