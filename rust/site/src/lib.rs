mod calendar;
mod calendar_view;
mod catalog;
mod catalog_controls;
mod climate_map;
mod cluster;
mod cluster_transfer;
mod cluster_view;
mod compatibility;
mod download;
mod drawing;
mod dxf_export;
mod embed;
mod garden;
mod garden_code;
mod garden_conflicts;
mod garden_editor;
mod garden_exports;
mod garden_layout;
mod garden_map;
mod garden_page;
mod garden_resolution;
mod garden_warnings;
mod ground_cover;
mod ground_cover_view;
mod guild;
mod guild_antagonisms;
mod guild_cards;
mod guild_io;
mod infrastructure;
mod pdf_export;
mod pest_defense;
mod pest_view;
mod placement;
mod plan;
mod pickers;
mod plant_details;
mod routes;
mod seasonal;
mod share_dialog;
mod spacing;
mod spacing_view;
mod spot;

use base64::Engine as _;
use catalog::{Catalog, Species};
use guild::{Guild, CLIMATES};
use leptos::{html, prelude::*};
use std::sync::Arc;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Response, Storage, UrlSearchParams};

const STORAGE_KEY: &str = "permaculture_plant_guild_v1";
const LANGUAGE_KEY: &str = "permaculture_plant_guild_lang";
const SOIL_KEY: &str = "permaculture_plant_guild_soil";
const ZONE_KEY: &str = "permaculture_plant_guild_zone";
const PAGE_SIZE: usize = 32;

#[wasm_bindgen]
pub fn mount(preloaded: js_sys::Map) -> Result<(), JsValue> {
    let window = window();
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("Missing document"))?;
    let root = document
        .get_element_by_id("planner")
        .ok_or_else(|| JsValue::from_str("Missing planner element"))?
        .dyn_into::<web_sys::HtmlElement>()?;
    let storage = window.local_storage().ok().flatten();
    let params = UrlSearchParams::new_with_str(&window.location().search()?)?;
    let pathname = window.location().pathname()?;
    let embed_route = matches!(
        pathname.trim_end_matches('/'),
        "/embed" | "/embed/index.html"
    );
    let language = language(&window, storage.as_ref(), &params, embed_route);
    let garden_route = routes::is_garden(&pathname) || params.has("garden");

    spawn_local(async move {
        let name = format!("catalog-{language}.json");
        match fetch_startup_data::<Catalog>(&name, &preloaded).await {
            Ok(catalog) => {
                if let Some(page) = document.document_element() {
                    let _ = page.set_attribute("lang", language);
                }
                let saved = storage
                    .as_ref()
                    .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten());
                let mut defaults = Guild::from_defaults(&catalog);
                let old_setting = |key| {
                    storage
                        .as_ref()
                        .and_then(|s| s.get_item(key).ok().flatten())
                };
                defaults.update_location(
                    old_setting(SOIL_KEY).as_deref(),
                    old_setting(ZONE_KEY).as_deref(),
                    None,
                );
                let initial = Guild::recover(
                    &catalog,
                    defaults,
                    saved.as_deref(),
                    &params,
                    routes::shared_tree(&pathname),
                );
                if embed_route {
                    let shared = params.get("garden").and_then(|code| {
                        let (garden, settings) = garden_code::decode(&catalog, &code)?;
                        Some((garden, settings, code))
                    });
                    let garden = if let Some((garden, mut settings, code)) = shared {
                        settings
                            .soil
                            .get_or_insert_with(|| initial.soil_type.clone());
                        settings
                            .climate
                            .get_or_insert_with(|| initial.climate_zone.clone());
                        let companions = if garden.star_plants.is_empty() {
                            Vec::new()
                        } else {
                            let sources =
                                match fetch_startup_data("garden-tree-data.json", &preloaded).await
                                {
                                    Ok(sources) => sources,
                                    Err(error) => {
                                        show_load_error(&root, &error);
                                        return;
                                    }
                                };
                            garden_resolution::resolve(&catalog, &garden, &settings, &sources)
                                .companions
                        };
                        Some((garden, settings, companions, code))
                    } else {
                        None
                    };
                    root.set_text_content(None);
                    leptos::mount::mount_to(root, move || {
                        view! { <embed::Embed catalog=Arc::new(catalog) guild=initial garden language /> }
                    })
                    .forget();
                } else if garden_route {
                    let sources = fetch_startup_data::<serde_json::Map<String, serde_json::Value>>(
                        "garden-tree-data.json",
                        &preloaded,
                    )
                    .await;
                    let sources = match sources {
                        Ok(sources) => sources,
                        Err(error) => {
                            show_load_error(&root, &error);
                            return;
                        }
                    };
                    let saved = storage
                        .as_ref()
                        .and_then(|s| s.get_item(garden::STORAGE_KEY).ok().flatten());
                    let (mut draft, templates_changed) = garden_page::Draft::load(
                        &catalog,
                        &sources,
                        saved.as_deref(),
                        &initial,
                        language,
                        params.get("garden").as_deref(),
                    );
                    if let (true, Some(storage)) = (templates_changed, &storage) {
                        draft.persist_templates(storage, saved.as_deref());
                    }
                    let mut imported = (Vec::new(), String::new());
                    if let Some(transfer) = cluster_transfer::read(&params) {
                        if let Some(result) =
                            draft.append_cluster(&catalog, &sources, &transfer, language)
                        {
                            imported = result;
                            if storage.as_ref().is_some_and(|storage| {
                                serde_json::to_string(&draft.garden).is_ok_and(|data| {
                                    storage.set_item(garden::STORAGE_KEY, &data).is_ok()
                                })
                            }) {
                                cluster_transfer::consume(&params);
                            }
                        }
                    }
                    root.set_text_content(None);
                    leptos::mount::mount_to(root, move || view! {
                        <garden_page::GardenPage catalog=Arc::new(catalog) sources=Arc::new(sources)
                            draft guild=initial imported language />
                    }).forget();
                } else {
                    root.set_text_content(None);
                    leptos::mount::mount_to(root, move || {
                        view! {
                            <Planner catalog=Arc::new(catalog) initial language />
                        }
                    })
                    .forget();
                }
            }
            Err(error) => {
                show_load_error(&root, &error);
            }
        }
    });
    Ok(())
}

