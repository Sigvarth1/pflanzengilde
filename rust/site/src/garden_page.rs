use crate::{
    calendar::Season,
    catalog::Catalog,
    cluster,
    cluster_transfer::{Transfer, APPLIED},
    garden::{Bed, Garden, GuildTemplate, Settings, Shape, Star, STORAGE_KEY},
    garden_code, garden_conflicts,
    garden_editor::CompanionEditor,
    garden_exports, garden_layout,
    garden_map::{translate_shape, GardenMap},
    garden_resolution,
    garden_warnings::GardenWarnings,
    ground_cover::{self, Cover},
    ground_cover_view::{self, DisplayControls},
    guild::{Guild, CLIMATES, SOILS},
    guild_io,
    infrastructure::{Form, Point, Target, Tool},
};
use leptos::{html, prelude::*};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use wasm_bindgen_futures::{spawn_local, JsFuture};

#[derive(Clone)]
pub struct Draft {
    pub(crate) garden: Garden,
    settings: Settings,
}

impl Draft {
    pub fn load(
        catalog: &Catalog,
        sources: &Map<String, Value>,
        saved: Option<&str>,
        guild: &Guild,
        language: &str,
        code: Option<&str>,
    ) -> (Self, bool) {
        let mut stored = Garden::saved(saved);
        let templates = std::mem::take(&mut stored.loaded_guild_templates);
        let shared = code.and_then(|code| garden_code::decode(catalog, code));
        let shared_plan = shared.is_some();
        let (mut garden, mut settings) = shared
            .map(|(mut garden, settings)| {
                garden.garden_id = format!(
                    "s{}",
                    crate::calendar::stable_hash(code.unwrap_or_default())
                );
                garden.auto_resolve_enabled = stored.auto_resolve_enabled;
                (garden, settings)
            })
            .unwrap_or_else(|| {
                let text = |key| {
                    stored
                        .other
                        .get(key)
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                };
                let settings = Settings {
                    soil: text("soil"),
                    climate: text("zone"),
                    southern: text("hemisphere").as_deref().unwrap_or(&guild.hemisphere)
                        == "SOUTHERN",
                    english: language == "en",
                    established: text("treeAge").as_deref().unwrap_or(&guild.tree_age)
                        == "ESTABLISHED",
                };
                (stored, settings)
            });
        settings.soil.get_or_insert_with(|| guild.soil_type.clone());
        settings
            .climate
            .get_or_insert_with(|| guild.climate_zone.clone());
        settings.english = language == "en";
        if garden.garden_id.is_empty() {
            garden.garden_id = garden_id();
        }
        if garden.garden_name.trim().is_empty() {
            garden.garden_name = catalog.text("gardenPageDefaultName");
        }
        for star in &mut garden.star_plants {
            star.with_source(sources);
        }
        let (templates, templates_changed) = initial_templates(
            catalog,
            sources,
            &garden.star_plants,
            shared_plan,
            guild,
            templates,
        );
        garden.loaded_guild_templates = templates;
        let mut draft = Self { garden, settings };
        draft.update_settings_fields();
        (draft, templates_changed)
    }

    fn update_settings_fields(&mut self) {
        if let Some(soil) = &self.settings.soil {
            self.garden.other.insert("soil".into(), json!(soil));
        }
        if let Some(climate) = &self.settings.climate {
            self.garden.other.insert("zone".into(), json!(climate));
        }
        self.garden.other.insert(
            "hemisphere".into(),
            json!(if self.settings.southern {
                "SOUTHERN"
            } else {
                "NORTHERN"
            }),
        );
        self.garden.other.insert(
            "treeAge".into(),
            json!(if self.settings.established {
                "ESTABLISHED"
            } else {
                "YOUNG"
            }),
        );
        if self.garden.other.contains_key("name") {
            self.garden
                .other
                .insert("name".into(), json!(self.garden.garden_name));
        }
    }

    pub fn append_cluster(
        &mut self,
        catalog: &Catalog,
        sources: &Map<String, Value>,
        transfer: &Transfer,
        language: &str,
    ) -> Option<(Vec<String>, String)> {
        let species = catalog
            .trees
            .iter()
            .find(|tree| tree.id == transfer.guild.tree_id)?;
        if self.garden.other.get(APPLIED).and_then(Value::as_str) == Some(transfer.ticket.as_str())
        {
            return Some((Vec::new(), String::new()));
        }
        let was_empty = self.garden.star_plants.is_empty();
        let config = transfer.config;
        let mut stars =
            cluster::stars(species, config, &transfer.guild.plant_ids, &transfer.ticket);
        let _ = cluster::place(catalog, &self.garden.star_plants, &mut stars, 0.5, 0.0);
        self.settings = cluster::settings(&transfer.guild, language);
        for star in &mut stars {
            star.with_source(sources);
        }
        let ids: Vec<_> = stars.iter().map(|star| star.instance_id.clone()).collect();
        let key = if was_empty {
            "gardenPagePendingPlaced"
        } else {
            "gardenPagePendingPlacedBeside"
        };
        let message = catalog
            .text(key)
            .replace("{count}", &ids.len().to_string())
            .replace("{name}", &species.name);
        self.garden.star_plants.extend(stars);
        self.garden
            .other
            .insert(APPLIED.into(), json!(transfer.ticket));
        self.update_settings_fields();
        Some((ids, message))
    }

    pub fn persist_templates(&self, storage: &web_sys::Storage, saved: Option<&str>) {
        let mut data: Map<String, Value> = saved
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default();
        if let Ok(templates) = serde_json::to_value(&self.garden.loaded_guild_templates) {
            if data.get("loadedGuildTemplates") != Some(&templates) {
                // Opening a shared URL can add templates without replacing the local garden.
                data.insert("loadedGuildTemplates".into(), templates);
                if let Ok(raw) = serde_json::to_string(&data) {
                    let _ = storage.set_item(STORAGE_KEY, &raw);
                }
            }
        }
    }
}

