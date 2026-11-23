use crate::{
    catalog::Catalog,
    garden::Garden,
    garden_conflicts::{Conflict, ShadePocket},
    garden_page::{save, Draft},
    garden_resolution::{pin_key, selected_ids, Resolution, Substitution},
    placement::Sector,
};
use leptos::prelude::*;
use std::sync::Arc;

const SHADE_CHOICES: [&str; 3] = ["plant-wild-garlic", "plant-hosta", "plant-comfrey"];

fn shade_updates(
    garden: &Garden,
    catalog: &Catalog,
    pockets: &[ShadePocket],
    plant: &str,
    automatic: bool,
) -> Vec<(usize, Vec<String>)> {
    garden
        .star_plants
        .iter()
        .enumerate()
        .filter_map(|(index, tree)| {
            if !pockets
                .iter()
                .any(|pocket| pocket.trees.contains(&tree.instance_id))
            {
                return None;
            }
            let species = catalog
                .trees
                .iter()
                .find(|species| species.id == tree.tree_id)?;
            let mut ids = selected_ids(tree, species);
            if ids
                .iter()
                .any(|id| id == plant || (automatic && SHADE_CHOICES.contains(&id.as_str())))
            {
                return None;
            }
            let replace = ids.iter().position(|id| {
                catalog
                    .plants
                    .iter()
                    .find(|p| p.id == *id)
                    .is_some_and(|p| matches!(p.sector, Sector::SouthSun | Sector::WestWind))
            });
            if let Some(index) = replace.or_else(|| ids.len().checked_sub(1)) {
                ids[index] = plant.into();
            } else {
                ids.push(plant.into());
            }
            Some((index, ids))
        })
        .collect()
}

fn apply_shade(
    draft: RwSignal<Draft>,
    catalog: &Catalog,
    pockets: &[ShadePocket],
    plant: &str,
    automatic: bool,
) {
    let updates = draft
        .with_untracked(|state| shade_updates(&state.garden, catalog, pockets, plant, automatic));
    if !updates.is_empty() {
        save(draft, |state| {
            for (index, ids) in updates {
                state.garden.star_plants[index].selected_plant_ids = ids;
            }
        });
    }
}

fn name(catalog: &Catalog, id: &str) -> String {
    catalog
        .plants
        .iter()
        .find(|plant| plant.id == id)
        .map_or_else(|| id.to_owned(), |plant| plant.name.clone())
}

#[component]
pub fn GardenWarnings(
    catalog: Arc<Catalog>,
    draft: RwSignal<Draft>,
    resolution: Memo<Resolution>,
    shade: Memo<Vec<ShadePocket>>,
    notice: RwSignal<String>,
    language: &'static str,
) -> impl IntoView {
    let automatic = Memo::new(move |_| draft.with(|state| state.garden.auto_resolve_enabled));
    let auto_shade = Memo::new(move |_| draft.with(|state| state.garden.auto_shade_enabled));
    let shade_catalog = catalog.clone();
    Effect::new(move |_| {
        if auto_shade.get() {
            shade.with(|pockets| {
                apply_shade(draft, &shade_catalog, pockets, SHADE_CHOICES[0], true)
            });
        }
    });
    let warnings_catalog = catalog.clone();
    let cards_catalog = catalog.clone();
    let shades = catalog.clone();
    let efficiency = catalog.clone();
    view! {
        <section class="panel garden-warnings">
            <h2>{catalog.text("gardenWarningsTitle")}</h2>
            <label class="warnings-switch">
                <input type="checkbox" prop:checked=automatic on:change=move |event| {
                    save(draft, |state| state.garden.auto_resolve_enabled = event_target_checked(&event))
                } />{catalog.text("gardenResolveToggleTitle")}
            </label>
            {move || resolution.with(|result| {
                let warnings = result.unresolved.iter().map(|warning| conflict(warning, &warnings_catalog, language)).collect_view();
                view! {
                    <div class="warning-cards">
                        <h3>{warnings_catalog.text("gardenWarningsCompatHeading")}</h3>
                        {if result.unresolved.is_empty() {
                            view! { <p>{warnings_catalog.text("gardenWarningsAllHarmony")}</p> }.into_any()
                        } else {
                            view! { <p>{result.unresolved.len()} " " {warnings_catalog.text("gardenWarningsConflicts").to_lowercase()}</p> }.into_any()
                        }}
                        {warnings}
                    </div>
                }
            })}
            {move || resolution.with(|result| {
                let auto = result.substitutions.iter().map(|sub| view! {
                    <SwapRow catalog=cards_catalog.clone() sub=sub.clone() draft resolution notice language applied=true />
                }).collect_view();
                let suggested = result.suggestions.iter().map(|sub| view! {
                    <SwapRow catalog=cards_catalog.clone() sub=sub.clone() draft resolution notice language applied=false />
                }).collect_view();
                view! {
                    <div class="warning-cards" hidden=result.substitutions.is_empty()>
                        <h3>{cards_catalog.text("gardenResolveHeading")}</h3>
                        {auto}
                    </div>
                    <div class="warning-cards" hidden=result.suggestions.is_empty()>
                        <h3>{cards_catalog.text("gardenResolveSuggestionsHeading")}</h3>
                        {suggested}
                    </div>
                }
            })}
            <h3>{catalog.text("gardenWarningsCanopyShading")}</h3>
            <label class="warnings-switch">
                <input type="checkbox" prop:checked=auto_shade on:change=move |event| {
                    save(draft, |state| state.garden.auto_shade_enabled = event_target_checked(&event))
                } />{catalog.text("gardenWarningsAutoShadeTitle")}
            </label>
            {move || shade.with(|pockets| {
                if pockets.is_empty() {
                    return view! { <p>{shades.text("gardenWarningsNoShade")}</p> }.into_any();
                }
                let count = shades.text("gardenWarningsShadePockets").replace("{count}", &pockets.len().to_string());
                let buttons = SHADE_CHOICES.into_iter().map(|id| {
                    let title = name(&shades, id);
                    let active_catalog = shades.clone();
                    let button_catalog = shades.clone();
                    view! {
                        <button type="button" disabled=move || shade.with(|pockets| {
                            draft.with(|state| shade_updates(&state.garden, &button_catalog, pockets, id, false).is_empty())
                        })
                        on:click=move |_| {
                            shade.with_untracked(|pockets| apply_shade(draft, &active_catalog, pockets, id, false));
                            notice.set(active_catalog.text("gardenPageShadePlantPlaced").replace("{name}", &name(&active_catalog, id)));
                        }>{title}</button>
                    }
                }).collect_view();
                view! {
                    <p>{count}</p>
                    <div class="settings">{buttons}</div>
                }.into_any()
            })}
            {move || resolution.with(|result| {
                let stats = &result.stats;
                (stats.star_plant_count > 0).then(|| view! {
                    <h3>{efficiency.text("gardenWarningsEfficiency")}</h3>
                    <div class="settings">
                        <span>{stats.companion_plant_count} " " {efficiency.text("gardenWarningsCompanionsVs")
                            .replace("{count}", &stats.unoptimized_companion_count.to_string())}</span>
                        <span>{stats.savings_percent} "% " {efficiency.text("gardenWarningsSaved")}</span>
                        <span>{efficiency.text("gardenWarningsRolesCovered")} " " {stats.covered_roles} " / 9"</span>
                    </div>
                })
            })}
        </section>
    }
}