fn language(
    window: &web_sys::Window,
    storage: Option<&Storage>,
    params: &UrlSearchParams,
    embed: bool,
) -> &'static str {
    let normalize = |value: &str| match value.to_ascii_lowercase().split('-').next() {
        Some("de") => Some("de"),
        Some("en") => Some("en"),
        Some("nb" | "nn" | "no") => Some("nb"),
        _ => None,
    };
    if let Some(explicit) = params.get("lang").as_deref().and_then(normalize) {
        return explicit;
    }
    let saved = storage.and_then(|s| s.get_item(LANGUAGE_KEY).ok().flatten());
    let browser = window.navigator().language();
    let default = saved
        .as_deref()
        .and_then(normalize)
        .or_else(|| browser.as_deref().and_then(normalize))
        .unwrap_or("nb");
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let decode = |key| {
        params.get(key).and_then(|code| {
            URL_SAFE_NO_PAD
                .decode(code.trim().trim_end_matches('='))
                .ok()
        })
    };
    if embed || default != "nb" {
        if let Some(bytes) = decode("garden").filter(|bytes| bytes.len() >= 3) {
            return if bytes[0] & 128 != 0 { "en" } else { "de" };
        }
    }
    if default == "nb" {
        return default;
    }
    if let Some(bytes) = decode("g").filter(|bytes| bytes.len() >= 2) {
        return if bytes[1] & 16 != 0 { "en" } else { "de" };
    }
    default
}

fn show_load_error(root: &web_sys::HtmlElement, error: &JsValue) {
    web_sys::console::error_1(error);
    root.set_text_content(Some("Kunne ikke laste planleggeren. Last siden på nytt."));
}

