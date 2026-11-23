use crate::{
    calendar::Season,
    catalog::{Catalog, Species},
    drawing::number,
    guild::{Guild, CLIMATES, SOILS},
    pest_defense::{self, Citation},
};
use leptos::{html, prelude::*};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use wasm_bindgen_futures::spawn_local;

#[derive(Deserialize)]
struct References {
    credit: Option<String>,
    sources: Vec<Citation>,
}

type Notes = Arc<HashMap<String, References>>;

#[component]
pub fn PlantDetails(
    catalog: Arc<Catalog>,
    guild: RwSignal<Guild>,
    selected: RwSignal<Option<String>>,
    dialog: NodeRef<html::Dialog>,
    language: &'static str,
) -> impl IntoView {
    let notes = RwSignal::new(None::<Notes>);
    let loading = RwSignal::new(false);
    Effect::new(move |_| {
        if selected.with(Option::is_none)
            || notes.with_untracked(Option::is_some)
            || loading.get_untracked()
        {
            return;
        }
        loading.set(true);
        spawn_local(async move {
            if let Ok(data) = crate::fetch_json(&format!("plant-details-{language}.json")).await {
                notes.set(Some(Arc::new(data)));
            }
            loading.set(false);
        });
    });
    let close = catalog.text("closeBtn");
    let title_catalog = catalog.clone();
    view! {
        <dialog class="climate-dialog plant-dialog" node_ref=dialog aria-labelledby="plant-details-title"
            on:close=move |_| selected.set(None)
            on:click=move |ev| {
                if let Some(node) = dialog.get_untracked() {
                    if ev.target().is_some_and(|target| web_sys::EventTarget::from(node.clone()) == target) {
                        node.close();
                    }
                }
            }>
            <div class="climate-dialog-inner">
                <header class="climate-dialog-header">
                    <div>
                        <h2 id="plant-details-title">{move || dialog_title(&title_catalog, selected)}</h2>
                    </div>
                    <form method="dialog"><button type="submit" aria-label=close autofocus>"×"</button></form>
                </header>
                {move || selected.with(|id| {
                    let plant = catalog.plants.iter().find(|plant| Some(&plant.id) == id.as_ref())?;
                    Some(view! { <DetailsBody catalog=catalog.clone() plant=plant.clone() guild notes language /> })
                })}
            </div>
        </dialog>
    }
}

fn dialog_title(catalog: &Catalog, selected: RwSignal<Option<String>>) -> String {
    selected.with(|id| {
        catalog
            .plants
            .iter()
            .find(|plant| Some(&plant.id) == id.as_ref())
            .map_or_else(String::new, |plant| plant.name.clone())
    })
}

