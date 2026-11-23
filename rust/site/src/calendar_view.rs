use crate::{
    calendar::{Date, Season},
    catalog::{Catalog, Species},
    guild::Guild,
};
use leptos::prelude::*;
use std::sync::Arc;

pub fn today() -> Date {
    let date = js_sys::Date::new_0();
    Date {
        year: date.get_full_year(),
        month: (date.get_month() + 1) as u8,
        day: date.get_date() as u8,
    }
}

pub fn timestamp() -> String {
    let iso = String::from(js_sys::Date::new_0().to_iso_string());
    format!(
        "{}Z",
        iso.split('.')
            .next()
            .unwrap_or_default()
            .replace(['-', ':'], "")
    )
}

#[component]
pub fn SeasonalCalendar(
    catalog: Arc<Catalog>,
    guild: RwSignal<Guild>,
    filter: RwSignal<Option<crate::seasonal::Filter>>,
) -> impl IntoView {
    let season = RwSignal::new(Season::LateSpring);
    let labels = catalog.clone();
    let buttons = Season::ALL.into_iter().map(|item| view! {
        <button type="button" aria-pressed=move || if season.get() == item { "true" } else { "false" }
            on:click=move |_| season.set(item)>
            {labels.text(item.label())}
        </button>
    }).collect_view();
    let months = catalog.clone();
    let items = catalog.clone();
    let planting_label = catalog.text("calendarPlanting");
    let harvest_label = catalog.text("calendarCatHarvest");
    let bloom_label = catalog.text("bloom");
    let empty = catalog.text("noPlantChosenYet");
    view! {
        <section class="panel">
            <h2>{catalog.text("calendarTitle")}</h2>
            <p>{catalog.text("calendarSubtitle")}</p>
            <div class="grid-tools">{buttons}</div>
            <p>{move || {
                let southern = guild.with(|guild| guild.hemisphere == "SOUTHERN");
                let key = format!("months:{}:{}", season.get().label(), if southern { "south" } else { "north" });
                months.text(&key)
            }}</p>
            {move || {
                let active = season.get();
                let selected: Vec<String> = guild.with(|guild| {
                    items.trees.iter().filter(|tree| tree.id == guild.tree_id)
                        .chain(items.plants.iter().filter(|plant| guild.plant_ids.contains(&plant.id)))
                        .map(|plant| plant.id.clone()).collect()
                });
                let activities: Vec<_> = [
                    (planting_label.clone(), 0), (bloom_label.clone(), 1),
                    ("Chop & Drop".into(), 2), (harvest_label.clone(), 3),
                ].into_iter().map(|(label, kind)| {
                    let includes = |species: &Species| match kind {
                        0 => species.activity.planting.contains(&active), 1 => species.activity.flowering.contains(&active),
                        2 => species.activity.chop.contains(&active), _ => species.activity.harvest.contains(&active),
                    };
                    let names = items.trees.iter().chain(items.plants.iter())
                        .filter(|plant| selected.contains(&plant.id) && includes(plant))
                        .map(|plant| plant.name.clone()).collect::<Vec<_>>();
                    view! {
                        <div class="season-activity">
                            <dt>{label}</dt>
                            <dd>{if names.is_empty() { "–".into() } else { names.join(" · ") }}</dd>
                        </div>
                    }
                }).collect();
                if selected.is_empty() { view! { <p>{empty.clone()}</p> }.into_any() }
                else { view! { <dl class="season-activities">{activities}</dl> }.into_any() }
            }}
            <GuildCoverage catalog=catalog.clone() guild filter />
        </section>
    }
}