async fn fetch_startup_data<T: serde::de::DeserializeOwned>(
    name: &str,
    preloaded: &js_sys::Map,
) -> Result<T, JsValue> {
    if let Ok(request) = preloaded
        .get(&JsValue::from_str(name))
        .dyn_into::<js_sys::Promise>()
    {
        if let Some(text) = JsFuture::from(request)
            .await
            .ok()
            .and_then(|value| value.as_string())
        {
            return parse_asset(&text);
        }
    }
    fetch_json(name).await
}

async fn fetch_json<T: serde::de::DeserializeOwned>(name: &str) -> Result<T, JsValue> {
    let response: Response = JsFuture::from(window().fetch_with_str(&routes::asset(name)))
        .await?
        .dyn_into()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!(
            "Catalogue request failed: {}",
            response.status()
        )));
    }
    let text = JsFuture::from(response.text()?)
        .await?
        .as_string()
        .ok_or_else(|| JsValue::from_str("Expected text in catalogue response"))?;
    parse_asset(&text)
}

fn parse_asset<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, JsValue> {
    serde_json::from_str(text).map_err(|error| JsValue::from_str(&error.to_string()))
}

fn update_guild(guild: RwSignal<Guild>, action: impl FnOnce(&mut Guild)) {
    guild.update(action);
    let data = guild.with_untracked(serde_json::to_string);
    if let (Ok(data), Ok(Some(storage))) = (data, window().local_storage()) {
        if storage.set_item(STORAGE_KEY, &data).is_ok() {
            let _ = routes::clean_guild_query();
        }
        guild.with_untracked(|state| {
            let _ = storage.set_item(SOIL_KEY, &state.soil_type);
            let _ = storage.set_item(ZONE_KEY, &state.climate_zone);
        });
    }
}

fn toggle_companion(guild: RwSignal<Guild>, catalog: &Catalog, plant: &Species, checked: bool) {
    if checked
        && (plant.retired
            || guild.with_untracked(|state| {
                state.plant_ids.contains(&plant.id)
                    || !catalog.guild_incompatibilities(plant, state).is_empty()
            }))
    {
        return;
    }
    update_guild(guild, |state| {
        if checked {
            state.plant_ids.push(plant.id.clone());
        } else {
            state.plant_ids.retain(|id| id != &plant.id);
        }
    });
}