fn initial_templates(
    catalog: &Catalog,
    sources: &Map<String, Value>,
    stars: &[Star],
    shared: bool,
    guild: &Guild,
    stored: Vec<GuildTemplate>,
) -> (Vec<GuildTemplate>, bool) {
    let mut templates = Vec::new();
    let mut changed = false;
    let timestamp = String::from(js_sys::Date::new_0().to_iso_string());
    let mut insert = |id: String, label: &str, tree_id: &str, selected_plant_ids: Vec<String>| {
        if templates.iter().any(|entry: &GuildTemplate| entry.id == id) {
            return;
        }
        if let Some(existing) = stored.iter().find(|entry| {
            entry.id == id
                && entry.tree_id() == Some(tree_id)
                && entry.selected_plant_ids == selected_plant_ids
        }) {
            templates.push(existing.clone());
            return;
        }
        if let (Some(tree), Some(source)) = (
            catalog.trees.iter().find(|tree| tree.id == tree_id),
            sources.get(tree_id),
        ) {
            changed = true;
            templates.push(GuildTemplate {
                id,
                source_name: catalog.text(label).replace("{name}", &tree.name),
                star_tree: source.clone(),
                selected_plant_ids,
                imported_at: timestamp.clone(),
            });
        }
    };
    if shared {
        for star in stars {
            insert(
                format!("tmpl-shared-{}", star.tree_id),
                "gardenPageSharedSource",
                &star.tree_id,
                star.selected_plant_ids.clone(),
            );
        }
    }
    let planner_plants = if guild.plant_ids.is_empty() {
        catalog
            .trees
            .iter()
            .find(|tree| tree.id == guild.tree_id)
            .map(|tree| {
                tree.recommended_companions
                    .iter()
                    .take(5)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    } else {
        guild.plant_ids.clone()
    };
    insert(
        format!("tmpl-planner-{}", guild.tree_id),
        "gardenPagePlannerSource",
        &guild.tree_id,
        planner_plants,
    );
    for entry in stored {
        if !templates.iter().any(|template| template.id == entry.id) {
            templates.push(entry);
        }
    }
    (templates, changed)
}

fn garden_id() -> String {
    format!(
        "{:x}-{:x}",
        js_sys::Date::now() as u64,
        (js_sys::Math::random() * 1e9) as u32
    )
}

pub(crate) fn save(draft: RwSignal<Draft>, action: impl FnOnce(&mut Draft)) {
    draft.update(|draft| {
        action(draft);
        draft.update_settings_fields();
    });
    let persisted = if let (Ok(Some(storage)), Ok(state)) = (
        window().local_storage(),
        draft.with_untracked(|draft| serde_json::to_string(&draft.garden)),
    ) {
        storage.set_item(STORAGE_KEY, &state).is_ok()
    } else {
        false
    };
    if !persisted {
        return;
    }
    // Edits belong to the local draft. Reloading it must no longer replay the original shared link.
    if let Ok(location) = window().location().search() {
        if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&location) {
            if params.has("garden") {
                let _ = crate::routes::clean_garden_query(&params, "garden");
            }
        }
    }
}

fn text(language: &str, words: [&'static str; 3]) -> &'static str {
    words[match language {
        "de" => 2,
        "en" => 1,
        _ => 0,
    }]
}

#[component]
pub fn GardenPage(
    catalog: Arc<Catalog>,
    sources: Arc<Map<String, Value>>,
    draft: Draft,
    guild: Guild,
    imported: (Vec<String>, String),
    language: &'static str,
) -> impl IntoView {
    let draft = RwSignal::new(draft);
    let garden_inputs = Memo::new(move |_| {
        draft.with(|state| {
            (
                state.garden.star_plants.clone(),
                state.garden.infrastructure.clone(),
                state.settings.southern,
            )
        })
    });
    let layout_catalog = catalog.clone();
    let layout_sources = sources.clone();
    let resolution_inputs = Memo::new(move |_| {
        draft.with(|state| {
            (
                state.settings.soil.clone(),
                state.settings.climate.clone(),
                state.garden.auto_resolve_enabled,
                state.garden.pinned_companions.clone(),
            )
        })
    });
    let resolution = Memo::new(move |_| {
        garden_inputs.track();
        resolution_inputs.track();
        draft.with_untracked(|state| {
            garden_resolution::resolve(
                &layout_catalog,
                &state.garden,
                &state.settings,
                &layout_sources,
            )
        })
    });
    let companions = Memo::new(move |_| resolution.with(|resolved| resolved.companions.clone()));
    let shade_catalog = catalog.clone();
    let shade = Memo::new(move |_| {
        garden_inputs.with(|(stars, _, _)| garden_conflicts::shade(&shade_catalog, stars))
    });
    let cover_data = ground_cover_view::load();
    let cover_catalog = catalog.clone();
    let cover_settings = Memo::new(move |_| {
        draft.with(|state| (state.settings.climate.clone(), state.settings.established))
    });
    let covers = Memo::new(move |_| {
        cover_data.with(|data| {
            let Some(data) = data.as_ref() else {
                return Vec::new();
            };
            cover_settings.with(|(climate, established)| {
                garden_inputs.with(|(trees, infrastructure, southern)| {
                    companions.with(|positions| {
                        let settings = Settings {
                            soil: None,
                            climate: climate.clone(),
                            southern: *southern,
                            english: false,
                            established: *established,
                        };
                        ground_cover::garden(
                            data,
                            &cover_catalog,
                            trees,
                            positions,
                            infrastructure,
                            &settings,
                        )
                    })
                })
            })
        })
    });
    let show_covers = RwSignal::new(true);
    let cover_season = RwSignal::new(None::<Season>);
    let list_catalog = catalog.clone();
    let counts = Memo::new(move |_| {
        companions.with(|plants| {
            let mut counts = std::collections::BTreeMap::<&str, usize>::new();
            for companion in plants {
                *counts.entry(&companion.plant_id).or_default() += 1;
            }
            let mut counts: Vec<_> = list_catalog
                .plants
                .iter()
                .filter_map(|plant| {
                    let count = counts.get(plant.id.as_str())?;
                    let quantity = covers.with(|covers| {
                        covers
                            .iter()
                            .find(|cover| cover.plant_id == plant.id)
                            .map_or_else(
                                || format!("× {count}"),
                                |cover| {
                                    format!(
                                        "· {} m²",
                                        crate::drawing::number(cover.area_m2, 1, language)
                                    )
                                },
                            )
                    });
                    Some((plant.name.clone(), quantity))
                })
                .collect();
            counts.sort_by(|a, b| a.0.cmp(&b.0));
            counts
        })
    });
    let selected = RwSignal::new(imported.0.first().cloned().unwrap_or_default());
    let group = RwSignal::new(imported.0);
    let focused_plant = RwSignal::new(String::new());
    let center = RwSignal::new(None::<Point>);
    let tool = RwSignal::new(None::<Tool>);
    let picker = RwSignal::new(guild.tree_id.clone());
    let zoom = RwSignal::new(1.0_f64);
    let guild_for_import = guild.clone();
    let catalog_for_add = catalog.clone();
    let sources_for_add = sources.clone();
    let add = move |_| {
        let choice = picker.get_untracked();
        let (tree_id, template_plants) = if let Some(id) = choice.strip_prefix("template:") {
            let template = draft.with_untracked(|state| {
                let template = state
                    .garden
                    .loaded_guild_templates
                    .iter()
                    .find(|item| item.id == id)?;
                Some((
                    template.tree_id()?.to_owned(),
                    Some(template.selected_plant_ids.clone()),
                ))
            });
            let Some(template) = template else {
                return;
            };
            template
        } else {
            (choice, None)
        };
        let Some(tree) = catalog_for_add.trees.iter().find(|tree| tree.id == tree_id) else {
            return;
        };
        let ids = template_plants.unwrap_or_else(|| {
            if tree_id == guild.tree_id && !guild.plant_ids.is_empty() {
                guild.plant_ids.clone()
            } else {
                tree.recommended_companions.clone()
            }
        });
        let instance = instance_id();
        save(draft, |state| {
            let x = state
                .garden
                .star_plants
                .last()
                .map_or(0.0, |last| (last.x_m + 4.0).clamp(-300.0, 300.0));
            let y = state.garden.star_plants.last().map_or(0.0, |last| last.y_m);
            let mut star = Star::new(tree, instance.clone(), x, y, ids);
            star.with_source(&sources_for_add);
            state.garden.star_plants.push(star);
        });
        selected.set(instance.clone());
        group.set(vec![instance]);
    };
    let import_catalog = catalog.clone();
    let import_sources = sources.clone();
    let import_guild = guild_for_import;
    let file = NodeRef::<html::Input>::new();
    let notice = RwSignal::new(imported.1);
    let cleared = catalog.text("gardenPageCleared");
    let reset = move |_| {
        save(draft, |state| {
            state.garden.garden_id = garden_id();
            state.garden.star_plants.clear();
            state.garden.infrastructure = Default::default();
            state.garden.pinned_companions.clear();
        });
        selected.set(String::new());
        group.set(Vec::new());
        focused_plant.set(String::new());
        tool.set(None);
        notice.set(cleared.clone());
    };
    let import_file = Callback::new(move |(file, point): (web_sys::File, Point)| {
        let catalog = import_catalog.clone();
        let sources = import_sources.clone();
        let guild = import_guild.clone();
        let garden_id = draft.with_untracked(|state| state.garden.garden_id.clone());
        let filename = file.name();
        spawn_local(async move {
            let raw = JsFuture::from(file.text())
                .await
                .ok()
                .and_then(|value| value.as_string());
            if draft.with_untracked(|state| state.garden.garden_id != garden_id) {
                return;
            }
            let Some(mut imported) = raw
                .as_deref()
                .and_then(|raw| import_plan(&catalog, &sources, &guild, raw))
            else {
                notice.set(catalog.text("gardenPageInvalidJson"));
                return;
            };
            for tree in &mut imported.star_plants {
                tree.x_m = ((tree.x_m + point[0]).clamp(-300.0, 300.0) * 100.0).round() / 100.0;
                tree.y_m = ((tree.y_m + point[1]).clamp(-300.0, 300.0) * 100.0).round() / 100.0;
            }
            if point != [0.0; 2] {
                if let Some(outline) = &mut imported.infrastructure.outline {
                    translate_shape(outline, point);
                }
                for bed in &mut imported.infrastructure.raised_beds {
                    translate_shape(&mut bed.shape, point);
                }
            }
            let count = imported.star_plants.len();
            let focus = imported
                .star_plants
                .first()
                .map(|tree| tree.instance_id.clone());
            let source_name = filename
                .rsplit_once('.')
                .filter(|(_, extension)| extension.eq_ignore_ascii_case("json"))
                .map_or(filename.as_str(), |(stem, _)| stem)
                .trim();
            let timestamp = String::from(js_sys::Date::new_0().to_iso_string());
            let mut templates: Vec<_> = imported
                .star_plants
                .iter()
                .filter_map(|star| {
                    let tree = catalog.trees.iter().find(|tree| tree.id == star.tree_id)?;
                    Some(GuildTemplate {
                        id: format!("tmpl-{}", star.instance_id),
                        source_name: if source_name.is_empty() {
                            tree.name.clone()
                        } else {
                            source_name.into()
                        },
                        star_tree: sources.get(&star.tree_id)?.clone(),
                        selected_plant_ids: crate::garden_resolution::selected_ids(star, tree),
                        imported_at: timestamp.clone(),
                    })
                })
                .collect();
            save(draft, |state| {
                if state.garden.infrastructure.outline.is_none()
                    && state.garden.infrastructure.raised_beds.is_empty()
                {
                    state.garden.infrastructure = imported.infrastructure;
                }
                state.garden.star_plants.append(&mut imported.star_plants);
                templates.append(&mut state.garden.loaded_guild_templates);
                state.garden.loaded_guild_templates = templates;
            });
            if let Some(focus) = focus {
                selected.set(focus.clone());
                group.set(vec![focus]);
            }
            notice.set(
                catalog
                    .text("gardenPageImportedStars")
                    .replace("{count}", &count.to_string()),
            );
        });
    });
    let on_import = move |_| {
        let Some(input) = file.get() else { return };
        let Some(file) = input.files().and_then(|files| files.get(0)) else {
            return;
        };
        input.set_value("");
        import_file.run((file, [0.0; 2]));
    };
    let catalog_for_share = catalog.clone();
    let share = Callback::new(move |()| {
        draft.with_untracked(|state| {
            let origin = window().location().origin().unwrap_or_default();
            let code = garden_code::encode(&catalog_for_share, &state.garden, &state.settings);
            let title = format!("{} • Pflanzengilde.de", state.garden.garden_name);
            let text = catalog_for_share
                .text("shareGardenShareText")
                .replace("{stars}", &state.garden.star_plants.len().to_string())
                .replace(
                    "{companions}",
                    &companions.with_untracked(Vec::len).to_string(),
                );
            crate::share_dialog::Payload {
                url: format!("{origin}/garten/?garden={code}&lang={language}"),
                embed: format!("{origin}/embed/?garden={code}&lang={language}"),
                title,
                text,
            }
        })
    });
    let share_preview_catalog = catalog.clone();
    let preview = Callback::new(move |()| {
        draft.with_untracked(|state| {
            companions.with_untracked(|plants| {
                crate::embed::garden_card(
                    &share_preview_catalog,
                    &state.garden,
                    &state.settings,
                    plants,
                    None,
                    language,
                )
                .into_any()
            })
        })
    });
    let export_catalog = catalog.clone();
    let export = move |_| {
        let (plan, positions) = (draft.get_untracked(), companions.get_untracked());
        let catalog = export_catalog.clone();
        spawn_local(async move {
            match crate::fetch_json::<Map<String, Value>>("garden-plant-data.json").await {
                Ok(plants) => {
                    if export_garden(&plan, &positions, &plants, language).is_err() {
                        notice.set(catalog.text("gardenPageInvalidJson"));
                    }
                }
                Err(_) => notice.set(catalog.text("gardenPageInvalidJson")),
            }
        });
    };
    let calendar_catalog = catalog.clone();
    let export_calendar = move |_| {
        let catalog = calendar_catalog.clone();
        let (plan, positions) = (draft.get_untracked(), companions.get_untracked());
        spawn_local(async move {
            let saved = match crate::fetch_json::<crate::calendar::Data>(&format!(
                "calendar-{language}.json"
            ))
            .await
            {
                Ok(data) => {
                    let calendar = crate::calendar::garden_ics(
                        &catalog,
                        &data,
                        &plan.garden,
                        &plan.settings,
                        &positions,
                        crate::calendar_view::today().year,
                    );
                    crate::download::text(
                        &calendar,
                        "text/calendar;charset=utf-8",
                        "hageplan_kalender.ics",
                    )
                    .is_ok()
                }
                Err(_) => false,
            };
            notice.set(catalog.text(if saved {
                "exportCalendarSuccess"
            } else {
                "gardenPageCalendarError"
            }));
        });
    };
    let draw_catalog = catalog.clone();
    let pdf_catalog = catalog.clone();
    let export_pdf = move |_| {
        let plan = draft.get_untracked();
        let positions = companions.get_untracked();
        let areas = covers.get_untracked();
        let drawing =
            crate::drawing::Drawing::new(&pdf_catalog, &plan.garden, &positions, &areas, language);
        if drawing.empty() {
            notice.set(pdf_catalog.text("cadExportEmpty"));
            return;
        }
        let data = shade.with_untracked(|shades| {
            resolution.with_untracked(|result| {
                crate::pdf_export::garden(
                    &pdf_catalog,
                    &drawing,
                    &plan.settings,
                    &result.unresolved,
                    shades,
                    language,
                )
            })
        });
        let saved = crate::download::bytes(
            &data,
            "application/pdf",
            &file_name(&plan.garden.garden_name, "pdf"),
        )
        .is_ok();
        notice.set(pdf_catalog.text(if saved {
            "gardenPagePdfSuccess"
        } else {
            "gardenPagePdfError"
        }));
    };
    let draw = move |kind: &'static str| {
        let plan = draft.get_untracked();
        let positions = companions.get_untracked();
        let areas = covers.get_untracked();
        let drawing =
            crate::drawing::Drawing::new(&draw_catalog, &plan.garden, &positions, &areas, language);
        if drawing.empty() {
            notice.set(draw_catalog.text("cadExportEmpty"));
            return;
        }
        let (mime, data) = match kind {
            "csv" => (
                "text/csv;charset=utf-8",
                Some(garden_exports::csv(&draw_catalog, &drawing, language)),
            ),
            "svg" => (
                "image/svg+xml",
                Some(garden_exports::svg(&draw_catalog, &drawing, language)),
            ),
            _ => (
                "application/dxf",
                Some(crate::dxf_export::create(&draw_catalog, &drawing, language)),
            ),
        };
        let saved = data.is_some_and(|data| {
            crate::download::text(&data, mime, &file_name(&plan.garden.garden_name, kind)).is_ok()
        });
        notice.set(
            draw_catalog
                .text(if saved {
                    "cadExportSuccess"
                } else {
                    "cadExportError"
                })
                .replace("{format}", &kind.to_ascii_uppercase()),
        );
    };
    let csv = draw.clone();
    let svg = draw.clone();
    let dxf = draw;
    let catalog_for_list = catalog.clone();
    let list = Memo::new(move |_| {
        draft.with(|state| {
            state
                .garden
                .star_plants
                .iter()
                .map(|star| star.instance_id.clone())
                .collect::<Vec<_>>()
        })
    });
    let rows = move |instance: String| {
        view! {
            <StarRow id=instance draft selected group catalog=catalog_for_list.clone() language />
        }
    };
    let opts = catalog
        .trees
        .iter()
        .map(|tree| view! { <option value=tree.id.clone()>{tree.name.clone()}</option> })
        .collect_view();
    let template_catalog = catalog.clone();
    let template_options = Memo::new(move |_| {
        draft.with(|state| {
            state
                .garden
                .loaded_guild_templates
                .iter()
                .filter_map(|template| {
                    let id = template.tree_id()?;
                    let tree = template_catalog.trees.iter().find(|tree| tree.id == id)?;
                    let source = if template.id.starts_with("tmpl-shared-") {
                        template_catalog
                            .text("gardenPageSharedSource")
                            .replace("{name}", &tree.name)
                    } else if template.id.starts_with("tmpl-planner-") {
                        template_catalog
                            .text("gardenPagePlannerSource")
                            .replace("{name}", &tree.name)
                    } else {
                        format!("{} · {}", tree.name, template.source_name)
                    };
                    let count = if template.selected_plant_ids.is_empty() {
                        tree.recommended_companions.len()
                    } else {
                        template.selected_plant_ids.len()
                    };
                    let companions = template_catalog.text("gardenSidebarCompanions");
                    Some((
                        format!("template:{}", template.id),
                        format!("{source} · {count} {companions}"),
                    ))
                })
                .collect::<Vec<_>>()
        })
    });
    let soils = SOILS
        .into_iter()
        .map(|(value, label)| view! { <option value=value>{catalog.text(label)}</option> })
        .collect_view();
    let climates = CLIMATES
        .into_iter()
        .map(|(value, label)| view! { <option value=value>{catalog.text(label)}</option> })
        .collect_view();
    let name_hint = text(language, ["Hagens navn", "Garden name", "Gartenname"]);
    view! {
        <header class="site-header">
            <a class="brand" href=format!("{}?lang={language}", crate::routes::planner())><span class="brand-mark" aria-hidden="true"><svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7 20h10"/><path d="M10 20c5.5-2.5.8-6.4 3-10"/><path d="M9.5 9.4c1.1.8 1.8 2.2 2.3 3.7-2 .4-3.5.4-4.8-.3-1.2-.6-2.3-1.9-3-4.2 2.8-.5 4.4 0 5.5.8z"/><path d="M14.1 6a7 7 0 0 0-1.1 4c1.9-.1 3.3-.6 4.3-1.4 1-1 1.6-2.3 1.7-4.6-2.7.1-4 1-4.9 2z"/></svg></span><span>"pflanzengilde"<b>".de"</b></span></a>
            <nav class="site-nav">
                <a href=format!("{}?lang={language}", crate::routes::planner()) aria-current=if false { Some("page") } else { None }>{catalog.text("navPlanner")}</a>
                <a href=format!("{}?lang={language}", crate::routes::garden()) aria-current=if true { Some("page") } else { None }>{catalog.text("navGarden")}</a>
                <a href=format!("/guides/?lang={language}")>{catalog.text("navGuides")}</a>
            </nav>
            <crate::share_dialog::ShareDialog catalog=catalog.clone() language garden=true make_payload=share preview />
            <crate::routes::LanguageSelect language />
        </header>
        <main>
            <h1>{catalog.text("gardenGridTitle")}</h1>
            <section class="panel settings">
                <label>{name_hint}
                    <input prop:value=move || draft.with(|state| state.garden.garden_name.clone())
                        on:change=move |ev| save(draft, |state| state.garden.garden_name = event_target_value(&ev)) />
                </label>
                <label>{catalog.text("soilType")}
                    <select prop:value=move || draft.with(|state| state.settings.soil.clone().unwrap_or_default())
                        on:change=move |ev| save(draft, |state| state.settings.soil = Some(event_target_value(&ev)))>
                        {soils}
                    </select>
                </label>
                <label>{catalog.text("climateZone")}
                    <select prop:value=move || draft.with(|state| state.settings.climate.clone().unwrap_or_default())
                        on:change=move |ev| save(draft, |state| state.settings.climate = Some(event_target_value(&ev)))>
                        {climates}
                    </select>
                </label>
                <select aria-label="Hemisphere" prop:value=move || draft.with(|state| if state.settings.southern { "SOUTHERN" } else { "NORTHERN" })
                    on:change=move |ev| save(draft, |state| state.settings.southern = event_target_value(&ev) == "SOUTHERN")>
                    <option value="NORTHERN">{catalog.text("northHemisphere")}</option>
                    <option value="SOUTHERN">{catalog.text("southHemisphere")}</option>
                </select>
                <label>{catalog.text("treeAgeLabel")}
                    <select prop:value=move || draft.with(|state| if state.settings.established { "ESTABLISHED" } else { "YOUNG" })
                        on:change=move |ev| save(draft, |state| state.settings.established = event_target_value(&ev) == "ESTABLISHED")>
                        <option value="YOUNG">{catalog.text("treeAgeYoung")}</option>
                        <option value="ESTABLISHED">{catalog.text("treeAgeEstablished")}</option>
                    </select>
                </label>
                <crate::climate_map::ClimateMap catalog=catalog.clone() language
                    selection=Signal::derive(move || draft.with(|state|
                        (state.settings.climate.clone().unwrap_or_default(), state.settings.southern)))
                    on_select=Callback::new(move |(zone, southern): (String, bool)| {
                        save(draft, |state| {
                            state.settings.climate = Some(zone);
                            state.settings.southern = southern;
                        });
                    }) />
            </section>
            <section class="panel">
                <DisplayControls catalog=catalog.clone() active=show_covers season=cover_season />
                <div class="grid-tools">
                    <button type="button" on:click=move |_| zoom.update(|value| *value = (*value * 1.3).min(4.0))
                        aria-label=catalog.text("gardenGridZoomIn")>"+"</button>
                    <button type="button" on:click=move |_| zoom.update(|value| *value = (*value / 1.3).max(0.25))
                        aria-label=catalog.text("gardenGridZoomOut")>"−"</button>
                    <button type="button" on:click=move |_| { zoom.set(1.0); center.set(None); selected.set(String::new()); group.set(Vec::new()); }>
                        {catalog.text("gardenGridCenterView")}
                    </button>
                </div>
                <GardenMap catalog=catalog.clone() draft selected group focused_plant zoom center companions resolution shade covers show_covers cover_season tool notice language on_import=import_file />
            </section>
            <GardenWarnings catalog=catalog.clone() draft resolution shade notice language />
            <InfrastructureControls catalog=catalog.clone() draft tool language />
            <section class="panel">
                <h2>{catalog.text("gardenSidebarActiveStarPlants")}</h2>
                <div class="settings garden-tree-picker">
                    <label>{catalog.text("gardenSidebarAddStarPlant")}
                        <select prop:value=picker on:change=move |ev| picker.set(event_target_value(&ev))>
                            <optgroup label=text(language, ["Plantekatalog", "Plant catalogue", "Pflanzenkatalog"])>{opts}</optgroup>
                            <optgroup label=text(language, ["Laugsmaler", "Guild templates", "Gildenvorlagen"])
                                hidden=move || template_options.with(Vec::is_empty)>
                                <For each=move || template_options.get() key=Clone::clone children=move |(value, label)| {
                                    view! { <option value=value>{label}</option> }
                                } />
                            </optgroup>
                        </select>
                    </label>
                    <button type="button" on:click=add>{catalog.text("gardenSidebarAddStarPlant")}</button>
                </div>
                <p hidden=move || list.with(|ids| !ids.is_empty())>{catalog.text("gardenSidebarNoStarPlants")}</p>
                <ul class="garden-stars"><For each=move || list.get() key=String::clone children=rows /></ul>
            </section>
            <section class="panel">
                <h2>{catalog.text("gardenSidebarActiveCompanions")}</h2>
                <p hidden=move || companions.with(|plants| !plants.is_empty())>{catalog.text("gardenSidebarNoCompanions")}</p>
                <ul class="garden-companions">
                    <For each=move || counts.get() key=|item| item.clone() children=move |(name, quantity)| view! {
                        <li>{name} " " {quantity}</li>
                    } />
                </ul>
            </section>
            <CompanionEditor catalog=catalog.clone() draft selected focused_plant resolution notice language />
            <div role="status" class="notice">{notice}</div>
            <section class="panel settings">
                <label>{catalog.text("importPlan")}
                    <input type="file" accept=".json,application/json" node_ref=file on:change=on_import />
                </label>
                <button type="button" on:click=export>{catalog.text("exportJson")}</button>
                <button type="button" on:click=export_calendar>{catalog.text("exportCalendar")}</button>
                <button type="button" on:click=export_pdf>{catalog.text("exportPdf")}</button>
                <button type="button" on:click=move |_| csv("csv")>{catalog.text("navbarExportCsv")}</button>
                <button type="button" on:click=move |_| svg("svg")>{catalog.text("navbarExportSvg")}</button>
                <button type="button" on:click=move |_| dxf("dxf")>{catalog.text("navbarExportDxf")}</button>
                <button type="button" on:click=reset>{text(language, ["Tøm hageplanen", "Clear garden plan", "Gartenplan leeren"])}</button>
            </section>
            <GeoExport catalog=catalog.clone() draft companions covers notice language />
        </main>
        <crate::routes::SiteFooter catalog=catalog.clone() language />
    }
}

#[component]
fn StarRow(
    id: String,
    draft: RwSignal<Draft>,
    selected: RwSignal<String>,
    group: RwSignal<Vec<String>>,
    catalog: Arc<Catalog>,
    language: &'static str,
) -> impl IntoView {
    let query = id.clone();
    let star = Memo::new(move |_| {
        draft.with(|state| {
            state
                .garden
                .star_plants
                .iter()
                .find(|tree| tree.instance_id == query)
                .cloned()
                .map(|tree| (tree.tree_id, tree.custom_name, tree.x_m, tree.y_m))
        })
    });
    let labels = catalog.clone();
    let select_id = id.clone();
    let active_id = id.clone();
    let x_id = id.clone();
    let y_id = id.clone();
    let name_id = id.clone();
    let duplicate_id = id.clone();
    let duplicate_catalog = catalog.clone();
    let on_x = move |ev| edit_coordinate(draft, &x_id, true, &event_target_value(&ev));
    let on_y = move |ev| edit_coordinate(draft, &y_id, false, &event_target_value(&ev));
    view! {
        <li class="garden-star settings" class:selected=move || selected.with(|current| current == &active_id) || group.with(|ids| ids.contains(&active_id))>
            <button type="button" on:click=move |_| { selected.set(select_id.clone()); group.set(vec![select_id.clone()]); }>
                {move || star.with(|star| star.as_ref().map(|(tree, custom, _, _)|
                    custom.clone().or_else(|| labels.trees.iter().find(|species| &species.id == tree).map(|species| species.name.clone()))
                    .unwrap_or_else(|| tree.clone())))}
            </button>
            <label>{text(language, ["Navn på tre", "Tree name", "Baumname"])}
                <input type="text" maxlength="64"
                    prop:value=move || star.with(|star| star.as_ref().and_then(|entry| entry.1.clone()).unwrap_or_default())
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        let name: String = value.trim().chars().take(64).collect();
                        save(draft, |state| {
                            if let Some(tree) = state.garden.star_plants.iter_mut().find(|tree| tree.instance_id == name_id) {
                                tree.custom_name = (!name.is_empty()).then_some(name);
                            }
                        });
                    } />
            </label>
            <label>"X (m)"<input type="number" step="0.25" min="-300" max="300"
                prop:value=move || star.with(|star| star.as_ref().map(|entry| entry.2.to_string()).unwrap_or_default())
                on:change=on_x /></label>
            <label>"Y (m)"<input type="number" step="0.25" min="-300" max="300"
                prop:value=move || star.with(|star| star.as_ref().map(|entry| entry.3.to_string()).unwrap_or_default())
                on:change=on_y /></label>
            <button type="button" on:click=move |_| {
                let existing = draft.with_untracked(|state| state.garden.star_plants.iter().find(|tree| tree.instance_id == duplicate_id).cloned());
                if let Some(mut copy) = existing {
                    let radius = duplicate_catalog.trees.iter().find(|item| item.id == copy.tree_id).map_or(1.0, |item| item.mature_radius_m);
                    copy.x_m = ((copy.x_m + radius * 1.8).clamp(-300.0, 300.0) * 10.0).round() / 10.0;
                    copy.instance_id = instance_id();
                    let id = copy.instance_id.clone();
                    save(draft, |state| state.garden.star_plants.push(copy));
                    selected.set(id.clone());
                    group.set(vec![id]);
                }
            }>{text(language, ["Dupliser", "Duplicate", "Duplizieren"])}</button>
            <button type="button" on:click=move |_| {
                save(draft, |state| state.garden.remove(&id));
                group.update(|group| group.retain(|item| item != &id));
                if selected.with_untracked(|current| current == &id) {
                    selected.set(group.with_untracked(|group| group.first().cloned().unwrap_or_default()));
                }
            }>{catalog.text("gardenSidebarRemove")}</button>
        </li>
    }
}

fn export_garden(
    draft: &Draft,
    positions: &[garden_layout::Companion],
    plants: &Map<String, Value>,
    language: &str,
) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsValue;
    let companions: Vec<_> = positions
        .iter()
        .filter_map(|entry| {
            let plant = plants.get(&entry.plant_id)?;
            let mut data = serde_json::to_value(entry).ok()?;
            data["plant"] = plant.clone();
            Some(data)
        })
        .collect();
    let mut data = serde_json::to_value(&draft.garden)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    if let Some(data) = data.as_object_mut() {
        data.remove("loadedGuildTemplates");
    }
    data["name"] = json!(draft.garden.garden_name);
    data["version"] = json!("2.0");
    data["language"] = json!(language);
    data["createdAt"] = json!(String::from(js_sys::Date::new_0().to_iso_string()));
    data["placedCompanions"] = json!(companions);
    data["gridBoundsM"] = json!({"minX": -20, "maxX": 20, "minY": -20, "maxY": 20});
    let text = serde_json::to_string_pretty(&data)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    crate::download::text(&text, "application/json", "garden_plan.json")
}

