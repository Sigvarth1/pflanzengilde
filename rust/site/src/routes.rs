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

#[component]
pub fn LanguageSelect(language: &'static str) -> impl IntoView {
    let change = move |event| {
        let value = event_target_value(&event);
        if let Ok(Some(storage)) = window().local_storage() {
            let _ = storage.set_item(crate::LANGUAGE_KEY, &value);
        }
        let location = window().location();
        if let Ok(params) = UrlSearchParams::new_with_str(&location.search().unwrap_or_default()) {
            params.set("lang", &value);
            let _ = location.set_search(&String::from(params.to_string()));
        }
    };
    let label = match language {
        "nb" => "Språk",
        "de" => "Sprache",
        _ => "Language",
    };
    view! {
        <select aria-label=label prop:value=language on:change=change>
            <option value="nb">"Norsk"</option>
            <option value="en">"English"</option>
            <option value="de">"Deutsch"</option>
        </select>
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
            <a href=format!("/credits/?lang={language}")>{catalog.text("photoCreditsTitle")}</a>
            {impressum.then(|| view! { <a href=format!("/impressum/?lang={language}")>{catalog.text("impressumLink")}</a> })}
            {privacy.then(|| view! { <a href=format!("/datenschutz/?lang={language}")>{catalog.text("appPrivacyLink")}</a> })}
        </footer>
    }
}
