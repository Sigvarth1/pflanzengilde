use crate::{catalog::Catalog, guild::CLIMATES};
use leptos::{html, prelude::*};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use wasm_bindgen_futures::spawn_local;

type Selection = (String, bool);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Region {
    id: String,
    zone: String,
    hemisphere: String,
    name: String,
    geo_areas: String,
    lat_range: String,
    button: String,
    usda: String,
    color: String,
    fill_color: String,
    beacon_x: f64,
    beacon_y: f64,
    polygon_path: String,
}

impl Region {
    fn selection(&self) -> Selection {
        (self.zone.clone(), self.hemisphere == "SOUTHERN")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Boundary {
    d: String,
    stroke: String,
    stroke_width: f64,
    dash_array: String,
    opacity: f64,
}

#[derive(Deserialize)]
struct MapData {
    land: String,
    boundaries: Vec<Boundary>,
    regions: Vec<Region>,
    labels: HashMap<String, String>,
}

impl MapData {
    fn text(&self, key: &str) -> String {
        self.labels.get(key).cloned().unwrap_or_default()
    }
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Idle,
    Loading,
    Ready(Arc<MapData>),
    Failed,
}

#[component]
pub fn ClimateMap(
    catalog: Arc<Catalog>,
    language: &'static str,
    selection: Signal<Selection>,
    on_select: Callback<Selection>,
) -> impl IntoView {
    let dialog = NodeRef::<html::Dialog>::new();
    let state = RwSignal::new(State::Idle);
    let hovered = RwSignal::new(None);
    let load = move || {
        if !state.with_untracked(|state| matches!(state, State::Idle | State::Failed)) {
            return;
        }
        state.set(State::Loading);
        spawn_local(async move {
            match crate::fetch_json::<MapData>(&format!("climate-map-{language}.json")).await {
                Ok(data) if !data.regions.is_empty() => state.set(State::Ready(Arc::new(data))),
                _ => state.set(State::Failed),
            }
        });
    };
    let show = move |_| {
        if let Some(dialog) = dialog.get_untracked() {
            if dialog.show_modal().is_ok() {
                load();
            }
        }
    };
    let title = catalog.text("climateZoneModalTitle");
    let loading = catalog.text("loadingFallback");
    let error = match language {
        "de" => "Die Karte konnte nicht geladen werden.",
        "en" => "Could not load the map.",
        _ => "Kunne ikke laste kartet.",
    };
    let retry = match language {
        "de" => "Erneut versuchen",
        "en" => "Try again",
        _ => "Prøv igjen",
    };
    let modal_catalog = catalog.clone();

    view! {
        <button type="button" title=catalog.text("siteConditionsOpenWorldMap")
            aria-haspopup="dialog" on:click=show>
            {catalog.text("siteConditionsWorldMap")}
        </button>
        <dialog class="climate-dialog" node_ref=dialog aria-labelledby="climate-map-title"
            on:close=move |_| hovered.set(None)
            on:click=move |ev| {
                if let Some(node) = dialog.get_untracked() {
                    if ev.target().is_some_and(|target| web_sys::EventTarget::from(node.clone()) == target) {
                        node.close();
                    }
                }
            }>
            <div class="climate-dialog-inner">
                <header class="climate-dialog-header">
                    <h2 id="climate-map-title">{title}</h2>
                    <form method="dialog">
                        <button type="submit" aria-label=catalog.text("closeBtn") autofocus>"×"</button>
                    </form>
                </header>
                {move || match state.get() {
                    State::Idle | State::Loading => view! { <p role="status">{loading.clone()}</p> }.into_any(),
                    State::Failed => view! {
                        <p role="status">{error}</p>
                        <button type="button" on:click=move |_| load()>{retry}</button>
                    }.into_any(),
                    State::Ready(data) => view! {
                        <MapContents catalog=modal_catalog.clone() data selection hovered on_select />
                    }.into_any(),
                }}
            </div>
        </dialog>
    }
}

#[component]
fn MapContents(
    catalog: Arc<Catalog>,
    data: Arc<MapData>,
    selection: Signal<Selection>,
    hovered: RwSignal<Option<usize>>,
    on_select: Callback<Selection>,
) -> impl IntoView {
    let find_region = data.clone();
    let active = Memo::new(move |_| {
        selection.with(|(zone, southern)| {
            find_region
                .regions
                .iter()
                .position(|region| {
                    &region.zone == zone && (region.hemisphere == "SOUTHERN") == *southern
                })
                .unwrap_or(0)
        })
    });
    let banner_data = data.clone();
    let active_data = data.clone();
    let zone_cards = CLIMATES
        .into_iter()
        .map(|(zone, label)| {
            let trees = catalog.trees.iter().filter(|plant| plant.climate_zones.iter().any(|item| item == zone)).count();
            let companions = catalog.plants.iter().filter(|plant| {
                !plant.retired && plant.climate_zones.iter().any(|item| item == zone)
            }).count();
            let summary = data.text("climateModalMatchingSpecies").replace("{count}", &(trees + companions).to_string());
            let counts = format!("{trees} {} · {companions} {}", data.text("climateModalAnchors"), data.text("climateModalCompanions"));
            let options = data.regions.iter().enumerate().filter(|(_, region)| region.zone == zone).map(|(i, region)| {
                let location = region.selection();
                view! {
                    <button type="button" aria-pressed=move || (active.get() == i).to_string()
                        on:focus=move |_| hovered.set(Some(i)) on:blur=move |_| hovered.set(None)
                        on:click=move |_| on_select.run(location.clone())>
                        {region.button.clone()}
                    </button>
                }
            }).collect_view();
            view! {
                <section class="climate-zone-card" class:climate-zone-selected=move || selection.with(|(name, _)| name == zone)>
                    <h3>{catalog.text(label)}</h3>
                    <small>{data.text(&format!("{label}Sub"))}</small>
                    <p>{data.text(&format!("{label}Desc"))}</p>
                    <div class="climate-region-buttons" role="group" aria-label=data.text("climateModalSelectRegion")>{options}</div>
                    <small>{summary} " · " {counts}</small>
                </section>
            }
        }).collect_view();

    view! {
        <p>{data.text("climateZoneModalSubtitle")}</p>
        <div class="climate-region-info">
            {move || {
                let region = &banner_data.regions[hovered.get().unwrap_or_else(|| active.get())];
                let direction = if region.hemisphere == "SOUTHERN" { "sunPathSouthNote" } else { "sunPathNorthNote" };
                view! {
                    <p>
                        <strong>{region.name.clone()}</strong> " · "
                        <span>{region.usda.clone()}</span> " · " {region.lat_range.clone()}
                    </p>
                    <p><strong>{banner_data.text("climateModalKeyRegions")}</strong>{region.geo_areas.clone()}</p>
                    <p>{banner_data.text(direction)}</p>
                }
            }}
        </div>
        <figure class="climate-world">
            <figcaption>
                <strong>{data.text("worldMapTitle")}</strong>
                <span>{data.text("mapRegionHoverHint")}</span>
            </figcaption>
            <WorldMap data=data.clone() active hovered on_select />
        </figure>
        <div class="climate-zones">{zone_cards}</div>
        <footer class="climate-dialog-footer">
            <p role="status">{move || {
                let region = &active_data.regions[active.get()];
                let hemisphere = if region.hemisphere == "SOUTHERN" { "climateModalSouthernHemisphere" }
                    else { "climateModalNorthernHemisphere" };
                active_data.text("climateModalActive")
                    .replace("{region}", &region.name)
                    .replace("{hemisphere}", &active_data.text(hemisphere))
            }}</p>
            <form method="dialog">
                <button type="submit">{data.text("climateModalApplyClose")}</button>
            </form>
        </footer>
    }
}

#[component]
fn WorldMap(
    data: Arc<MapData>,
    active: Memo<usize>,
    hovered: RwSignal<Option<usize>>,
    on_select: Callback<Selection>,
) -> impl IntoView {
    let boundaries = data
        .boundaries
        .iter()
        .map(|line| {
            view! {
                <path d=line.d.clone() stroke=line.stroke.clone() stroke-width=line.stroke_width
                    fill="none" stroke-dasharray=line.dash_array.clone() opacity=line.opacity />
            }
        })
        .collect_view();
    let regions = data.regions.iter().enumerate().map(|(i, region)| view! {
        <path d=region.polygon_path.clone() fill=region.fill_color.clone()
            fill-opacity=move || {
                if hovered.get() == Some(i) { 0.95 } else if active.get() == i { 0.8 } else { 0.32 }
            } />
    }).collect_view();
    let hit_areas = data.regions.iter().enumerate().map(|(i, region)| {
        let location = region.selection();
        view! {
            <path data-climate-region=region.id.clone() d=region.polygon_path.clone() fill="transparent"
                on:pointerenter=move |_| hovered.set(Some(i)) on:pointerleave=move |_| hovered.set(None)
                on:click=move |_| on_select.run(location.clone()) />
        }
    }).collect_view();
    let labels = [
        (525, 122, "Europe"),
        (215, 125, "NorthAmerica"),
        (730, 115, "Asia"),
        (525, 260, "Africa"),
        (315, 305, "SouthAmerica"),
        (830, 345, "Australia"),
        (925, 380, "NewZealand"),
    ]
    .into_iter()
    .map(|(x, y, key)| {
        view! {
            <text x=x y=y text-anchor="middle">{data.text(&format!("climateModalMap{key}"))}</text>
        }
    })
    .collect_view();
    let latitudes = [
        (68, "66.5°N"),
        (92, "~55°–62°N"),
        (152, "~35°–43°N"),
        (188, "~23.5°N"),
        (253, ""),
        (318, "~23.5°S"),
        (352, "~35°S"),
        (408, "~55°S"),
    ]
    .into_iter()
    .map(|(y, label)| {
        let label = if label.is_empty() {
            data.text("climateModalMapEquator")
        } else {
            label.into()
        };
        view! { <text x="990" y=y text-anchor="end">{label}</text> }
    })
    .collect_view();
    let current = data.clone();
    view! {
        <svg viewBox="0 20 1000 440" role="img" aria-label=data.text("climateModalMapAria")>
            <defs><clipPath id="climate-land"><path d=data.land.clone() /></clipPath></defs>
            <rect x="0" y="20" width="1000" height="440" fill="#172438" />
            <g clip-path="url(#climate-land)">
                <rect x="0" y="20" width="1000" height="440" fill="#334155" />
                {regions}
            </g>
            <path d=data.land.clone() fill="none" stroke="#94a3b8" stroke-width="1" opacity="0.7" />
            <line x1="0" y1="65" x2="1000" y2="65" stroke="#38bdf8" stroke-dasharray="3 3" opacity="0.3" />
            {boundaries}
            <g fill="#e2e8f0" font-size="11" font-weight="bold" pointer-events="none">{labels}</g>
            <g fill="#cbd5e1" font-size="10" pointer-events="none">
                {latitudes}
                <text x="12" y="45">{data.text("climateModalMapNorthernHemisphere")}</text>
                <text x="12" y="440">{data.text("climateModalMapSouthernHemisphere")}</text>
            </g>
            {move || {
                let region = &current.regions[active.get()];
                view! {
                    <circle cx=region.beacon_x cy=region.beacon_y r="7" stroke="white" stroke-width="2"
                        fill=region.color.clone() />
                }
            }}
            <g class="climate-hit-areas">{hit_areas}</g>
        </svg>
    }
}