#[component]
fn Planner(catalog: Arc<Catalog>, initial: Guild, language: &'static str) -> impl IntoView {
    let guild = RwSignal::new(initial);
    let detail_plant = RwSignal::new(None::<String>);
    let detail_dialog = NodeRef::<html::Dialog>::new();
    let open_details = Callback::new(move |id: String| {
        if let Some(dialog) = detail_dialog.get_untracked() {
            detail_plant.set(Some(id));
            if !dialog.open() {
                let _ = dialog.show_modal();
            }
        }
    });
    let cover_data = ground_cover_view::load();
    let search = RwSignal::new(String::new());
    let gap_filter = RwSignal::new(None::<seasonal::Filter>);
    let pest_filter = RwSignal::new(None::<String>);
    let role = RwSignal::new("ALL");
    let optimal_soil = RwSignal::new(false);
    let optimal_zone = RwSignal::new(true);
    let show_conflicts = RwSignal::new(false);
    let limit = RwSignal::new(PAGE_SIZE);
    let active_catalog = catalog.clone();
    let tree = Memo::new(move |_| {
        guild.with(|state| {
            active_catalog
                .trees
                .iter()
                .position(|tree| tree.id == state.tree_id)
        })
    });
    let defense_catalog = catalog.clone();
    let defense = Memo::new(move |_| {
        let tree = &defense_catalog.trees[tree.get()?];
        pest_filter.with(|pest| {
            guild.with(|state| {
                pest_defense::resolve(
                    &defense_catalog,
                    tree,
                    pest.as_deref()?,
                    &state.climate_zone,
                )
            })
        })
    });
    let available_catalog = catalog.clone();
    let available = Memo::new(move |_| {
        defense.with(|result| {
            result
                .as_ref()
                .map(|result| {
                    guild.with(|state| pest_defense::available(&available_catalog, state, result))
                })
                .unwrap_or_default()
        })
    });
    let select_pest = Callback::new(move |name: String| {
        gap_filter.set(None);
        role.set("ALL");
        search.set(String::new());
        pest_filter.set(Some(name));
        if let Some(section) = document().get_element_by_id("plant-catalog") {
            section.scroll_into_view();
        }
    });
    let filters = catalog.clone();
    let matches = Memo::new(move |_| {
        let query = search.with(|value| value.trim().to_lowercase());
        let matches_query = |plant: &Species| {
            query.is_empty()
                || plant
                    .names
                    .iter()
                    .any(|name| name.to_lowercase().contains(&query))
                || plant.botanical_name.to_lowercase().contains(&query)
                || plant
                    .roles
                    .iter()
                    .chain(plant.role_ids.iter())
                    .any(|role| role.to_lowercase().contains(&query))
        };
        let star = tree.get().map(|i| &filters.trees[i]);
        let gap = gap_filter.get();
        let pest = pest_filter.with(Option::is_some);
        let role = role.get();
        let optimal_soil = optimal_soil.get();
        let optimal_zone = optimal_zone.get();
        let show_conflicts = show_conflicts.get();
        guild.with(|state| {
            let candidates = if let Some(filter) = gap {
                seasonal::candidates(&filters, state, filter, true)
            } else if pest {
                defense.with(|result| {
                    result
                        .as_ref()
                        .map(|result| result.plants.clone())
                        .unwrap_or_default()
                })
            } else {
                (0..filters.plants.len()).collect()
            };
            let mut ids = Vec::new();
            let mut conflicting = 0;
            for index in candidates {
                let plant = &filters.plants[index];
                if plant.retired
                    || !matches_query(plant)
                    || (optimal_soil && !plant.suitable_soils.contains(&state.soil_type))
                    || (optimal_zone && !plant.climate_zones.contains(&state.climate_zone))
                {
                    continue;
                }
                if gap.is_none() && !pest {
                    let matches_role = match role {
                        "ALL" => true,
                        "RECOMMENDED" => star.is_some_and(|star| {
                            star.recommended_companions.contains(&plant.id)
                                || plant.recommended_for_trees.contains(&star.id)
                        }),
                        _ => plant.role_ids.iter().any(|value| value == role),
                    };
                    if !matches_role {
                        continue;
                    }
                }
                let selected = state.plant_ids.contains(&plant.id);
                let incompatible =
                    !selected && !filters.guild_incompatibilities(plant, state).is_empty();
                if incompatible {
                    conflicting += 1;
                }
                if !incompatible || show_conflicts {
                    ids.push((
                        index,
                        if selected {
                            0
                        } else if incompatible {
                            2
                        } else {
                            1
                        },
                    ));
                }
            }
            if gap.is_none() && !pest {
                ids.sort_by(|a, b| {
                    a.1.cmp(&b.1)
                        .then_with(|| filters.plants[a.0].name.cmp(&filters.plants[b.0].name))
                });
            }
            (
                ids.into_iter().map(|(i, _)| i).collect::<Vec<_>>(),
                conflicting,
            )
        })
    });
    let selected_catalog = catalog.clone();
    let chosen = Memo::new(move |_| {
        guild.with(|state| {
            selected_catalog
                .plants
                .iter()
                .enumerate()
                .filter(|(_, plant)| state.plant_ids.contains(&plant.id))
                .map(|(index, _)| index)
                .collect::<Vec<_>>()
        })
    });
    let tree_catalog = catalog.clone();
    let switch_catalog = catalog.clone();
    let notice = RwSignal::new(String::new());
    let clear_filters = Callback::new(move |()| {
        gap_filter.set(None);
        pest_filter.set(None);
    });
    let preset_catalog = catalog.clone();
    let select_preset = Callback::new(move |name: &'static str| {
        if let Some(preset) = preset_catalog.presets.get(name) {
            clear_filters.run(());
            role.set("ALL");
            update_guild(guild, |state| {
                state.plant_ids = preset.plant_ids.clone();
                notice.set(preset_catalog.set_tree(state, preset.tree_id.clone()));
            });
        }
    });
    let reset_catalog = catalog.clone();
    let reset_guild = move |_| {
        clear_filters.run(());
        role.set("ALL");
        search.set(String::new());
        update_guild(guild, |state| {
            state.plant_ids.clear();
            state.tree_id = reset_catalog
                .trees
                .first()
                .map(|tree| tree.id.clone())
                .unwrap_or_default();
        });
        notice.set(String::new());
    };
    let pdf_catalog = catalog.clone();
    let export_pdf = move |_| {
        let plan = guild.get_untracked();
        let saved = pdf_export::guild(&pdf_catalog, &plan, cover_data.get_untracked().as_deref())
            .is_some_and(|data| {
                download::bytes(
                    &data,
                    "application/pdf",
                    &format!("{}_plan.pdf", plan.tree_id),
                )
                .is_ok()
            });
        notice.set(pdf_catalog.text(if saved {
            "gardenPagePdfSuccess"
        } else {
            "appPdfError"
        }));
    };
    let calendar_catalog = catalog.clone();
    let export_calendar = move |_| {
        let catalog = calendar_catalog.clone();
        let plan = guild.get_untracked();
        spawn_local(async move {
            let saved =
                match fetch_json::<calendar::Data>(&format!("calendar-{language}.json")).await {
                    Ok(data) => calendar::guild_ics(
                        &catalog,
                        &data,
                        &plan,
                        language,
                        calendar_view::today(),
                        &calendar_view::timestamp(),
                    )
                    .is_some_and(|content| {
                        download::text(
                            &content,
                            "text/calendar;charset=utf-8",
                            &format!("{}_kalender.ics", plan.tree_id),
                        )
                        .is_ok()
                    }),
                    Err(_) => false,
                };
            notice.set(catalog.text(if saved {
                "exportCalendarSuccess"
            } else {
                "gardenPageCalendarError"
            }));
        });
    };
    let import_catalog = catalog.clone();
    let file_ref = NodeRef::<html::Input>::new();
    let export_catalog = catalog.clone();
    let export_json = move |_| {
        let timestamp = String::from(js_sys::Date::new_0().to_iso_string());
        let saved = guild.with_untracked(|state| {
            guild_io::export(&export_catalog, state, language, &timestamp).is_some_and(
                |(file, data)| download::text(&data, "application/json", &file).is_ok(),
            )
        });
        notice.set(if saved {
            String::new()
        } else {
            export_catalog.text("cadExportError")
        });
    };
    let import = move |_| {
        let Some(input) = file_ref.get() else { return };
        let Some(file) = input.files().and_then(|files| files.get(0)) else {
            return;
        };
        input.set_value("");
        let catalog = import_catalog.clone();
        spawn_local(async move {
            let result = JsFuture::from(file.text())
                .await
                .ok()
                .and_then(|value| value.as_string());
            let plan = result.as_deref().and_then(|text| {
                guild.with_untracked(|state| guild_io::import(&catalog, text, state))
            });
            match plan {
                Some(guild_io::ImportedPlan::Guild(imported)) => {
                    update_guild(guild, |state| *state = imported);
                    gap_filter.set(None);
                    pest_filter.set(None);
                    notice.set(catalog.text("importSuccess"));
                }
                Some(guild_io::ImportedPlan::Garden) => {
                    let storage = window().local_storage().ok().flatten();
                    let saved = storage.zip(result).is_some_and(|(storage, raw)| {
                        storage
                            .set_item("permaculture_garden_grid_v1", &raw)
                            .is_ok()
                    });
                    if !saved
                        || window()
                            .location()
                            .set_href(&format!("{}?lang={language}", routes::garden()))
                            .is_err()
                    {
                        notice.set(catalog.text("importInvalidFormat"));
                    }
                }
                None => notice.set(catalog.text("importInvalidFormat")),
            }
        });
    };
    let share_catalog = catalog.clone();
    let share = Callback::new(move |()| {
        guild.with_untracked(|state| {
            let origin = window().location().origin().unwrap_or_default();
            let code = state.encode(&share_catalog, language);
            let tree = share_catalog
                .trees
                .iter()
                .find(|tree| tree.id == state.tree_id)
                .map_or("", |tree| tree.name.as_str());
            share_dialog::Payload {
                url: format!("{origin}/share/{}/?g={code}&lang={language}", state.tree_id),
                embed: format!("{origin}/embed/?g={code}&lang={language}"),
                title: format!("{tree} {}", share_catalog.text("permacultureGuildLabel")),
                text: share_catalog
                    .text("shareGuildShareText")
                    .replace("{tree}", tree),
            }
        })
    });
    let share_preview_catalog = catalog.clone();
    let preview = Callback::new(move |()| {
        guild.with_untracked(|state| {
            embed::guild_card(&share_preview_catalog, state, language, false).into_any()
        })
    });
    let grid_catalog = catalog.clone();
    let filter_catalog = catalog.clone();
    let no_match_catalog = catalog.clone();
    let conflict_catalog = catalog.clone();
    let conflict_label = catalog.clone();
    Effect::new(move |_| {
        if gap_filter.get().is_some() {
            pest_filter.set(None);
        }
    });
    Effect::new(move |_| {
        gap_filter.track();
        pest_filter.track();
        role.track();
        optimal_soil.track();
        optimal_zone.track();
        show_conflicts.track();
        limit.set(PAGE_SIZE);
    });

    let chosen_catalog = catalog.clone();
    let climates = CLIMATES
        .into_iter()
        .map(|(value, label)| {
            view! {
                <option value=value>{catalog.text(label)}</option>
            }
        })
        .collect_view();
    view! {
        <header class="site-header">
            <a class="brand" href=format!("{}?lang={language}", routes::planner())><span class="brand-mark" aria-hidden="true"><svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7 20h10"/><path d="M10 20c5.5-2.5.8-6.4 3-10"/><path d="M9.5 9.4c1.1.8 1.8 2.2 2.3 3.7-2 .4-3.5.4-4.8-.3-1.2-.6-2.3-1.9-3-4.2 2.8-.5 4.4 0 5.5.8z"/><path d="M14.1 6a7 7 0 0 0-1.1 4c1.9-.1 3.3-.6 4.3-1.4 1-1 1.6-2.3 1.7-4.6-2.7.1-4 1-4.9 2z"/></svg></span><span>"pflanzengilde"<b>".de"</b></span></a>
            <nav class="site-nav">
                <a href=format!("{}?lang={language}", routes::planner()) aria-current=if true { Some("page") } else { None }>{catalog.text("navPlanner")}</a>
                <a href=format!("{}?lang={language}", routes::garden()) aria-current=if false { Some("page") } else { None }>{catalog.text("navGarden")}</a>
                <a href=format!("/guides/?lang={language}")>{catalog.text("navGuides")}</a>
            </nav>
            <share_dialog::ShareDialog catalog=catalog.clone() language make_payload=share preview />
            <routes::LanguageSelect language />
        </header>
        <main>
            <h1>{catalog.text("pdfTitle")}</h1>
            <section class="panel">
                <header class="panel-head">
                    <div>
                        <h2>{catalog.text("soilBarTitle")}</h2>
                        <p>{catalog.text("soilBarSubtitle")}</p>
                    </div>
                </header>
                <div class="settings site-controls">
                    <label>{catalog.text("climateZone")}
                        <select prop:value=move || guild.with(|s| s.climate_zone.clone())
                            on:change=move |ev| update_guild(guild, |s| s.climate_zone = event_target_value(&ev))>
                            {climates}
                        </select>
                    </label>
                    <select aria-label="Hemisphere" prop:value=move || guild.with(|s| s.hemisphere.clone())
                        on:change=move |ev| update_guild(guild, |s| s.hemisphere = event_target_value(&ev))>
                        <option value="NORTHERN">{catalog.text("northHemisphere")}</option>
                        <option value="SOUTHERN">{catalog.text("southHemisphere")}</option>
                    </select>
                    <climate_map::ClimateMap catalog=catalog.clone() language
                        selection=Signal::derive(move || guild.with(|state|
                            (state.climate_zone.clone(), state.hemisphere == "SOUTHERN")))
                        on_select=Callback::new(move |(zone, southern): (String, bool)| {
                            update_guild(guild, |state| {
                                state.climate_zone = zone;
                                state.hemisphere = if southern { "SOUTHERN" } else { "NORTHERN" }.into();
                            });
                        }) />
                </div>
                <pickers::SoilPicker catalog=catalog.clone() guild
                    on_select=Callback::new(move |soil: String| update_guild(guild, |s| s.soil_type = soil)) />
            </section>
            <section class="panel">
                <header class="panel-head">
                    <div>
                        <h2>{catalog.text("step1Title")}</h2>
                        <p>{catalog.text("step1Subtitle")}</p>
                    </div>
                    <div class="settings guild-template-actions">
                        <catalog_controls::Presets catalog=catalog.clone() on_select=select_preset />
                        <button type="button" on:click=reset_guild>{catalog.text("resetGuild")}</button>
                    </div>
                </header>
                <pickers::TreePicker catalog=catalog.clone() guild language
                    on_select=Callback::new(move |id: String| {
                        update_guild(guild, |state| notice.set(switch_catalog.set_tree(state, id)));
                        gap_filter.set(None);
                        pest_filter.set(None);
                    }) />
                {move || tree.get().map(|index| view! { <SpeciesSummary species=tree_catalog.trees[index].clone() /> })}
                <pest_view::PestChoices catalog=catalog.clone() guild tree selected=pest_filter on_select=select_pest />
            </section>
            <div class="settings plan-actions">
                <label>{catalog.text("importPlan")}
                    <input type="file" accept=".json,application/json" node_ref=file_ref on:change=import />
                </label>
                <button type="button" on:click=export_json>{catalog.text("exportJson")}</button>
                <button type="button" on:click=export_calendar>{catalog.text("exportCalendar")}</button>
                <button type="button" on:click=export_pdf>{catalog.text("exportPdf")}</button>
            </div>
            <div role="status" class="notice">{notice}</div>
            <plan::GuildPlan catalog=catalog.clone() guild cover_data language on_select=open_details />
            <calendar_view::SeasonalCalendar catalog=catalog.clone() guild filter=gap_filter />
            <guild_cards::RoleOverview catalog=catalog.clone() chosen language />
            <guild_cards::GuildNotices catalog=catalog.clone() guild language />
            <section class="panel">
                <h2>{catalog.text("pdfPlantInventory")}</h2>
                <ul class="plants">
                    <For each=move || chosen.get() key=|index| *index
                        children=move |index| view! {
                            <PlantRow plant=chosen_catalog.plants[index].clone() guild catalog=chosen_catalog.clone() on_details=open_details />
                        } />
                </ul>
            </section>
            <section id="plant-catalog" class="panel">
                <h2>{catalog.text("step2Title")}</h2>
                <p>{catalog.text("step2Subtitle")}</p>
                <pest_view::PestAdvice catalog=catalog.clone() selected=pest_filter defense available language />
                <div class="notice" hidden=move || gap_filter.with(Option::is_none)>
                    {move || gap_filter.with(|filter| filter.as_ref().map(|filter| format!("{} · {}",
                        filter_catalog.text(&format!("role:{}",filter.role)),
                        filter_catalog.text(filter.season.label()))))}
                    " "
                    <button type="button" on:click=move |_|gap_filter.set(None)>
                        {catalog.text("clearGapFilter")}
                    </button>
                </div>
                <label class="search">{catalog.text("searchPlaceholder")}
                    <input type="search" prop:value=search
                        on:input=move |ev| { search.set(event_target_value(&ev)); limit.set(PAGE_SIZE); } />
                </label>
                <catalog_controls::CatalogControls catalog=catalog.clone() role soil=optimal_soil
                    zone=optimal_zone on_change=clear_filters />
                <div class="catalog-conflicts" hidden=move || matches.with(|(_, count)| *count == 0) && !show_conflicts.get()>
                    <span hidden=show_conflicts>{move || matches.with(|(_, count)| {
                        let name = tree.get().map(|i| conflict_catalog.trees[i].name.as_str()).unwrap_or_default();
                        conflict_catalog.text("compatHiddenNoteGuild").replace("{count}", &count.to_string()).replace("{star}", name)
                    })}</span>
                    <button type="button" aria-pressed=move || show_conflicts.get().to_string()
                        on:click=move |_| show_conflicts.update(|open| *open = !*open)>
                        {move || conflict_label.text(if show_conflicts.get() { "compatHideHidden" } else { "compatShowHidden" })}
                    </button>
                </div>
                <p hidden=move || matches.with(|(ids, _)| !ids.is_empty())>
                    {move || no_match_catalog.text(if gap_filter.with(Option::is_some) {"noGapPlantsFound"}
                        else if pest_filter.with(Option::is_some) {"noPestCompanionsFound"} else {"noPlantsMatched"})}
                </p>
                <ul class="plants">
                    <For each=move || matches.with(|(ids, _)| ids.iter().take(limit.get()).copied().collect::<Vec<_>>())
                        key=|index| *index children=move |index| view! {
                            <PlantRow plant=grid_catalog.plants[index].clone() guild catalog=grid_catalog.clone() on_details=open_details />
                        } />
                </ul>
                <button type="button" hidden=move || matches.with(|(ids, _)| ids.len() <= limit.get())
                    on:click=move |_| limit.set(usize::MAX)>
                    {catalog.text("showAll")}
                </button>
            </section>
        </main>
        <plant_details::PlantDetails catalog=catalog.clone() guild selected=detail_plant dialog=detail_dialog language />
        <routes::SiteFooter catalog=catalog.clone() language />
    }
}