pub(crate) fn instance_id() -> String {
    format!(
        "tree-{:x}-{:x}",
        js_sys::Date::now() as u64,
        (js_sys::Math::random() * 1e9) as u32
    )
}

fn import_plan(
    catalog: &Catalog,
    sources: &Map<String, Value>,
    current_guild: &Guild,
    raw: &str,
) -> Option<Garden> {
    let mut loaded = match guild_io::import(catalog, raw, current_guild)? {
        guild_io::ImportedPlan::Garden => Garden::saved(Some(raw)),
        guild_io::ImportedPlan::Guild(guild) => {
            let tree = catalog.trees.iter().find(|tree| tree.id == guild.tree_id)?;
            Garden {
                star_plants: vec![Star::new(tree, String::new(), 0.0, 0.0, guild.plant_ids)],
                ..Garden::default()
            }
        }
    };
    loaded.star_plants.retain_mut(|tree| {
        if !catalog
            .trees
            .iter()
            .any(|species| species.id == tree.tree_id)
        {
            return false;
        }
        tree.instance_id = instance_id();
        tree.with_source(sources);
        true
    });
    (!loaded.star_plants.is_empty()).then_some(loaded)
}

fn edit_coordinate(draft: RwSignal<Draft>, id: &str, x: bool, value: &str) {
    let Ok(value) = value.parse::<f64>() else {
        return;
    };
    if !value.is_finite() {
        return;
    }
    save(draft, |state| {
        if let Some(tree) = state
            .garden
            .star_plants
            .iter_mut()
            .find(|tree| tree.instance_id == id)
        {
            let coordinate = if x { &mut tree.x_m } else { &mut tree.y_m };
            *coordinate = value.clamp(-300.0, 300.0);
        }
    });
}

