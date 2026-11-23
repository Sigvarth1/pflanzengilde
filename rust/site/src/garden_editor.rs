use crate::{
    catalog::{Catalog, Species},
    garden::Star,
    garden_page::{save, Draft},
    garden_resolution::{pin_key, Resolution},
};
use leptos::prelude::*;
use std::sync::Arc;

const PAGE_SIZE: usize = 24;

#[derive(Clone, PartialEq)]
struct Selection {
    id: String,
    tree_id: String,
    name: String,
    plants: Vec<String>,
}

fn edit(
    draft: RwSignal<Draft>,
    resolution: Memo<Resolution>,
    catalog: &Catalog,
    tree_id: &str,
    plant: &Species,
    notice: RwSignal<String>,
) {
    let tree = draft.with_untracked(|state| {
        state
            .garden
            .star_plants
            .iter()
            .find(|tree| tree.instance_id == tree_id)
            .cloned()
    });
    let Some(tree) = tree else { return };
    let Some(species) = catalog.trees.iter().find(|item| item.id == tree.tree_id) else {
        return;
    };
    let name = tree.custom_name.as_deref().unwrap_or(&species.name);
    let mut ids = resolution.with_untracked(|state| {
        state
            .effective_companion_ids
            .get(tree_id)
            .cloned()
            .unwrap_or_default()
    });
    let remove = ids.contains(&plant.id);
    let message = |key| {
        catalog
            .text(key)
            .replace("{plant}", &plant.name)
            .replace("{tree}", name)
    };
    if remove {
        if ids.len() == 1 {
            notice.set(message("gardenPageCompanionLastOne"));
            return;
        }
        ids.retain(|id| id != &plant.id);
    } else {
        if species.juglone_producer && plant.juglone_sensitive {
            notice.set(catalog.text("jugloneWarning"));
            return;
        }
        ids.push(plant.id.clone());
    }
    let pin = pin_key(tree_id, &plant.id);
    save(draft, |state| {
        if let Some(tree) = state
            .garden
            .star_plants
            .iter_mut()
            .find(|tree| tree.instance_id == tree_id)
        {
            tree.selected_plant_ids = ids;
        }
        if remove {
            state.garden.pinned_companions.retain(|key| key != &pin);
        } else if !state.garden.pinned_companions.contains(&pin) {
            state.garden.pinned_companions.push(pin);
        }
    });
    notice.set(message(if remove {
        "gardenPageCompanionRemoved"
    } else {
        "gardenPageCompanionAdded"
    }));
}

