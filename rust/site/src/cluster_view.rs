use crate::{
    calendar::Season,
    catalog::Catalog,
    cluster::{self, Config, Pattern},
    cluster_transfer,
    drawing::number,
    garden::Infrastructure,
    ground_cover::{self, Cover, Data},
    ground_cover_view,
    guild::Guild,
    placement::Metrics,
};
use leptos::prelude::*;
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

#[component]
pub fn ClusterControls(
    catalog: Arc<Catalog>,
    guild: RwSignal<Guild>,
    options: RwSignal<Config>,
    config: Memo<Config>,
    spacing_override: RwSignal<Option<(String, f64)>>,
    language: &'static str,
) -> impl IntoView {
    let limits = catalog.clone();
    let bounds = Memo::new(move |_| {
        guild.with(|state| {
            limits
                .trees
                .iter()
                .find(|tree| tree.id == state.tree_id)
                .map(|tree| {
                    let spacing = cluster::spacing(tree);
                    (spacing.input_min, spacing.input_max)
                })
                .unwrap_or((1.5, 10.0))
        })
    });
    let notice = RwSignal::new(String::new());
    let actions = catalog.clone();
    let on_open = move |_| {
        if cluster_transfer::open(guild.get_untracked(), config.get_untracked(), language).is_err()
        {
            notice.set(actions.text("gardenPageHandoffError"));
        }
    };
    view! {
        <fieldset class="cluster-settings">
            <legend>{catalog.text("gardenPlanClusterTitle")}</legend>
            <div class="settings">
                <label>{catalog.text("gardenPlanNumberOfStarPlants")}
                    <select prop:value=move || options.with(|config| config.count.to_string())
                        on:change=move |event| {
                            let count = event_target_value(&event).parse::<usize>().unwrap_or(1).clamp(1, 6);
                            options.update(|config| {
                                config.count = count;
                                if config.grid_cols.is_some_and(|cols| cols > count) { config.grid_cols = None; }
                            });
                        }>
                        {(1..=6).map(|count| view! { <option value=count.to_string()>{count}</option> }).collect_view()}
                    </select>
                </label>
                <label hidden=move || options.with(|config| config.count < 2)>{catalog.text("gardenPlanLayoutPattern")}
                    <select prop:value=move || options.with(|config| config.pattern.key())
                        on:change=move |event| {
                            let value = event_target_value(&event);
                            options.update(|config| config.pattern = match value.as_str() {
                                "grid" => Pattern::Grid, "triangle" => Pattern::Triangle, _ => Pattern::Line,
                            });
                        }>
                        {[Pattern::Line, Pattern::Grid, Pattern::Triangle].map(|pattern| view! {
                            <option value=pattern.key()>{catalog.text(pattern.label())}</option>
                        })}
                    </select>
                </label>
                <label hidden=move || options.with(|config| config.count < 2 || config.pattern != Pattern::Grid)>
                    {catalog.text("gardenPlanGridColumns")}
                    <select prop:value=move || options.with(|config| config.grid_cols.unwrap_or(0).to_string())
                        on:change=move |event| {
                            let value = event_target_value(&event).parse::<usize>().unwrap_or_default();
                            options.update(|config| config.grid_cols = (1..=config.count).contains(&value).then_some(value));
                        }>
                        <option value="0">{catalog.text("gardenPlanAutoColumns")}</option>
                        {move || options.with(|config| (1..=config.count).map(|count| view! {
                            <option value=count.to_string()>{count}</option>
                        }).collect_view())}
                    </select>
                </label>
                <label hidden=move || options.with(|config| config.count < 2
                    || !(config.pattern == Pattern::Line || (config.pattern == Pattern::Triangle && config.count == 3)))>
                    {catalog.text("gardenPlanOrientation")}
                    <input type="number" step="1" min="0" max="359"
                        prop:value=move || options.with(|config| config.orientation_deg.to_string())
                        on:input=move |event| {
                            if let Ok(value) = event_target_value(&event).parse::<f64>() {
                                if (0.0..=359.0).contains(&value) {
                                    options.update(|config| config.orientation_deg = value);
                                }
                            }
                        } />
                </label>
                <label hidden=move || options.with(|config| config.count < 2)>
                    {catalog.text("gardenPlanPlantSpacing")}
                    <input type="range" step="0.1" min=move || bounds.get().0 max=move || bounds.get().1
                        prop:value=move || config.with(|config| config.spacing_m.to_string())
                        on:change=move |event| {
                            if let Ok(value) = event_target_value(&event).parse::<f64>() {
                                let tree = guild.with_untracked(|state| state.tree_id.clone());
                                let (min, max) = bounds.get_untracked();
                                spacing_override.set(Some((tree, value.clamp(min, max))));
                            }
                        } />
                    <output>{move || config.with(|config| format!("{} m", number(config.spacing_m, 1, language)))}</output>
                </label>
                <button type="button" on:click=on_open>{catalog.text("gardenPlanOpenInGardenGrid")}</button>
            </div>
            <p role="status" hidden=move || notice.with(String::is_empty)>{notice}</p>
        </fieldset>
    }
}