pub(crate) fn commit_shape(draft: RwSignal<Draft>, target: Target, shape: Shape) {
    save(draft, |draft| {
        let infrastructure = &mut draft.garden.infrastructure;
        match target {
            Target::Outline => infrastructure.outline = Some(shape),
            Target::NewBed if infrastructure.raised_beds.len() < 64 => {
                let id = (1..)
                    .map(|n| format!("bed-{n}"))
                    .find(|id| !infrastructure.raised_beds.iter().any(|bed| &bed.id == id))
                    .expect("finite list of bed IDs");
                infrastructure.raised_beds.push(Bed {
                    id,
                    shape,
                    height_m: 0.45,
                    name: None,
                });
            }
            Target::Bed(id) => {
                if let Some(bed) = infrastructure
                    .raised_beds
                    .iter_mut()
                    .find(|bed| bed.id == id)
                {
                    bed.shape = shape;
                }
            }
            Target::NewBed => {}
        }
    });
}

#[component]
fn DrawingButtons(
    catalog: Arc<Catalog>,
    tool: RwSignal<Option<Tool>>,
    target: Target,
) -> impl IntoView {
    [
        (Form::Rectangle, "infraRect", "infraRectHint"),
        (Form::Circle, "infraCircle", "infraCircleHint"),
        (Form::Free, "infraLasso", "infraLassoHint"),
    ].into_iter().map(|(form, text, hint)| {
        let choice = Tool { form, target: target.clone() };
        let compare = choice.clone();
        view! {
            <button type="button" title=catalog.text(hint)
                aria-pressed=move || if tool.with(|value| value.as_ref() == Some(&compare)) { "true" } else { "false" }
                on:click=move |_| {
                    if tool.with_untracked(|value| value.as_ref() == Some(&choice)) { tool.set(None); }
                    else { tool.set(Some(choice.clone())); }
                }>
                {catalog.text(text)}
            </button>
        }
    }).collect_view()
}

