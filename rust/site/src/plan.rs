use crate::{
    catalog::Catalog,
    cluster::{self, Config},
    cluster_view::{ClusterControls, ClusterMap},
    ground_cover::{self, Data},
    ground_cover_view::{self, DisplayControls},
    guild::Guild,
    placement::{place, Metrics, Zone},
    spot::{self, Inspector, Probe, SpotControl, SpotReadout},
};
use leptos::prelude::*;
use std::sync::Arc;

impl Zone {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Collar => "zoneCollar",
            Self::Bulb => "zoneBulb",
            Self::Mid => "zoneMid",
            Self::Drip => "zoneDrip",
            Self::Outer => "zoneOuter",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Layers {
    pub rings: RwSignal<bool>,
    pub sun: RwSignal<bool>,
    pub spread: RwSignal<bool>,
}

pub fn sun_overlay(
    id: &'static str,
    left: f64,
    top: f64,
    width: f64,
    southern: bool,
) -> impl IntoView {
    let (shade, sun) = if southern {
        ("100%", "0%")
    } else {
        ("0%", "100%")
    };
    view! {
        <g data-plan-sun=if southern { "SOUTHERN" } else { "NORTHERN" } pointer-events="none">
            <defs>
                <linearGradient id=id x1="0%" x2="0%" y1=shade y2=sun>
                    <stop offset="0%" stop-color="#0891b2" stop-opacity=".15" />
                    <stop offset="48%" stop-color="#b8c4ac" stop-opacity="0" />
                    <stop offset="100%" stop-color="#eab308" stop-opacity=".25" />
                </linearGradient>
            </defs>
            <rect x=left y=top width=width height=width fill=format!("url(#{id})") />
        </g>
    }
}

#[component]
pub fn GuildPlan(
    catalog: Arc<Catalog>,
    guild: RwSignal<Guild>,
    cover_data: RwSignal<Option<Arc<Data>>>,
    language: &'static str,
    on_select: Callback<String>,
) -> impl IntoView {
    let title = catalog.text("pdfRadialMap");
    let subtitle = catalog.text("pdfRadialMapSub");
    let map_label = title.clone();
    let age_label = catalog.text("treeAgeLabel");
    let young_label = catalog.text("treeAgeYoung");
    let established_label = catalog.text("treeAgeEstablished");
    let controls = catalog.clone();
    let cluster_controls = catalog.clone();
    let maps = catalog.clone();
    let radial_catalog = catalog.clone();
    let probe_catalog = catalog.clone();
    let tree_catalog = catalog.clone();
    let options = RwSignal::new(Config::default());
    let spacing_override = RwSignal::new(None::<(String, f64)>);
    let config = Memo::new(move |_| {
        let mut config = options.get();
        guild.with(|state| {
            if let Some(tree) = tree_catalog
                .trees
                .iter()
                .find(|tree| tree.id == state.tree_id)
            {
                let limits = cluster::spacing(tree);
                config.spacing_m = spacing_override
                    .with(|value| {
                        value
                            .as_ref()
                            .filter(|(id, _)| id == &tree.id)
                            .map_or(limits.optimal, |(_, value)| *value)
                    })
                    .clamp(limits.input_min, limits.input_max);
            }
        });
        config
    });
    let active = RwSignal::new(true);
    let season = RwSignal::new(None);
    let layers = Layers {
        rings: RwSignal::new(true),
        sun: RwSignal::new(true),
        spread: RwSignal::new(true),
    };
    let layer_controls = [(layers.rings, "toggleZones"), (layers.sun, "toggleSun"), (layers.spread, "spacingToggleCanopy")]
        .into_iter().map(|(flag, key)| view! {
            <label>
                <input type="checkbox" prop:checked=flag on:change=move |event| flag.set(event_target_checked(&event)) />
                {catalog.text(key)}
            </label>
        }).collect_view();
    let ladder = catalog.text("ladderPath");
    let inspector = Inspector::new();
    let select_at_point = Callback::new(move |id| {
        if !inspector.active.get_untracked() {
            on_select.run(id);
        }
    });
    let layout = Memo::new(move |_| {
        guild.with(|state| {
            let Some(tree) = radial_catalog
                .trees
                .iter()
                .find(|tree| tree.id == state.tree_id)
            else {
                return None;
            };
            let plants: Vec<_> = state
                .plant_ids
                .iter()
                .filter_map(|id| radial_catalog.plants.iter().find(|plant| plant.id == *id))
                .collect();
            let metrics = Metrics::new(tree.mature_radius_m);
            let positions = place(tree, &plants, state.hemisphere == "SOUTHERN");
            let report = Arc::new(crate::spacing::analyze(
                &radial_catalog,
                tree,
                state,
                &positions,
                &metrics,
            ));
            let covers = cover_data.with(|data| {
                data.as_ref().map_or_else(Vec::new, |data| {
                    ground_cover::radial(data, tree, &positions, state)
                })
            });
            let extent = positions
                .iter()
                .map(|p| p.distance + p.species.spread_m / 2.0)
                .fold(metrics.rings[4], f64::max)
                + 0.5;
            let markers = positions
                .iter()
                .map(|p| {
                    let radians = f64::from(p.angle).to_radians();
                    let (x, y) = (p.distance * radians.sin(), -p.distance * radians.cos());
                    let label = format!(
                        "{} · {} m · {}° · {}",
                        p.species.name,
                        p.distance,
                        p.angle,
                        radial_catalog.text(p.zone.label())
                    );
                    (
                        p.species.id.clone(),
                        x,
                        y,
                        p.species.spread_m / 2.0,
                        p.species.color.clone(),
                        label,
                    )
                })
                .collect::<Vec<_>>();
            Some((
                extent,
                metrics.canopy,
                metrics.rings,
                markers,
                covers,
                state.hemisphere == "SOUTHERN",
                report,
            ))
        })
    });
    let inspect = Callback::new(move |probe: Probe| {
        guild.with(|state| {
            let tree = probe_catalog
                .trees
                .iter()
                .find(|tree| tree.id == state.tree_id);
            layout.with(|layout| {
                let Some((_, _, _, markers, covers, _, _)) = layout else {
                    return Vec::new();
                };
                spot::query(
                    probe,
                    tree.into_iter().map(|tree| (tree, [0.0, 0.0])),
                    markers.iter().filter_map(|(id, x, y, _, _, _)| {
                        probe_catalog
                            .plants
                            .iter()
                            .find(|plant| &plant.id == id)
                            .map(|p| (p, [*x, *y]))
                    }),
                    if active.get() { covers } else { &[] },
                    season.get(),
                )
            })
        })
    });
    let radial_controls = catalog.clone();
    let radial_readout = catalog.clone();
    let spacing_catalog = catalog.clone();
    view! {
        <section class="panel">
            <h2>{title}</h2>
            <p>{subtitle}</p>
            <ClusterControls catalog=cluster_controls guild options config spacing_override language />
            <label>{age_label}
                <select prop:value=move || guild.with(|g| if g.tree_age == "ESTABLISHED" { "ESTABLISHED" } else { "YOUNG" })
                    on:change=move |event| crate::update_guild(guild, |g| g.tree_age = event_target_value(&event))>
                    <option value="YOUNG">{young_label}</option>
                    <option value="ESTABLISHED">{established_label}</option>
                </select>
            </label>
            <div class="plan-layers settings">{layer_controls}</div>
            <DisplayControls catalog=controls active season />
            <ClusterMap catalog=maps guild config cover_data show_covers=active season language layers on_select />
            {move || (config.with(|config| config.count) == 1).then(|| layout.get()).flatten().map(|(extent, canopy, rings, markers, covers, southern, _)| {
                let visible = active.get();
                let extent = if visible {
                    covers.iter().flat_map(|cover| cover.rings.iter().flatten())
                        .fold(extent, |extent, [x, y]| extent.max(x.abs() + 0.35).max(y.abs() + 0.35))
                } else { extent };
                let viewbox = format!("{} {} {} {}", -extent, -extent, 2.0 * extent, 2.0 * extent);
                let shown = |id: &str| visible.then(|| covers.iter().find(|cover| cover.plant_id == id)).flatten();
                let legend = markers.iter().map(|(id, _, _, _, color, label)| {
                    let label = shown(id).map_or_else(|| label.clone(), |cover| format!("{} · {:.1} m²", cover.name, cover.area_m2));
                    let id = id.clone();
                    view! { <li><button type="button" class="map-legend-item" on:click=move |_| on_select.run(id.clone())>
                        <span class="map-key" style:background-color=color.clone()></span>{label}
                    </button></li> }
                }).collect_view();
                let spread = layers.spread.get();
                let dot = (extent * 0.012).clamp(0.08, 0.23);
                let marks = markers.into_iter().filter_map(|(id, x, y, radius, color, label)| {
                    let covered = shown(&id);
                    if covered.is_some_and(|cover| cover.detached_anchors.is_empty()) { return None; }
                    let plant_id = id.clone();
                    let grow = spread && covered.is_none() && radius > dot;
                    Some(view! {
                        {grow.then(|| view! { <circle cx=x cy=y r=radius class="plan-spread" fill=color.clone() stroke=color.clone() /> })}
                        <circle cx=x cy=y r=dot fill=color class="map-plant plan-hit" data-plant=id
                            on:click=move |_| select_at_point.run(plant_id.clone())>
                            <title>{label}</title>
                        </circle>
                    })
                }).collect_view();
                view! {
                    <div class="map">
                        <div>
                        <SpotControl catalog=radial_controls.clone() inspector language />
                        <svg role="img" aria-label=map_label.clone() viewBox=viewbox tabindex="0"
                            class:spot-mode=inspector.active
                            on:pointerdown=move |ev| inspector.pointer(&ev, [-extent, -extent], 2.0 * extent, true)
                            on:pointermove=move |ev| inspector.pointer(&ev, [-extent, -extent], 2.0 * extent, false)
                            on:pointerleave=move |_| inspector.clear_hover()
                            on:keydown=move |ev| { inspector.keyboard(&ev, [-extent, -extent], 2.0 * extent); }>
                            {layers.sun.get().then(|| sun_overlay("radial-sun", -extent, -extent, 2.0 * extent, southern))}
                            <path class="plan-access" d=access_sector(rings[4])><title>{ladder.clone()}</title></path>
                            {layers.rings.get().then(|| rings.into_iter().rev().map(|radius| view! {
                                <circle r=radius class="map-ring" />
                            }).collect_view())}
                            {spread.then(|| view! { <circle r=canopy class="map-canopy"/> })}
                            {visible.then(|| ground_cover_view::selectable_shapes(&covers, season.get(), select_at_point))}
                            <circle r="0.08" class="map-trunk"/>
                            {marks}
                            {inspector.marker()}
                        </svg>
                        <SpotReadout catalog=radial_readout.clone() inspector inspect language from_trunk=true />
                        </div>
                        <ul>{legend}</ul>
                    </div>
                }
            })}
            {move || (config.with(|config| config.count) == 1).then(|| layout.with(|layout| {
                let (_, _, _, _, _, _, report) = layout.as_ref()?;
                (!report.conflicts.is_empty() || !report.crowded.is_empty()).then(|| view! {
                    <crate::spacing_view::SpacingWarnings catalog=spacing_catalog.clone() guild report=report.clone() language />
                })
            })).flatten()}
        </section>
    }
}

fn access_sector(radius: f64) -> String {
    let point = |bearing: f64| {
        let (sin, cos) = bearing.to_radians().sin_cos();
        (radius * sin, -radius * cos)
    };
    let (x1, y1) = point(215.0);
    let (x2, y2) = point(245.0);
    format!("M 0 0 L {x1} {y1} A {radius} {radius} 0 0 1 {x2} {y2} Z")
}