#[component]
fn DetailsBody(
    catalog: Arc<Catalog>,
    plant: Species,
    guild: RwSignal<Guild>,
    notes: RwSignal<Option<Notes>>,
    language: &'static str,
) -> impl IntoView {
    let plant = Arc::new(plant);
    let credit_id = plant.id.clone();
    let refs = move || notes.get().filter(|data| data.contains_key(&credit_id));
    let selected_id = plant.id.clone();
    let checked = Memo::new(move |_| guild.with(|state| state.plant_ids.contains(&selected_id)));
    let add = catalog.text("addGuildBtn");
    let remove = catalog.text("removeGuildBtn");
    let warning_plant = plant.clone();
    let warning_catalog = catalog.clone();
    let problems = Memo::new(move |_| {
        guild.with(|state| warning_catalog.guild_incompatibilities(&warning_plant, state))
    });
    let toggle_catalog = catalog.clone();
    let toggle_plant = plant.clone();
    let warning = move || problems.with(|reasons| reasons.join(" · "));
    let disabled = plant.retired;
    let dimensions = format!(
        "{} m / {} m",
        number(plant.spread_m, 2, language),
        number(plant.height_m.unwrap_or_default(), 2, language)
    );
    let distance = format!(
        "{}–{} m",
        number(plant.min_distance_m, 2, language),
        number(plant.max_distance_m, 2, language)
    );
    let climates = labels(&plant.climate_zones, &CLIMATES, &catalog).join(" · ");
    let soils = labels(&plant.suitable_soils, &SOILS, &catalog).join(" · ");
    let unsuitable = labels(&plant.unsuitable_soils, &SOILS, &catalog).join(" · ");
    let credit = plant.id.clone();
    let sources_id = plant.id.clone();
    let role_links = plant.role_ids.iter().map(|role| view! {
        <a href=format!("/guides/?lang={language}#role-{role}")>{catalog.text(&format!("role:{role}"))}</a>
    }).collect_view();
    let advice_catalog = catalog.clone();
    let advice_plant = plant.clone();
    let advice_id = plant.id.clone();
    let citation_heading = catalog.text("evidenceSourcesTitle");
    let reports = Memo::new(move |_| {
        guild.with(|state| {
            let Some(tree) = advice_catalog
                .trees
                .iter()
                .find(|tree| tree.id == state.tree_id)
            else {
                return Vec::new();
            };
            let Some(index) = advice_catalog
                .plants
                .iter()
                .position(|item| item.id == advice_id)
            else {
                return Vec::new();
            };
            let mut seen = HashSet::new();
            tree.vulnerabilities
                .iter()
                .filter_map(|pest| {
                    let result =
                        pest_defense::resolve(&advice_catalog, tree, pest, &state.climate_zone)?;
                    (result.plants.contains(&index) && seen.insert(result.rule))
                        .then_some(result.rule)
                })
                .collect::<Vec<_>>()
        })
    });
    let advice_title = catalog.text("pestDefenseForStarTree");
    let evidence_catalog = catalog.clone();

    view! {
        <div class="plant-profile">
            <img src=plant.image_url.clone() alt=plant.name.clone() width="160" height="160" />
            <div>
                <p><i>{plant.botanical_name.clone()}</i></p>
                <p>{plant.description.clone()}</p>
                {move || refs().and_then(|data| data[&credit].credit.clone()).map(|label| view! {
                    <p><small><a href=format!("/credits/?lang={language}#credit-{credit}")>{label}</a></small></p>
                })}
            </div>
        </div>
        <h3>{catalog.text("modalRolesTitle")}</h3>
        <div class="plant-role-links">{role_links}</div>
        <dl class="plant-facts">
            {[
                ("layer", plant.layer_label.clone()),
                ("modalSector", plant.sector_label.clone()),
                ("modalZone", catalog.text(plant.zone.label())),
                ("modalSpreadHeight", dimensions),
                ("modalDistance", distance),
                ("modalJuglone", plant.juglone_label.clone()),
                ("lightPref", plant.sun_label.clone()),
                ("climateZone", climates),
                ("plantingTime", plant.activity.planting_time.clone()),
                ("harvestTime", plant.activity.harvest_time.clone()),
            ].into_iter().filter(|(_, value)| !value.is_empty()).map(|(label, value)| view! {
                <div><dt>{catalog.text(label)}</dt><dd>{value}</dd></div>
            }).collect_view()}
        </dl>
        <h3>{catalog.text("modalSoilTitle")}</h3>
        <p>{plant.activity.soil_notes.clone()}</p>
        <p>{catalog.text("plantDetailSuitable")} " " {soils}</p>
        {(!unsuitable.is_empty()).then(|| view! { <p>{catalog.text("plantDetailUnsuitable")} " " {unsuitable}</p> })}
        <h3>{catalog.text("modalPhenologyTitle")}</h3>
        <dl class="plant-seasons">{Season::ALL.into_iter().map(|season| {
            let activity = &plant.activity;
            let tasks = [
                (&activity.planting, "plantingSeasonTag"), (&activity.harvest, "harvestSeasonTag"),
                (&activity.flowering, "bloomingTag"), (&activity.foliage, "livingMulchTag"),
                (&activity.chop, "chopDropTag"), (&activity.pest_deterrence, "pestDefenseTag"),
            ].into_iter().filter(|(seasons, _)| seasons.contains(&season)).map(|(_, key)| catalog.text(key)).collect::<Vec<_>>();
            let tasks = if tasks.is_empty() { catalog.text("dormantTag") } else { tasks.join(" · ") };
            view! { <div><dt>{catalog.text(season.label())}</dt><dd>{tasks}</dd></div> }
        }).collect_view()}</dl>
        {move || reports.with(|rules| {
            let name = guild.with(|state| evidence_catalog.trees.iter()
                .find(|tree| tree.id == state.tree_id).map_or_else(String::new, |tree| tree.name.clone()));
            (!rules.is_empty()).then(|| view! {
                <h3>{advice_title.replace("{tree}", &name)}</h3>
                {rules.iter().map(|&i| {
                    let rule = &evidence_catalog.pest_defense_rules[i];
                    view! {
                        <details>
                            <summary>{rule.title.clone()} " · " {rule.companion_evidence(&advice_plant.id).label(&evidence_catalog)}</summary>
                            <p>{rule.companion_role(&advice_plant.id)}</p>
                            <p>{rule.mechanism.clone()}</p>
                            <ul>{citations(&rule.citations)}</ul>
                        </details>
                    }
                }).collect_view()}
            })
        })}
        {move || notes.with(|data| {
            let sources = &data.as_ref()?.get(&sources_id)?.sources;
            (!sources.is_empty()).then(|| view! {
                <h3>{citation_heading.clone()}</h3>
                <ul>{citations(sources)}</ul>
            })
        })}
        <p class="warning" hidden=move || problems.with(Vec::is_empty)>{warning}</p>
        <button type="button" disabled=move || !checked.get() && (disabled || !problems.with(Vec::is_empty))
            on:click=move |_| crate::toggle_companion(guild, &toggle_catalog, &toggle_plant, !checked.get_untracked())>
            {move || if checked.get() { remove.clone() } else { add.clone() }}
        </button>
    }
}

fn labels(values: &[String], options: &[(&str, &str)], catalog: &Catalog) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            options
                .iter()
                .find(|(id, _)| id == value)
                .map(|(_, label)| catalog.text(label))
        })
        .collect()
}

fn citations(sources: &[Citation]) -> impl IntoView {
    sources.iter().map(|citation| {
        view! {
            <li>{if let Some(href) = &citation.href {
                view! { <a href=href.clone() target="_blank" rel="noopener noreferrer">{citation.label.clone()}</a> }.into_any()
            } else {
                citation.label.clone().into_any()
            }}</li>
        }
    }).collect_view()
}
