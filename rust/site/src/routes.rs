use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::UrlSearchParams;

fn mount_attribute(name: &str) -> Option<String> {
    document().get_element_by_id("planner")?.get_attribute(name)
}

pub fn asset(name: &str) -> String {
    let base = mount_attribute("data-assets").unwrap_or_else(|| "/rust".into());
    format!("{base}/{name}")
}

pub fn planner() -> &'static str {
    if mount_attribute("data-preview").as_deref() == Some("true") {
        "/rust/"
    } else {
        "/"
    }
}

pub fn garden() -> &'static str {
    if planner() == "/rust/" {
        "/rust/garten/"
    } else {
        "/garten/"
    }
}

const SPROUT: &str = r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><path d="M7 20h10"/><path d="M10 20c5.5-2.5.8-6.4 3-10"/><path d="M9.5 9.4c1.1.8 1.8 2.2 2.3 3.7-2 .4-3.5.4-4.8-.3-1.2-.6-2.3-1.9-3-4.2 2.8-.5 4.4 0 5.5.8z"/><path d="M14.1 6a7 7 0 0 0-1.1 4c1.9-.1 3.3-.6 4.3-1.4 1-1 1.6-2.3 1.7-4.6-2.7.1-4 1-4.9 2z"/></svg>"#;
const GRID: &str = r#"<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M3 15h18M9 3v18M15 3v18"/></svg>"#;
const BOOK: &str = r#"<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>"#;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Planner,
    Garden,
}

/// Logo and the segmented page navigation, shared with the static guide pages.
#[component]
pub fn SiteNav(
    catalog: std::sync::Arc<crate::catalog::Catalog>,
    language: &'static str,
    active: Section,
) -> impl IntoView {
    let current = move |section: Section| (section == active).then_some("page");
    let links = [
        (format!("{}?lang={language}", planner()), catalog.text("navPlanner"), SPROUT, current(Section::Planner)),
        (format!("{}?lang={language}", garden()), catalog.text("navGarden"), GRID, current(Section::Garden)),
        (format!("/guides/?lang={language}"), catalog.text("navGuides"), BOOK, None),
    ];
    view! {
        <a class="brand" href=format!("{}?lang={language}", planner()) aria-label="Pflanzengilde.de">
            <span class="brand-mark" aria-hidden="true" inner_html=SPROUT></span>
            <span class="brand-name">"pflanzengilde"<b>".de"</b></span>
        </a>
        <nav class="site-nav">
            {links.into_iter().map(|(href, text, icon, current)| {
                let label = text.clone();
                view! {
                    <a href=href aria-current=current title=label.clone() aria-label=label>
                        <span class="nav-icon" aria-hidden="true" inner_html=icon></span>
                        <span class="nav-text">{text}</span>
                    </a>
                }
            }).collect_view()}
        </nav>
    }
}

/// NO / EN / DE switch. Remembers the choice and reloads the page in that language.
#[component]
pub fn LanguageSelect(language: &'static str) -> impl IntoView {
    let choose = move |value: &'static str| {
        if let Ok(Some(storage)) = window().local_storage() {
            let _ = storage.set_item(crate::LANGUAGE_KEY, value);
        }
        let location = window().location();
        if let Ok(params) = UrlSearchParams::new_with_str(&location.search().unwrap_or_default()) {
            params.set("lang", value);
            let _ = location.set_search(&String::from(params.to_string()));
        }
    };
    let label = match language {
        "nb" => "Språk",
        "de" => "Sprache",
        _ => "Language",
    };
    view! {
        <div class="language-switch" role="group" aria-label=label>
            {[("nb", "NO", "Norsk"), ("en", "EN", "English"), ("de", "DE", "Deutsch")].into_iter().map(|(code, short, name)| view! {
                <button type="button" lang=code aria-label=name aria-pressed=(code == language).to_string()
                    on:click=move |_| if code != language { choose(code) }>{short}</button>
            }).collect_view()}
        </div>
    }
}

pub fn is_garden(path: &str) -> bool {
    let path = path.trim_end_matches('/').trim_end_matches("/index.html");
    matches!(
        path,
        "/garten" | "/garden" | "/rust/garten" | "/rust/garden"
    )
}

pub fn shared_tree(path: &str) -> Option<&str> {
    ["/share/", "/guild/"].iter().find_map(|prefix| {
        let slug = path.strip_prefix(prefix)?.split('/').next()?;
        (!slug.is_empty()
            && slug
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'-' | b'_')))
        .then_some(slug)
    })
}

pub fn clean_guild_query() -> Result<(), JsValue> {
    let location = window().location();
    let params = UrlSearchParams::new_with_str(&location.search()?)?;
    let path = location.pathname()?;
    let shared = shared_tree(&path).is_some();
    if !shared
        && !["g", "tree", "plant", "plants"]
            .iter()
            .any(|key| params.has(key))
    {
        return Ok(());
    }
    for key in [
        "g",
        "tree",
        "plant",
        "plants",
        "soil",
        "zone",
        "hemi",
        "hemisphere",
    ] {
        params.delete(key);
    }
    let query = String::from(params.to_string());
    let query = if query.is_empty() {
        String::new()
    } else {
        format!("?{query}")
    };
    let url = format!(
        "{}{query}{}",
        if shared { planner() } else { &path },
        location.hash()?
    );
    window()
        .history()?
        .replace_state_with_url(&JsValue::NULL, "", Some(&url))
}

pub fn clean_garden_query(params: &UrlSearchParams, key: &str) -> Result<(), JsValue> {
    params.delete(key);
    let location = window().location();
    let current = location.pathname()?;
    let destination = if is_garden(&current) {
        current
    } else {
        garden().into()
    };
    let query = String::from(params.to_string());
    let query = if query.is_empty() {
        String::new()
    } else {
        format!("?{query}")
    };
    let url = format!("{destination}{query}{}", location.hash()?);
    window()
        .history()?
        .replace_state_with_url(&JsValue::NULL, "", Some(&url))
}

pub fn legal() -> (bool, bool) {
    let links = mount_attribute("data-legal").unwrap_or_default();
    (
        links.split_whitespace().any(|p| p == "impressum"),
        links.split_whitespace().any(|p| p == "datenschutz"),
    )
}

#[leptos::prelude::component]
pub fn SiteFooter(
    catalog: std::sync::Arc<crate::catalog::Catalog>,
    language: &'static str,
) -> impl leptos::prelude::IntoView {
    use leptos::prelude::*;
    let (impressum, privacy) = legal();
    view! {
        <footer class="site-footer">
            <p>{catalog.text("footerText")}</p>
            <nav>
                <a href=format!("{}?lang={language}", planner())>{catalog.text("navPlanner")}</a>
                <a href=format!("{}?lang={language}", garden())>{catalog.text("navGarden")}</a>
                <a href=format!("/guides/?lang={language}")>{catalog.text("navGuides")}</a>
                <a href=format!("/credits/?lang={language}")>{catalog.text("photoCreditsTitle")}</a>
                {impressum.then(|| view! { <a href=format!("/impressum/?lang={language}")>{catalog.text("impressumLink")}</a> })}
                {privacy.then(|| view! { <a href=format!("/datenschutz/?lang={language}")>{catalog.text("appPrivacyLink")}</a> })}
            </nav>
        </footer>
    }
}