#[component]
pub fn CompanionEditor(
    catalog: Arc<Catalog>,
    draft: RwSignal<Draft>,
    selected: RwSignal<String>,
    focused_plant: RwSignal<String>,
    resolution: Memo<Resolution>,
    notice: RwSignal<String>,
    language: &'static str,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let limit = RwSignal::new(PAGE_SIZE);
    let selection_catalog = catalog.clone();
    let target = Memo::new(move |_| {
        let id = selected.get();
        draft.with(|state| {
            let tree = state
                .garden
                .star_plants
                .iter()
                .find(|tree| tree.instance_id == id)
                .or_else(|| {
                    (state.garden.star_plants.len() == 1).then(|| &state.garden.star_plants[0])
                })?;
            let species = selection_catalog
                .trees
                .iter()
                .find(|item| item.id == tree.tree_id)?;
            let plants = resolution.with(|state| {
                state
                    .effective_companion_ids
                    .get(&tree.instance_id)
                    .cloned()
                    .unwrap_or_default()
            });
            Some(Selection {
                id: tree.instance_id.clone(),
                tree_id: tree.tree_id.clone(),
                name: tree
                    .custom_name
                    .clone()
                    .unwrap_or_else(|| species.name.clone()),
                plants,
            })
        })
    });
    let focused_catalog = catalog.clone();
    Effect::new(move |_| {
        let id = focused_plant.get();
        if let Some(plant) = focused_catalog.plants.iter().find(|plant| plant.id == id) {
            query.set(plant.name.clone());
            limit.set(PAGE_SIZE);
        }
    });
    let picker_catalog = catalog.clone();
    let list = Memo::new(move |_| {
        let input = query.with(|value| value.trim().to_lowercase());
        target.with(|target| {
            let Some(target) = target else {
                return Vec::new();
            };
            let mut plants: Vec<_> = picker_catalog
                .plants
                .iter()
                .enumerate()
                .filter(|(_, plant)| !plant.retired || target.plants.contains(&plant.id))
                .filter(|(_, plant)| {
                    input.is_empty()
                        || plant
                            .names
                            .iter()
                            .any(|name| name.to_lowercase().contains(&input))
                        || plant.botanical_name.to_lowercase().contains(&input)
                        || plant
                            .roles
                            .iter()
                            .any(|role| role.to_lowercase().contains(&input))
                })
                .map(|(index, _)| index)
                .collect();
            plants.sort_by(|&a, &b| {
                let [a, b] = [&picker_catalog.plants[a], &picker_catalog.plants[b]];
                target
                    .plants
                    .contains(&b.id)
                    .cmp(&target.plants.contains(&a.id))
                    .then_with(|| a.name.cmp(&b.name))
            });
            plants
        })
    });
    let heading = match language {
        "de" => "Begleitpflanzen bearbeiten",
        "en" => "Edit companion plants",
        _ => "Rediger følgeplanter",
    };
    let rows = catalog.clone();
    let labels = catalog.clone();
    view! {
        <section class="panel garden-editor" id="garden-companion-editor">
            <h2>{heading}</h2>
            <p hidden=move || target.with(Option::is_some)>{catalog.text("gardenPageCompanionNoTree")}</p>
            <div hidden=move || target.with(Option::is_none)>
                <h3>{move || target.with(|tree| tree.as_ref().map(|tree| tree.name.clone()))}</h3>
                <label>{catalog.text("searchPlaceholder")}
                    <input type="search" prop:value=query on:input=move |event| {
                        query.set(event_target_value(&event));
                        limit.set(PAGE_SIZE);
                    } />
                </label>
                {move || target.with(|tree| tree.as_ref().map(|tree| {
                    let active = &tree.plants;
                    let tree_id = tree.id.clone();
                    let walnut = rows.trees.iter().find(|t| t.id == tree.tree_id).is_some_and(|tree| tree.juglone_producer);
                    let items = list.with(|list| list.iter().take(limit.get()).map(|&index| {
                        let species = &rows.plants[index];
                        let checked = active.contains(&species.id);
                        let pin = pin_key(&tree_id, &species.id);
                        let pinned = draft.with(|state| state.garden.pinned_companions.contains(&pin));
                        let blocked = !checked && walnut && species.juglone_sensitive;
                        let last = checked && active.len() == 1;
                        let edit_catalog = rows.clone();
                        let id = tree_id.clone();
                        let plant_name = species.name.clone();
                        let title = blocked.then(|| rows.text("jugloneWarning"))
                            .or_else(|| last.then(|| rows.text("gardenPageCompanionLastOne")
                                .replace("{plant}", &species.name).replace("{tree}", &tree.name)));
                        let aria = if pinned {
                            match language { "de" => "Ausgewählt und fixiert", "en" => "Selected and pinned", _ => "Valgt og låst" }
                        } else { "" };
                        view! {
                            <li class="garden-editor-plant" data-plant=species.id.clone()>
                                <label title=title>
                                    <input type="checkbox" aria-label=plant_name.clone() checked=checked disabled=blocked || last
                                        on:change=move |_| {
                                            edit(draft, resolution, &edit_catalog, &id, &edit_catalog.plants[index], notice);
                                        } />
                                    <span>{plant_name}</span>
                                    {pinned.then(|| view! { <span class="plant-pin" title=aria aria-label=aria>"◆"</span> })}
                                </label>
                                <details>
                                    <summary>{rows.text("details")}</summary>
                                    <small>{species.botanical_name.clone()}</small>
                                    <p>{species.description.clone()}</p>
                                </details>
                            </li>
                        }
                    }).collect_view());
                    view! { <ul class="garden-editor-list">{items}</ul> }
                }))}
                <p hidden=move || list.with(|items| !items.is_empty())>{catalog.text("noPlantsMatched")}</p>
                <button type="button" hidden=move || list.with(|items| items.len() <= limit.get())
                    on:click=move |_| limit.update(|end| *end += PAGE_SIZE)>{labels.text("showAll")}</button>
            </div>
        </section>
    }
}

pub fn label(tree: &Star, catalog: &Catalog) -> String {
    tree.custom_name.clone().unwrap_or_else(|| {
        catalog
            .trees
            .iter()
            .find(|plant| plant.id == tree.tree_id)
            .map_or_else(|| tree.tree_id.clone(), |plant| plant.name.clone())
    })
}