#[component]
pub fn ClusterMap(
    catalog: Arc<Catalog>,
    guild: RwSignal<Guild>,
    config: Memo<Config>,
    cover_data: RwSignal<Option<Arc<Data>>>,
    show_covers: RwSignal<bool>,
    season: RwSignal<Option<Season>>,
    language: &'static str,
    layers: crate::plan::Layers,
    on_select: Callback<String>,
) -> impl IntoView {
    let preference = RwSignal::new(cluster_transfer::auto_resolve());
    let inspector = crate::spot::Inspector::new();
    let select_at_point = Callback::new(move |id| {
        if !inspector.active.get_untracked() {
            on_select.run(id);
        }
    });
    let event = window_event_listener(leptos::ev::storage, move |event| {
        if event
            .key()
            .is_none_or(|key| key == crate::garden::STORAGE_KEY)
        {
            preference.set(cluster_transfer::auto_resolve());
        }
    });
    on_cleanup(move || event.remove());
    let calculations = catalog.clone();
    let preview = Memo::new(move |_| {
        let config = config.get();
        if config.count < 2 {
            return None;
        }
        guild.with(|state| {
            let species = calculations
                .trees
                .iter()
                .find(|tree| tree.id == state.tree_id)?;
            Some(cluster::preview(
                &calculations,
                species,
                config,
                state,
                preference.get(),
                language,
            ))
        })
    });
    let cover_catalog = catalog.clone();
    let covers = Memo::new(move |_| {
        cover_data.with(|data| {
            let Some(data) = data.as_ref() else {
                return Vec::new();
            };
            preview.with(|plan| {
                let Some(plan) = plan else { return Vec::new() };
                guild.with(|state| {
                    ground_cover::garden_at_resolution(
                        data,
                        &cover_catalog,
                        &plan.stars,
                        &plan.resolution.companions,
                        &Infrastructure::default(),
                        &cluster::settings(state, language),
                        Some(0.08),
                    )
                })
            })
        })
    });
    let spot_catalog = catalog.clone();
    let inspect = Callback::new(move |probe: crate::spot::Probe| {
        preview.with(|plan| {
            let Some(plan) = plan else { return Vec::new() };
            covers.with(|areas| {
                crate::spot::grid_query(
                    &spot_catalog,
                    probe,
                    &plan.stars,
                    &plan.resolution.companions,
                    if show_covers.get() { areas } else { &[] },
                    season.get(),
                )
            })
        })
    });
    move || {
        preview.with(|plan| plan.as_ref().and_then(|plan| {
        let species = catalog.trees.iter().find(|tree| tree.id == plan.stars[0].tree_id)?;
        let visible = show_covers.get();
        let bounds = if visible { covers.with(|covers| include_covers(plan.bounds, covers)) } else { plan.bounds };
        let [[min_x, min_y], [max_x, max_y]] = bounds;
        let width = (max_x - min_x).max(max_y - min_y).max(4.0) * 1.16;
        let left = (min_x + max_x - width) / 2.0;
        let top = (min_y + max_y - width) / 2.0;
        let metrics = Metrics::new(species.mature_radius_m);
        let trunk = (width / 150.0).clamp(0.1, 0.25);
        let spread = layers.spread.get();
        let rings_visible = layers.rings.get();
        let southern = guild.with(|state| state.hemisphere == "SOUTHERN");
        let trees = plan.stars.iter().map(|tree| {
            let label = species.name.clone();
            let rings = metrics.rings[..4].iter().rev().map(|&radius| view! { <circle r=radius class="map-ring"/> }).collect_view();
            view! {
                <g data-cluster-tree=tree.instance_id.clone() transform=format!("translate({} {})", tree.x_m, tree.y_m)>
                    {rings_visible.then_some(rings)}
                    {spread.then(|| view! { <circle class="map-canopy" r=metrics.canopy/> })}
                    <circle class="map-trunk" r=trunk/>
                    <title>{label}</title>
                </g>
            }
        }).collect_view();
        let mut counts = BTreeMap::new();
        let points = plan.resolution.companions.iter().filter_map(|plant| {
            let species = catalog.plants.iter().find(|p| p.id == plant.plant_id)?;
            *counts.entry((&species.name, &species.color, &species.id)).or_insert(0) += 1;
            let covered = visible.then(|| covers.with(|areas| {
                areas.iter().find(|area| area.plant_id == plant.plant_id).map(|area|
                    area.detached_anchors.iter().any(|(id, _)| id == &plant.instance_id))
            })).flatten();
            if covered == Some(false) { return None; }
            let dot = (width / 160.0).clamp(0.075, 0.22);
            let id = species.id.clone();
            let show_spread = spread && covered.is_none() && species.spread_m / 2.0 > dot;
            let label = if plant.is_merged {
                format!("{} · {}", species.name, catalog.text("gardenPlanSharedByTrees")
                    .replace("{count}", &plant.servicing_tree_ids.len().to_string()))
            } else { species.name.clone() };
            Some(view! {
                {show_spread.then(|| view! { <circle class="plan-spread" cx=plant.x_m cy=plant.y_m r=species.spread_m / 2.0
                    fill=species.color.clone() stroke=species.color.clone() /> })}
                <circle class="map-plant plan-hit" class:cluster-shared=plant.is_merged data-cluster-companion=plant.instance_id.clone()
                    cx=plant.x_m cy=plant.y_m r=dot fill=species.color.clone()
                    on:click=move |_| select_at_point.run(id.clone())>
                    <title>{label}</title>
                </circle>
            })
        }).collect_view();
        let items = counts.into_iter().map(|((name, color, id), count)| {
            let area = visible.then(|| covers.with(|covers| covers.iter().find(|cover| &cover.plant_id == id).map(|cover| cover.area_m2))).flatten();
            let label = area.map_or_else(|| format!("{name} × {count}"), |area| format!("{name} · {} m²", number(area, 1, language)));
            let id = id.clone();
            view! { <li><button type="button" class="map-legend-item" on:click=move |_| on_select.run(id.clone())>
                <span class="map-key" style:background-color=color.clone()></span>{label}
            </button></li> }
        }).collect_view();
        let mut pairs = HashSet::new();
        let mut dimensions = Vec::new();
        for (i, a) in plan.stars.iter().enumerate() {
            if let Some((j, b)) = plan.stars.iter().enumerate().filter(|(j, _)| i != *j).min_by(|(_, a_near), (_, b_near)| {
                (a.x_m - a_near.x_m).hypot(a.y_m - a_near.y_m)
                    .total_cmp(&(a.x_m - b_near.x_m).hypot(a.y_m - b_near.y_m))
            }) {
                if !pairs.insert((i.min(j), i.max(j))) { continue; }
                let d = (a.x_m - b.x_m).hypot(a.y_m - b.y_m);
                let [x, y] = [(a.x_m + b.x_m) / 2.0, (a.y_m + b.y_m) / 2.0 - width * 0.008];
                let caption = format!("{} m", number(d, 1, language));
                dimensions.push(view! {
                    <g class="cluster-distance">
                        <line x1=a.x_m y1=a.y_m x2=b.x_m y2=b.y_m/>
                        <text x=x y=y style:font-size=format!("{}px", width * 0.026)>{caption}</text>
                    </g>
                });
            }
        }
        let resolved = &plan.resolution.stats;
        let savings = catalog.text("gardenPlanSavings")
            .replace("{optimized}", &resolved.companion_plant_count.to_string())
            .replace("{unoptimized}", &resolved.unoptimized_companion_count.to_string())
            .replace("{percent}", &resolved.savings_percent.to_string());
        let warnings = plan.resolution.unresolved.iter().map(|warning| {
            let (title, detail) = warning.text(&catalog, language);
            view! { <details class=format!("warning-detail {}",warning.severity.class())><summary>{title}</summary><p>{detail}</p></details> }
        }).collect_view();
        let swaps = plan.resolution.substitutions.iter().chain(&plan.resolution.suggestions).map(|sub| {
            let name = |id: &str| catalog.plants.iter().find(|p| p.id == id).map_or_else(|| id.to_owned(), |p| p.name.clone());
            let added = if sub.added_plant_ids.is_empty() { catalog.text("gardenResolveDropped") }
                else { sub.added_plant_ids.iter().map(|id| name(id)).collect::<Vec<_>>().join(" + ") };
            let caption = format!("{} → {added}", name(&sub.removed_plant_id));
            let (title, _) = sub.conflict.text(&catalog, language);
            view! { <details class="warning-detail"><summary>{caption}</summary><p>{title}</p></details> }
        }).collect_view();
        Some(view! {
            <p>{savings}</p>
            <div class="map cluster-map">
                <div>
                <crate::spot::SpotControl catalog=catalog.clone() inspector language />
                <svg role="img" aria-label=catalog.text("gardenPlanClusterTitle") viewBox=format!("{left} {top} {width} {width}") tabindex="0"
                    class:spot-mode=inspector.active
                    on:pointerdown=move |ev| inspector.pointer(&ev, [left, top], width, true)
                    on:pointermove=move |ev| inspector.pointer(&ev, [left, top], width, false)
                    on:pointerleave=move |_| inspector.clear_hover()
                    on:keydown=move |ev| { inspector.keyboard(&ev, [left, top], width); }>
                    {layers.sun.get().then(|| crate::plan::sun_overlay("cluster-sun", left, top, width, southern))}
                    {trees}
                    {visible.then(|| covers.with(|areas| ground_cover_view::selectable_shapes(areas, season.get(), select_at_point)))}
                    {plan.shades.iter().map(|pocket| view! {
                        <circle class="grid-shade" cx=pocket.point[0] cy=pocket.point[1] r=pocket.radius/>
                    }).collect_view()}
                    {dimensions}
                    {points}
                    <g class="map-conflicts" pointer-events="none">
                        {plan.resolution.unresolved.iter().map(|warning| view! {
                            <line data-cluster-conflict=warning.id.clone() class=warning.severity.class()
                                x1=warning.a.point[0] y1=warning.a.point[1] x2=warning.b.point[0] y2=warning.b.point[1]/>
                        }).collect_view()}
                    </g>
                    {inspector.marker()}
                </svg>
                <crate::spot::SpotReadout catalog=catalog.clone() inspector inspect language />
                </div>
                <ul>{items}</ul>
            </div>
            {(!plan.resolution.unresolved.is_empty()).then(|| view! {
                <h3>{catalog.text("gardenWarningsConflicts")}</h3>
                {warnings}
            })}
            {(!plan.resolution.substitutions.is_empty() || !plan.resolution.suggestions.is_empty()).then(|| view! {
                <h3>{catalog.text(if preference.get() { "gardenResolveHeading" } else { "gardenResolveSuggestionsHeading" })}</h3>
                {swaps}
            })}
        })
    }))
    }
}

fn include_covers(mut bounds: [[f64; 2]; 2], covers: &[Cover]) -> [[f64; 2]; 2] {
    for point in covers.iter().flat_map(|cover| cover.rings.iter().flatten()) {
        for (axis, &value) in point.iter().enumerate() {
            bounds[0][axis] = bounds[0][axis].min(value);
            bounds[1][axis] = bounds[1][axis].max(value);
        }
    }
    bounds
}
