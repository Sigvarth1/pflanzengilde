use crate::{
    calendar::Season,
    catalog::Catalog,
    ground_cover::{Cover, Data},
};
use leptos::prelude::*;
use std::sync::Arc;
use wasm_bindgen_futures::spawn_local;

pub fn load() -> RwSignal<Option<Arc<Data>>> {
    let data = RwSignal::new(None);
    spawn_local(async move {
        if let Ok(config) = crate::fetch_json("ground-cover.json").await {
            data.set(Some(Arc::new(config)));
        }
    });
    data
}

#[component]
pub fn DisplayControls(
    catalog: Arc<Catalog>,
    active: RwSignal<bool>,
    season: RwSignal<Option<Season>>,
) -> impl IntoView {
    view! {
        <div class="cover-controls settings">
            <label>
                <input type="checkbox" prop:checked=active
                    on:change=move |event| active.set(event_target_checked(&event)) />
                {catalog.text("showGroundCovers")}
            </label>
            <label>{catalog.text("coverSeasonLabel")}
                <select disabled=move || !active.get() prop:value=move || season.get().map_or("", Season::slug)
                    on:change=move |event| {
                        let value = event_target_value(&event);
                        season.set(Season::ALL.into_iter().find(|season| season.slug() == value));
                    }>
                    <option value="">{catalog.text("coverSeasonAll")}</option>
                    {Season::ALL.into_iter().map(|season| view! {
                        <option value=season.slug()>{catalog.text(season.label())}</option>
                    }).collect_view()}
                </select>
            </label>
        </div>
    }
}

pub fn shapes(covers: &[Cover], season: Option<Season>) -> impl IntoView {
    render_shapes(covers, season, None)
}

pub fn selectable_shapes(
    covers: &[Cover],
    season: Option<Season>,
    on_select: Callback<String>,
) -> impl IntoView {
    render_shapes(covers, season, Some(on_select))
}

fn render_shapes(
    covers: &[Cover],
    season: Option<Season>,
    on_select: Option<Callback<String>>,
) -> impl IntoView {
    covers.iter().map(|cover| {
        let in_season = cover.in_season(season);
        let fill = if in_season { cover.opacity } else { 0.05 };
        let stroke = if in_season { 0.55 } else { 0.25 };
        let dash = if !in_season { Some("2 3") } else if cover.hard_edge { None } else { Some("5 3") };
        let color = cover.color.clone();
        let title = format!("{} · {:.1} m²", cover.name, cover.area_m2);
        let id = cover.plant_id.clone();
        view! {
            <g class="ground-cover" class:plan-hit=on_select.is_some() data-cover=cover.plant_id.clone()
                pointer-events={if on_select.is_some() { "visiblePainted" } else { "none" }}
                on:click=move |_| { if let Some(select) = on_select { select.run(id.clone()); } }>
                <title>{title}</title>
                <path d=Cover::path(&cover.rings) fill=color.clone() fill-rule="evenodd" fill-opacity=fill
                    stroke=color.clone() stroke-opacity=stroke stroke-dasharray=dash stroke-width="1"
                    vector-effect="non-scaling-stroke" stroke-linejoin="round" />
                {cover.dots.iter().map(|&[x, y]| view! {
                    <circle cx=x cy=y r="0.04" fill=color.clone() fill-opacity=if in_season { 0.75 } else { 0.18 } />
                }).collect_view()}
            </g>
        }
    }).collect_view()
}
