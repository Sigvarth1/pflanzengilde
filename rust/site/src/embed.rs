use crate::{
    catalog::{Catalog, Species},
    drawing::number,
    garden::{Garden, Settings},
    garden_layout::Companion,
    guild::Guild,
    routes,
};
use leptos::prelude::*;
use std::{collections::BTreeSet, sync::Arc};

pub type SharedGarden = (Garden, Settings, Vec<Companion>, String);

#[component]
pub fn Embed(
    catalog: Arc<Catalog>,
    guild: Guild,
    garden: Option<SharedGarden>,
    language: &'static str,
) -> impl IntoView {
    let (impressum, privacy) = routes::legal();
    let content = if let Some((garden, settings, companions, code)) = garden {
        let link = format!("/garten/?garden={code}&lang={language}");
        garden_card(
            &catalog,
            &garden,
            &settings,
            &companions,
            Some(link),
            language,
        )
        .into_any()
    } else {
        guild_card(&catalog, &guild, language, true).into_any()
    };
    view! {
        <main class="embed-page">
            {content}
            <footer class="embed-legal">
                {impressum.then(|| view! { <a href=format!("/impressum/?lang={language}") target="_blank" rel="noopener noreferrer">{catalog.text("impressumLink")}</a> })}
                {privacy.then(|| view! { <a href=format!("/datenschutz/?lang={language}") target="_blank" rel="noopener noreferrer">{catalog.text("appPrivacyLink")}</a> })}
            </footer>
        </main>
    }
}

pub(crate) fn guild_card(
    catalog: &Catalog,
    guild: &Guild,
    language: &str,
    show_action: bool,
) -> impl IntoView {
    let Some(tree) = catalog.trees.iter().find(|tree| tree.id == guild.tree_id) else {
        return ().into_any();
    };
    let selected: Vec<_> = catalog
        .plants
        .iter()
        .filter(|plant| guild.plant_ids.contains(&plant.id))
        .collect();
    let name = tree.name.clone();
    let credit = tree.photo_credit.clone().map(|label| {
        let href = format!("/credits/?lang={language}#credit-{}", tree.id);
        view! {
            <a class="embed-credit" href=href target="_blank" rel="noopener noreferrer"
                title=label.clone() aria-label=label>"©"</a>
        }
    });
    view! {
        <article class="embed-card embed-guild">
            <img class="embed-backdrop" src=tree.image_url.clone() alt=name.clone() />
            {credit}
            <div class="embed-content">
                <div class="embed-chips">
                    <span>{catalog.text("permacultureGuildLabel")}</span>
                    {(!selected.is_empty()).then(|| view! {
                        <span>{format!("{} {}", selected.len(), catalog.text("embedCardCompanions"))}</span>
                    })}
                </div>
                <h1>{name}</h1>
                <p class="embed-botanical">{tree.botanical_name.clone()}</p>
                <p class="embed-description">{tree.description.clone()}</p>
                <div class="embed-chips">
                    {selected.iter().take(4).map(|plant| chip(plant, 0)).collect_view()}
                    {more(catalog, selected.len(), 4)}
                </div>
                <div class="embed-action">
                    <a class="embed-brand" href="/" target="_blank" rel="noopener noreferrer">
                        <strong>"Pflanzengilde.de"</strong>
                        <small>{catalog.text("embedCardTagline")}</small>
                    </a>
                    {show_action.then(|| view! {
                        <a class="embed-button" href=format!("/?g={}&lang={language}", guild.encode(catalog, language))
                            target="_blank" rel="noopener noreferrer">{catalog.text("openInPlanner")}</a>
                    })}
                </div>
            </div>
        </article>
    }.into_any()
}

pub(crate) fn garden_card(
    catalog: &Catalog,
    garden: &Garden,
    settings: &Settings,
    companions: &[Companion],
    link: Option<String>,
    language: &str,
) -> impl IntoView {
    let trees: Vec<_> = garden
        .star_plants
        .iter()
        .filter_map(|star| {
            let species = catalog.trees.iter().find(|tree| tree.id == star.tree_id)?;
            Some((species, [star.x_m, star.y_m], false))
        })
        .collect();
    let plants: Vec<_> = companions
        .iter()
        .filter_map(|plant| {
            let species = catalog
                .plants
                .iter()
                .find(|entry| entry.id == plant.plant_id)?;
            Some((species, [plant.x_m, plant.y_m], plant.is_merged))
        })
        .collect();
    let title = if garden.garden_name.trim().is_empty() {
        catalog.text("gardenPageDefaultName")
    } else {
        garden.garden_name.clone()
    };
    let tree_chips = inventory(&trees);
    let plant_chips = inventory(&plants);
    let conditions = [settings.soil.as_deref(), settings.climate.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    view! {
        <article class="embed-card embed-garden">
            {garden_map(&trees, &plants, &title, language)}
            <div class="embed-content">
                <div class="embed-chips">
                    <span>{catalog.text("shareGardenMultiGuildPlan")}</span>
                    <small>{conditions}</small>
                </div>
                <h1>{title}</h1>
                <p>{format!("{} {} · {} {}", trees.len(), catalog.text("shareGardenStarTrees"),
                    plants.len(), catalog.text("shareGardenCompanions"))}</p>
                <div class="embed-chips">
                    {tree_chips.iter().map(|(species, count)| chip(species, *count)).collect_view()}
                    {plant_chips.iter().take(6).map(|(species, count)| chip(species, *count)).collect_view()}
                    {more(catalog, plant_chips.len(), 6)}
                </div>
                {link.map(|href| view! {
                    <a class="embed-button" href=href target="_blank" rel="noopener noreferrer">
                        {catalog.text("gardenPlanOpenInGardenGrid")}
                    </a>
                })}
            </div>
        </article>
    }
}

type Marker<'a> = (&'a Species, [f64; 2], bool);