#[component]
fn InfrastructureControls(
    catalog: Arc<Catalog>,
    draft: RwSignal<Draft>,
    tool: RwSignal<Option<Tool>>,
    language: &'static str,
) -> impl IntoView {
    let beds = Memo::new(move |_| {
        draft.with(|state| {
            state
                .garden
                .infrastructure
                .raised_beds
                .iter()
                .map(|bed| bed.id.clone())
                .collect::<Vec<_>>()
        })
    });
    let has_outline =
        Memo::new(move |_| draft.with(|state| state.garden.infrastructure.outline.is_some()));
    let list_catalog = catalog.clone();
    let instructions = catalog.clone();
    view! {
        <section class="panel" aria-label=catalog.text("infraTitle")>
            <h2>{catalog.text("infraTitle")}</h2>
            <fieldset>
                <legend>{catalog.text("infraGardenShape")}</legend>
                <div class="grid-tools">
                    <DrawingButtons catalog=catalog.clone() tool target=Target::Outline />
                    <button type="button" hidden=move || !has_outline.get() on:click=move |_| {
                        if tool.with_untracked(|tool| tool.as_ref().is_some_and(|tool| tool.target == Target::Outline)) {
                            tool.set(None);
                        }
                        save(draft, |state| state.garden.infrastructure.outline = None);
                    }>{catalog.text("infraDelete")}</button>
                </div>
                <p hidden=move || has_outline.get()>{catalog.text("infraNoOutline")}</p>
            </fieldset>
            <fieldset>
                <legend>{catalog.text("infraRaisedBeds")}</legend>
                <div class="grid-tools" hidden=move || beds.with(|ids| ids.len() >= 64)>
                    <DrawingButtons catalog=catalog.clone() tool target=Target::NewBed />
                </div>
                <p hidden=move || beds.with(|ids| !ids.is_empty())>{catalog.text("infraNoBeds")}</p>
                <ul class="garden-stars">
                    <For each=move || beds.get() key=String::clone children=move |id| view! {
                        <BedControls catalog=list_catalog.clone() draft tool id language />
                    } />
                </ul>
            </fieldset>
            <button type="button" hidden=move || tool.with(Option::is_none) on:click=move |_| tool.set(None)>
                {catalog.text("geoRefCancel")}
            </button>
            <p role="status" hidden=move || tool.with(Option::is_none)>
                {move || tool.with(|tool| tool.as_ref().map(|tool| instructions.text(match tool.form {
                    Form::Rectangle => "infraRectHint", Form::Circle => "infraCircleHint", Form::Free => "infraLassoHint"
                })))}
            </p>
        </section>
    }
}