fn conflict(item: &Conflict, catalog: &Catalog, language: &str) -> impl IntoView {
    let (title, description) = item.text(catalog, language);
    let citations = item
        .citations(catalog)
        .into_iter()
        .map(|source| view! { <li>{source}</li> })
        .collect_view();
    let severity = item.severity.label(language);
    let class = format!("warning-detail {}", item.severity.class());
    let bed_guide = item.guide().map(|anchor| {
        let name = catalog.text("navGuides");
        view! { <p><a href=format!("/guides/raised_beds/?lang={language}#{anchor}")>{name}</a></p> }
    });
    view! {
        <details class=class data-warning=item.id.clone()>
            <summary><strong>{severity}</strong> " · " {title}</summary>
            <p>{description}</p>
            {bed_guide}
            <ul class="warning-sources">{citations}</ul>
        </details>
    }
}

#[component]
fn SwapRow(
    catalog: Arc<Catalog>,
    sub: Substitution,
    draft: RwSignal<Draft>,
    resolution: Memo<Resolution>,
    notice: RwSignal<String>,
    language: &'static str,
    applied: bool,
) -> impl IntoView {
    let removed = name(&catalog, &sub.removed_plant_id);
    let added = if sub.added_plant_ids.is_empty() {
        catalog.text("gardenResolveDropped")
    } else {
        sub.added_plant_ids
            .iter()
            .map(|id| name(&catalog, id))
            .collect::<Vec<_>>()
            .join(" + ")
    };
    let explain = |key: &str, roles: &[String]| {
        (!roles.is_empty()).then(|| {
            catalog.text(key).replace(
                "{roles}",
                &roles
                    .iter()
                    .map(|role| catalog.text(&format!("role:{role}")))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        })
    };
    let kept = explain("gardenResolveRolesKept", &sub.roles_preserved);
    let lost = explain("gardenResolveRolesLost", &sub.roles_lost);
    let reason = catalog
        .text("gardenResolveReason")
        .replace("{reason}", &sub.conflict.text(&catalog, language).0);
    let title = if applied {
        catalog
            .text("gardenResolveUndo")
            .replace("{plant}", &removed)
    } else {
        catalog.text("gardenResolveApply")
    };
    let message = if applied {
        catalog
            .text("gardenResolveKeptToast")
            .replace("{plant}", &removed)
    } else {
        catalog
            .text("gardenResolveAppliedToast")
            .replace("{removed}", &removed)
            .replace("{added}", &added)
    };
    let id = sub.id.clone();
    let apply = move |_| {
        if applied {
            save(draft, |state| {
                for tree in &sub.served_tree_ids {
                    let pin = pin_key(tree, &sub.removed_plant_id);
                    if !state.garden.pinned_companions.contains(&pin) {
                        state.garden.pinned_companions.push(pin);
                    }
                }
            });
        } else {
            resolution.with_untracked(|current| {
                save(draft, |state| current.apply(&mut state.garden, &sub))
            });
        }
        notice.set(message.clone());
    };
    view! {
        <article class="warning-detail" data-swap=id>
            <p><strong>{removed} " → " {added}</strong></p>
            <p>{reason}</p>
            {kept.map(|kept| view! { <p>{kept}</p> })}
            {lost.map(|lost| view! { <p>{lost}</p> })}
            <button type="button" on:click=apply>{title}</button>
        </article>
    }
}