#[component]
fn SpeciesSummary(species: Species) -> impl IntoView {
    view! {
        <div class="species">
            <img src=species.image_url width="64" height="64" alt="" loading="lazy" />
            <div><strong>{species.name}</strong><br/><i>{species.botanical_name}</i></div>
        </div>
        <p>{species.description}</p>
    }
}

#[component]
fn PlantRow(
    plant: Species,
    guild: RwSignal<Guild>,
    catalog: Arc<Catalog>,
    on_details: Callback<String>,
) -> impl IntoView {
    let plant = Arc::new(plant);
    let toggle = plant.clone();
    let selected = plant.clone();
    let growing_conditions = plant.clone();
    let details = catalog.text("details");
    let details_id = plant.id.clone();
    let toggle_catalog = catalog.clone();
    let warning_catalog = catalog.clone();
    let warning_plant = plant.clone();
    let warnings = Memo::new(move |_| {
        guild.with(|state| warning_catalog.guild_incompatibilities(&warning_plant, state))
    });
    let disabled_plant = plant.clone();
    let disable = move || {
        guild.with(|state| {
            !state.plant_ids.contains(&disabled_plant.id)
                && (disabled_plant.retired || !warnings.with(Vec::is_empty))
        })
    };
    view! {
        <li>
            <label class="species">
                <input type="checkbox"
                    prop:checked=move || guild.with(|state| state.plant_ids.contains(&selected.id))
                    disabled=disable
                    on:change=move |event| toggle_companion(guild, &toggle_catalog, &toggle, event_target_checked(&event)) />
                <img src=plant.image_url.clone() width="48" height="48" alt="" loading="lazy" />
                <span><strong>{plant.name.clone()}</strong><br/><i>{plant.botanical_name.clone()}</i></span>
            </label>
            <p>{plant.roles.join(" · ")}</p>
            <p class="warning" hidden=move || warnings.with(Vec::is_empty)>
                {move || warnings.with(|messages| messages.join(" · "))}
            </p>
            <p class="warning" hidden=move || guild.with(|state| {
                !growing_conditions.unsuitable_soils.contains(&state.soil_type)
            })>{catalog.text("soilUnsuitable")}</p>
            <button type="button" aria-haspopup="dialog" on:click=move |_| on_details.run(details_id.clone())>{details}</button>
        </li>
    }
}