#[component]
fn BedControls(
    catalog: Arc<Catalog>,
    draft: RwSignal<Draft>,
    tool: RwSignal<Option<Tool>>,
    id: String,
    language: &'static str,
) -> impl IntoView {
    let lookup = id.clone();
    let change = id.clone();
    let replace = id.clone();
    let label = text(language, ["Høybed", "Raised bed", "Hochbeet"]);
    let values = Memo::new(move |_| {
        draft.with(|state| {
            state
                .garden
                .infrastructure
                .raised_beds
                .iter()
                .position(|bed| bed.id == lookup)
                .map(|i| {
                    let bed = &state.garden.infrastructure.raised_beds[i];
                    (
                        bed.name
                            .clone()
                            .unwrap_or_else(|| format!("{label} {}", i + 1)),
                        bed.height_m,
                    )
                })
        })
    });
    view! {
        <li class="garden-star settings">
            <strong>{move || values.with(|value| value.as_ref().map(|value| value.0.clone()))}</strong>
            <label>{catalog.text("infraHeight")}
                <input type="number" min="0.05" max="2" step="0.05" prop:value=move || values.with(|value| value.as_ref().map(|value| value.1.to_string()).unwrap_or_default())
                    on:change=move |ev| {
                        let Ok(height) = event_target_value(&ev).parse::<f64>() else { return; };
                        if !height.is_finite() { return; }
                        save(draft, |state| {
                            if let Some(bed) = state.garden.infrastructure.raised_beds.iter_mut().find(|bed| bed.id == change) {
                                bed.height_m = height.clamp(0.05, 2.0);
                            }
                        });
                    } />
            </label>
            <DrawingButtons catalog=catalog.clone() tool target=Target::Bed(replace) />
            <button type="button" on:click=move |_| {
                if tool.with_untracked(|tool| tool.as_ref().is_some_and(|tool| tool.target == Target::Bed(id.clone()))) {
                    tool.set(None);
                }
                save(draft, |state| state.garden.infrastructure.raised_beds.retain(|bed| bed.id != id));
            }>{catalog.text("infraDelete")}</button>
        </li>
    }
}