fn inventory<'a>(points: &[Marker<'a>]) -> Vec<(&'a Species, usize)> {
    let mut inventory: Vec<(&Species, usize)> = Vec::new();
    for &(species, _, _) in points {
        if let Some((_, count)) = inventory
            .iter_mut()
            .find(|(entry, _)| entry.id == species.id)
        {
            *count += 1;
        } else {
            inventory.push((species, 1));
        }
    }
    inventory
}

fn chip(species: &Species, count: usize) -> impl IntoView {
    let label = if count == 0 {
        species.name.clone()
    } else {
        format!("{count}× {}", species.name)
    };
    view! { <span><i style:background-color=species.color.clone()></i>{label}</span> }
}

fn more(catalog: &Catalog, total: usize, limit: usize) -> impl IntoView {
    (total > limit).then(|| {
        view! {
            <span>{format!("+{} {}", total - limit, catalog.text("embedCardMore"))}</span>
        }
    })
}

fn garden_map(
    trees: &[Marker<'_>],
    plants: &[Marker<'_>],
    title: &str,
    language: &str,
) -> impl IntoView {
    let span = trees
        .iter()
        .chain(plants)
        .flat_map(|marker| marker.1.map(f64::abs))
        .fold(8.0, f64::max);
    let point = |[x, y]: [f64; 2]| {
        [
            100.0 + x * 84.0 / (span * 1.35),
            100.0 + y * 84.0 / (span * 1.35),
        ]
    };
    let mut edges = BTreeSet::new();
    for (i, &(_, [x, y], _)) in trees.iter().enumerate() {
        let mut neighbours: Vec<_> = trees
            .iter()
            .enumerate()
            .filter_map(|(j, &(_, [tx, ty], _))| (i != j).then(|| (j, (tx - x).hypot(ty - y))))
            .collect();
        neighbours.sort_by(|a, b| a.1.total_cmp(&b.1));
        edges.extend(
            neighbours
                .into_iter()
                .take(2)
                .map(|(j, _)| (i.min(j), i.max(j))),
        );
    }
    let edges = edges.into_iter().map(|(i, j)| {
        let [x, y] = trees[i].1;
        let [tx, ty] = trees[j].1;
        let [ax, ay] = point([x, y]);
        let [bx, by] = point([tx, ty]);
        let distance = format!("{} m", number((tx - x).hypot(ty - y), 1, language));
        let mx = (ax + bx) / 2.0;
        let my = (ay + by) / 2.0;
        view! {
            <g>
                <line x1=ax y1=ay x2=bx y2=by stroke="#94a3b8" />
                <rect x={mx - 14.0} y={my - 6.0} width="28" height="11" rx="2" fill="#0f172a" />
                <text x=mx y={my + 2.5} text-anchor="middle" font-size="6.5" fill="#e2e8f0">{distance}</text>
            </g>
        }
    }).collect_view();
    let markers = |points: &[Marker<'_>], star| {
        points.iter().map(|&(species, coords, merged)| {
        let [x, y] = point(coords);
        let color = species.color.clone();
        view! {
            <g>

                {merged.then(|| view! { <circle cx=x cy=y r="5.2" fill="none" stroke=color.clone() /> })}
                <circle cx=x cy=y r={if star { "5.5" } else { "3.2" }} fill=color />
                {star.then(|| view! { <circle cx=x cy=y r="1.6" fill="white" /> })}
            </g>
        }
    }).collect_view()
    };
    view! {
        <svg class="embed-map" viewBox="0 0 200 200" role="img" aria-label=title.to_owned()>
            <line x1="100" y1="0" x2="100" y2="200" stroke="#334155" stroke-dasharray="2,2"/>
            <line x1="0" y1="100" x2="200" y2="100" stroke="#334155" stroke-dasharray="2,2"/>
            {edges}
            {markers(plants, false)}
            {markers(trees, true)}
        </svg>
    }
}
