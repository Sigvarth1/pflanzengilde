//! Visual card pickers for soil type and star plant, matching the React design.
use crate::catalog::Catalog;
use crate::guild::{Guild, SOILS};
use leptos::prelude::*;
use std::sync::Arc;

const SOIL_PH_KEYS: [&str; 6] = [
    "siteConditionsLoamPh",
    "siteConditionsClayPh",
    "siteConditionsSandyPh",
    "siteConditionsChalkyPh",
    "siteConditionsAcidicPh",
    "siteConditionsSiltPh",
];

fn soil_image(id: &str) -> String {
    let file = match id {
        "LOAM" => "loam.webp?v=4",
        "CLAY" => "clay.webp",
        "SANDY" => "sandy.webp",
        "CHALKY" => "chalky.webp",
        "ACIDIC" => "acidic.webp",
        _ => "silt.webp",
    };
    format!("/images/soils/soil-{file}")
}

fn metres(value: f64, language: &str) -> String {
    let text = format!("{value:.1}");
    if language == "en" { text } else { text.replace('.', ",") }
}

const CHECK: &str = r#"<svg viewBox="0 0 24 24" width="11" height="11" fill="none" stroke="currentColor" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>"#;
const SUN: &str = r#"<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M6.3 17.7l-1.4 1.4M19.1 4.9l-1.4 1.4"/></svg>"#;
const PARTIAL: &str = r#"<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v2M4.9 4.9l1.4 1.4M20 12h2M19.1 4.9l-1.4 1.4M15.9 12.7a4 4 0 0 0-5.9-4.5"/><path d="M13 22H7a5 5 0 1 1 4.9-6H13a3 3 0 0 1 0 6Z"/></svg>"#;
const SHADE: &str = r#"<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><path d="M17.5 19H9a7 7 0 1 1 6.7-9h1.8a4.5 4.5 0 1 1 0 9Z"/></svg>"#;

/// Six photo cards for the soil type. Each card says whether the chosen star plant likes that soil.
#[component]
pub fn SoilPicker(catalog: Arc<Catalog>, guild: RwSignal<Guild>, on_select: Callback<String>) -> impl IntoView {
    let cards = SOILS
        .into_iter()
        .zip(SOIL_PH_KEYS)
        .map(|((id, label_key), ph_key)| {
            let label = catalog.text(label_key);
            let ph = catalog.text(ph_key);
            let good = catalog.text("soilChipGood");
            let bad = catalog.text("soilChipBad");
            let fit_catalog = catalog.clone();
            let fit = move || {
                guild.with(|state| {
                    let tree = fit_catalog.trees.iter().find(|tree| tree.id == state.tree_id)?;
                    if tree.preferred_soils.iter().any(|soil| soil == id) {
                        Some(true)
                    } else if tree.unsuitable_soils.iter().any(|soil| soil == id) {
                        Some(false)
                    } else {
                        None
                    }
                })
            };
            let selected = move || guild.with(|state| state.soil_type == id);
            view! {
                <button type="button" class="soil-card" aria-pressed=move || selected().to_string()
                    on:click=move |_| on_select.run(id.to_string())>
                    <img src=soil_image(id) alt="" loading="lazy" decoding="async" />
                    <span class="soil-card-top">
                        {move || match fit() {
                            Some(true) => view! { <span class="fit-chip fit-good"><i></i>{good.clone()}</span> }.into_any(),
                            Some(false) => view! { <span class="fit-chip fit-bad"><i></i>{bad.clone()}</span> }.into_any(),
                            None => view! { <span></span> }.into_any(),
                        }}
                        <span class="card-check" hidden=move || !selected() inner_html=CHECK></span>
                    </span>
                    <span class="soil-card-text">
                        <strong>{label.clone()}</strong>
                        <small>{ph.clone()}</small>
                    </span>
                </button>
            }
        })
        .collect_view();
    view! { <div class="soil-cards" role="group" aria-label=catalog.text("soilType")>{cards}</div> }
}

/// Grid of star plant cards with photo, sun need, soil fit and drip radius.
#[component]
pub fn TreePicker(
    catalog: Arc<Catalog>,
    guild: RwSignal<Guild>,
    language: &'static str,
    on_select: Callback<String>,
) -> impl IntoView {
    let drip = catalog.text("dripLine");
    let optimal = catalog.text("soilOptimal");
    let unsuitable = catalog.text("soilUnsuitable");
    let cards = catalog
        .trees
        .iter()
        .filter(|tree| !tree.retired)
        .map(|tree| {
            let id = tree.id.clone();
            let pressed_id = id.clone();
            let click_id = id.clone();
            let preferred = tree.preferred_soils.clone();
            let unsuited = tree.unsuitable_soils.clone();
            let zones = tree.climate_zones.clone();
            let soil_fit = move || {
                guild.with(|state| {
                    if preferred.contains(&state.soil_type) {
                        Some(true)
                    } else if unsuited.contains(&state.soil_type) {
                        Some(false)
                    } else {
                        None
                    }
                })
            };
            let out_of_zone = move || guild.with(|state| !zones.contains(&state.climate_zone));
            let sun = match tree.sun_preference.as_str() {
                "FULL_SUN" => Some(SUN),
                "PARTIAL_SUN" => Some(PARTIAL),
                "FULL_SHADE" => Some(SHADE),
                _ => None,
            };
            let (optimal, unsuitable) = (optimal.clone(), unsuitable.clone());
            view! {
                <button type="button" class="tree-card"
                    aria-pressed=move || guild.with(|state| state.tree_id == pressed_id).to_string()
                    class:out-of-zone=out_of_zone
                    on:click=move |_| on_select.run(click_id.clone())>
                    <span class="tree-card-top">
                        <img src=tree.image_url.clone() alt="" width="36" height="36" loading="lazy" decoding="async"
                            style=format!("background:{}", tree.color) />
                        <span class="tree-card-badges">
                            {sun.map(|icon| view! { <span class="sun-icon" title=tree.sun_label.clone() inner_html=icon></span> })}
                            {move || match soil_fit() {
                                Some(true) => view! { <span class="fit-dot fit-good" title=optimal.clone()></span> }.into_any(),
                                Some(false) => view! { <span class="fit-dot fit-bad" title=unsuitable.clone()></span> }.into_any(),
                                None => ().into_any(),
                            }}
                            {tree.juglone_producer.then(|| view! {
                                <span class="juglone-badge">{if language == "en" { "Juglone" } else { "Juglon" }}</span>
                            })}
                        </span>
                    </span>
                    <strong>{tree.name.clone()}</strong>
                    <i>{tree.botanical_name.clone()}</i>
                    <small>{tree.category_label.clone()}</small>
                    <span class="tree-card-foot">
                        <span>{format!("{}: {} m", drip, metres(tree.mature_radius_m, language))}</span>
                        <b>{tree.bloom_label.clone()}</b>
                    </span>
                </button>
            }
        })
        .collect_view();
    view! { <div class="tree-cards" role="group" aria-label=catalog.text("step1Title")>{cards}</div> }
}