fn file_name(name: &str, extension: &str) -> String {
    let mut cleaned = String::new();
    for ch in name.trim().chars().take(96) {
        if ch.is_alphanumeric() || ch == '-' {
            cleaned.extend(ch.to_lowercase());
        } else if !cleaned.ends_with('_') {
            cleaned.push('_');
        }
    }
    let stem = cleaned.trim_matches('_');
    format!(
        "{}.{extension}",
        if stem.is_empty() { "garden-plan" } else { stem }
    )
}

fn coord(input: &str) -> Option<f64> {
    let value = input.trim().trim_end_matches('°').trim().replace(',', ".");
    value.parse::<f64>().ok().filter(|value| value.is_finite())
}

const GEO_KEY: &str = "pflanzengilde-geo-reference";

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeoInput {
    anchor_id: String,
    lat: f64,
    lon: f64,
}

fn geo_preferences() -> Map<String, Value> {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(GEO_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

#[component]
fn GeoExport(
    catalog: Arc<Catalog>,
    draft: RwSignal<Draft>,
    companions: Memo<Vec<garden_layout::Companion>>,
    covers: Memo<Vec<Cover>>,
    notice: RwSignal<String>,
    language: &'static str,
) -> impl IntoView {
    let visible = RwSignal::new(false);
    let lat = RwSignal::new(String::new());
    let lon = RwSignal::new(String::new());
    let anchor = RwSignal::new("origin".to_owned());
    let refs_catalog = catalog.clone();
    let refs = Memo::new(move |_| {
        draft.with(|draft| {
            draft
                .garden
                .star_plants
                .iter()
                .filter_map(|tree| {
                    let species = refs_catalog
                        .trees
                        .iter()
                        .find(|species| species.id == tree.tree_id)?;
                    let name = tree.custom_name.as_deref().unwrap_or(&species.name);
                    Some((
                        tree.instance_id.clone(),
                        refs_catalog
                            .text("geoRefAnchorTree")
                            .replace("{name}", name),
                    ))
                })
                .collect::<Vec<_>>()
        })
    });
    let opening_catalog = catalog.clone();
    let open = move |_| {
        let plan = draft.get_untracked();
        if plan.garden.star_plants.is_empty()
            && plan.garden.infrastructure.outline.is_none()
            && plan.garden.infrastructure.raised_beds.is_empty()
        {
            notice.set(opening_catalog.text("cadExportEmpty"));
            return;
        }
        if let Some(initial) = geo_preferences()
            .remove(&plan.garden.garden_id)
            .and_then(|value| serde_json::from_value::<GeoInput>(value).ok())
        {
            lat.set(initial.lat.to_string());
            lon.set(initial.lon.to_string());
            anchor.set(
                if refs.with_untracked(|refs| refs.iter().any(|(id, _)| id == &initial.anchor_id)) {
                    initial.anchor_id
                } else {
                    "origin".into()
                },
            );
        }
        visible.set(true);
    };
    let export_catalog = catalog.clone();
    let export = move |event: web_sys::SubmitEvent| {
        event.prevent_default();
        let plan = draft.get_untracked();
        let positions = companions.get_untracked();
        let areas = covers.get_untracked();
        let drawing = crate::drawing::Drawing::new(
            &export_catalog,
            &plan.garden,
            &positions,
            &areas,
            language,
        );
        let anchor_id = anchor.get_untracked();
        let point = drawing
            .trees
            .iter()
            .find(|tree| tree.id == anchor_id)
            .map_or([0.0; 2], |tree| tree.point);
        let (Some(latitude), Some(longitude)) =
            (coord(&lat.get_untracked()), coord(&lon.get_untracked()))
        else {
            notice.set(export_catalog.text("geoRefInvalid"));
            return;
        };
        let reference = garden_exports::Reference {
            point,
            lat: latitude,
            lon: longitude,
        };
        if !reference.valid() {
            notice.set(export_catalog.text("geoRefInvalid"));
            return;
        }
        let stamp = String::from(js_sys::Date::new_0().to_iso_string());
        let data = garden_exports::geojson(&drawing, &reference, &stamp);
        if crate::download::text(
            &data,
            "application/geo+json",
            &file_name(drawing.name, "geojson"),
        )
        .is_err()
        {
            notice.set(export_catalog.text("cadExportError"));
            return;
        }
        let mut preferences = geo_preferences();
        preferences.insert(
            plan.garden.garden_id,
            json!(GeoInput {
                anchor_id,
                lat: latitude,
                lon: longitude
            }),
        );
        if let Ok(Some(storage)) = window().local_storage() {
            let _ = storage.set_item(GEO_KEY, &Value::Object(preferences).to_string());
        }
        visible.set(false);
        notice.set(
            export_catalog
                .text("cadExportSuccess")
                .replace("{format}", "GeoJSON"),
        );
    };
    view! {
        <section class="panel">
            <button type="button" hidden=move || visible.get() on:click=open>{catalog.text("navbarExportGeoJson")}</button>
            <form hidden=move || !visible.get() on:submit=export>
                <h2>{catalog.text("geoRefTitle")}</h2>
                <p>{catalog.text("geoRefIntro")}</p>
                <div class="settings">
                    <label>{catalog.text("geoRefAnchor")}
                        <select prop:value=anchor on:change=move |ev| anchor.set(event_target_value(&ev))>
                            <option value="origin">{catalog.text("geoRefAnchorOrigin")}</option>
                            <For each=move || refs.get() key=|item| item.clone() children=move |(id, title)| {
                                view! { <option value=id>{title}</option> }
                            } />
                        </select>
                    </label>
                    <label>{catalog.text("geoRefLat")}
                        <input type="text" inputmode="decimal" required prop:value=lat on:input=move |ev| {
                            let data = event_target_value(&ev);
                            if let Some((a,b)) = data.split_once(';').or_else(|| data.split_once(", ")) {
                                lat.set(a.trim().to_owned()); lon.set(b.trim().to_owned());
                            } else { lat.set(data); }
                        } />
                    </label>
                    <label>{catalog.text("geoRefLon")}
                        <input type="text" inputmode="decimal" required prop:value=lon on:input=move |ev| lon.set(event_target_value(&ev)) />
                    </label>
                    <button type="submit">{catalog.text("geoRefExport")}</button>
                    <button type="button" on:click=move |_| visible.set(false)>{catalog.text("geoRefCancel")}</button>
                </div>
            </form>
        </section>
    }
}