#[component]
fn GuildCoverage(
    catalog: Arc<Catalog>,
    guild: RwSignal<Guild>,
    filter: RwSignal<Option<crate::seasonal::Filter>>,
) -> impl IntoView {
    use crate::seasonal;
    let coverage_catalog = catalog.clone();
    let coverage =
        Memo::new(move |_| guild.with(|state| seasonal::coverage(&coverage_catalog, state)));
    let gap_catalog = catalog.clone();
    let gaps = Memo::new(move |_| {
        guild.with(|state| coverage.with(|rows| seasonal::gaps(&gap_catalog, state, rows, true)))
    });
    let overlap_catalog = catalog.clone();
    let redundancy =
        Memo::new(move |_| guild.with(|state| seasonal::redundant(&overlap_catalog, state, true)));
    let table_catalog = catalog.clone();
    let gap_text = catalog.clone();
    let overlap_text = catalog.clone();
    let columns = Season::ALL
        .into_iter()
        .map(|season| view! {<th scope="col">{catalog.text(season.label())}</th>})
        .collect_view();
    view! {
        <div class="role-coverage">
            <table>
                <thead><tr><th scope="col">{catalog.text("functionalRole")}</th>{columns}</tr></thead>
                <tbody>
                    {move || coverage.with(|rows|rows.iter().map(|row| {
                        let key=table_catalog.text(&format!("role:{}",row.role));
                        let cells=row.across.iter().map(|matches| {
                            let names=matches.iter().map(|&i|table_catalog.plants[i].name.as_str()).collect::<Vec<_>>().join(" · ");
                            view! {<td>{if names.is_empty() {"–".into()}else{names}}</td>}
                        }).collect_view();
                        view! {<tr><th scope="row">{key}</th>{cells}</tr>}
                    }).collect_view())}
                </tbody>
            </table>
        </div>
        <div hidden=move || gaps.with(Vec::is_empty)>
            <h3>{catalog.text("gapsDetectedTitle")}</h3>
            <div class="season-reports">
                {move || gaps.with(|gaps|gaps.iter().map(|gap| {
                    let current=gap.filter;
                    let heading=format!("{} · {}",gap_text.text(current.season.label()),gap_text.text(&format!("role:{}",current.role)));
                    let reason=gap_text.text(&format!("gap:{}:{}",current.role,current.season.slug()));
                    let recommendations=gap.suggestions.iter().take(3).map(|&i| {
                        let species=&gap_text.plants[i];
                        let id=species.id.clone();
                        let label=species.name.clone();
                        let text=gap_text.text("seasonalCalendarAddToFillGap").replace("{plant}",&label)
                            .replace("{season}",&gap_text.text(current.season.label()));
                        let data=gap_text.clone();
                        view! {
                            <button type="button" aria-label=text on:click=move |_|{
                                crate::update_guild(guild, |state| {
                                    if !state.plant_ids.contains(&id) && data.plants[i].suitable(state) &&
                                        data.guild_incompatibilities(&data.plants[i],state).is_empty() {
                                        state.plant_ids.push(id.clone());
                                    }
                                });
                                if filter.get_untracked()==Some(current) {filter.set(None);}
                            }> "+ " {label} </button>
                        }
                    }).collect_view();
                    view! {
                        <article class="season-activity gap-report">
                            <h4>{heading}</h4>
                            <p>{reason}</p>
                            <p>{gap_text.text("recommendedAllies")}</p>
                            <div class="grid-tools">{recommendations}</div>
                            <button type="button" on:click=move |_| {
                                filter.set(Some(current));
                                if let Some(element)=document().get_element_by_id("plant-catalog") {
                                    element.scroll_into_view();
                                }
                            }>{gap_text.text("filterGapAction")}</button>
                        </article>
                    }
                }).collect_view())}
            </div>
        </div>
        <div hidden=move || redundancy.with(Vec::is_empty)>
            <h3>{catalog.text("redundantSectionTitle")}</h3>
            <p>{catalog.text("redundantSectionSub")}</p>
            <div class="season-reports">
                {move || redundancy.with(|reports|reports.iter().map(|report| {
                    let species=&overlap_text.plants[report.plant];
                    let name=species.name.clone();
                    let id=species.id.clone();
                    let covered=report.covering.iter().map(|&i|overlap_text.plants[i].name.as_str()).collect::<Vec<_>>().join(" · ");
                    let swaps=report.alternatives.iter().map(|&i| {
                        let name=overlap_text.plants[i].name.clone();
                        let text=overlap_text.text("seasonalCalendarSwapWith").replace("{plant}",&species.name).replace("{candidate}",&name);
                        let old=species.id.clone();
                        let data=overlap_text.clone();
                        view! { <button type="button" aria-label=text on:click=move |_| {
                            crate::update_guild(guild,|state| {
                                let Some(index)=state.plant_ids.iter().position(|id|id==&old) else{return;};
                                let plant=&data.plants[i];
                                let mut remaining=state.clone();
                                remaining.plant_ids.remove(index);
                                if plant.suitable(&remaining)&&!remaining.plant_ids.contains(&plant.id)&&
                                    data.guild_incompatibilities(plant,&remaining).is_empty() {
                                    state.plant_ids[index].clone_from(&plant.id);
                                }
                            });
                        }>{name}</button> }
                    }).collect_view();
                    view! {
                        <article class="season-activity">
                            <h4>{name}</h4>
                            <p>{overlap_text.text("redundantCoveredByLabel")} " " {covered}</p>
                            {(!report.alternatives.is_empty()).then(||view! {
                                <p>{overlap_text.text("redundantSwapForUncovered")}</p>
                                <div class="grid-tools">{swaps}</div>
                            })}
                            <button type="button" on:click=move |_|crate::update_guild(guild,|state|state.plant_ids.retain(|candidate|candidate!=&id))>
                                {overlap_text.text("redundantRemoveAction")}
                            </button>
                        </article>
                    }
                }).collect_view())}
            </div>
        </div>
    }
}
