use crate::{
    catalog::Catalog,
    drawing::number,
    guild::Guild,
    guild_antagonisms::{self, Notice, Scope},
    pest_defense::Citation,
    placement::place,
    seasonal::ALL_ROLES,
};
use leptos::prelude::*;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn RoleOverview(
    catalog: Arc<Catalog>,
    chosen: Memo<Vec<usize>>,
    language: &'static str,
) -> impl IntoView {
    let content = catalog.clone();
    view! {
        <section class="panel">
            <h2>{catalog.text("coverageTitle")}</h2>
            <p>{catalog.text("coverageSubtitle")}</p>
            {move || chosen.with(|ids| {
                let plants: Vec<_> = ids.iter().map(|&index| &content.plants[index]).collect();
                let roles = ALL_ROLES.map(|role| plants.iter()
                    .filter(|plant| plant.role_ids.iter().any(|value| value == role))
                    .map(|plant| plant.name.clone()).collect::<Vec<_>>());
                let covered = roles.iter().filter(|names| !names.is_empty()).count();
                let percent = (covered * 100 + ALL_ROLES.len() / 2) / ALL_ROLES.len();
                let rows = ALL_ROLES.into_iter().zip(roles).map(|(role, plants)| {
                    let missing = plants.is_empty();
                    let text = if missing { content.text("notYetCovered") } else { plants.join(", ") };
                    view! {
                        <div class:missing-role=missing data-guild-role=role>
                            <dt><a href=format!("/guides/?lang={language}#role-{role}")>{content.text(&format!("role:{role}"))}</a></dt>
                            <dd>{text}</dd>
                        </div>
                    }
                }).collect_view();
                let mut multiple: Vec<_> = plants.into_iter().filter(|p| p.role_ids.len() >= 2).collect();
                multiple.sort_by_key(|p| std::cmp::Reverse(p.role_ids.len()));
                view! {
                    <div class="guild-role-total">
                        <strong data-guild-coverage=covered>{format!("{covered} / {}", ALL_ROLES.len())}</strong>
                        <span>{format!("{percent}% {}", content.text("coveredRatio"))}</span>
                        <meter min="0" max="9" value=covered aria-label=content.text("coverageTitle")></meter>
                    </div>
                    <dl class="role-grid">{rows}</dl>
                    {(!multiple.is_empty()).then(|| view! {
                        <h3>{content.text("heroesTitle")}</h3>
                        <ul class="role-heroes">{multiple.into_iter().map(|plant| view! {
                            <li>{format!("{} · {} {}", plant.name, plant.role_ids.len(), content.text("rolesStacked"))}</li>
                        }).collect_view()}</ul>
                        <p>{content.text("heroesTip")}</p>
                    })}
                }
            })}
        </section>
    }
}

#[derive(Deserialize)]
struct Reference {
    title: String,
    name: String,
    mechanism: String,
    advice: String,
    sources: Vec<Citation>,
}

type References = Arc<HashMap<String, Reference>>;

#[component]
pub fn GuildNotices(
    catalog: Arc<Catalog>,
    guild: RwSignal<Guild>,
    language: &'static str,
) -> impl IntoView {
    let lookup = catalog.clone();
    let geometry = Memo::new(move |_| {
        guild.with(|state| {
            (
                state.tree_id.clone(),
                state.plant_ids.clone(),
                state.hemisphere == "SOUTHERN",
            )
        })
    });
    let report = Memo::new(move |_| {
        geometry.with(|(id, selection, southern)| {
            let tree = lookup.trees.iter().find(|tree| &tree.id == id)?;
            let plants: Vec<_> = selection
                .iter()
                .filter_map(|id| lookup.plants.iter().find(|plant| &plant.id == id))
                .collect();
            let positions = place(tree, &plants, *southern);
            Some((
                tree.name.clone(),
                guild_antagonisms::analyze(&lookup, tree, &plants, &positions),
            ))
        })
    });
    let sources = RwSignal::new(None::<References>);
    let loading = RwSignal::new(false);
    let failed = RwSignal::new(false);
    let load = move |_| {
        if loading.get_untracked() || sources.with_untracked(Option::is_some) {
            return;
        }
        loading.set(true);
        failed.set(false);
        spawn_local(async move {
            match crate::fetch_json(&format!("guild-notices-{language}.json")).await {
                Ok(data) => sources.set(Some(Arc::new(data))),
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    };
    let content = catalog.clone();
    let status_catalog = catalog.clone();
    view! {
        <section class="panel">
            <h2>{catalog.text("antagonistsSectionTitle")}</h2>
            <p>{catalog.text("antagonistsSectionSub")}</p>
            {move || report.with(|report| report.as_ref().map(|(tree, notices)| {
                let sections = [
                    (Scope::Guild, "antagonistInternalTag"),
                    (Scope::Landscape, "antagonistExternalTag"),
                    (Scope::Resolved, "antagonistCardAutoOptimizedSpacing"),
                ];
                sections.into_iter().filter_map(|(scope, label)| {
                    let found: Vec<_> = notices.iter().filter(|notice| notice.scope == scope).cloned().collect();
                    if found.is_empty() {
                        return (scope == Scope::Guild).then(|| view! {
                            <p>{content.text("antagonistCardZeroConflicts")}</p>
                        }.into_any());
                    }
                    let count = found.len();
                    let descriptions = content.clone();
                    let tree = tree.clone();
                    Some(view! {
                        <details class="guild-notice-group" data-guild-notice-count=count>
                            <summary on:click=load>{format!("{} ({count})", content.text(label))}</summary>
                            {move || sources.with(|data| {
                                let data = data.as_ref()?;
                                Some(found.iter().filter_map(|notice| {
                                    let reference = data.get(&notice.id)?;
                                    Some(notice_view(&descriptions, notice, reference, &tree, language))
                                }).collect_view())
                            })}
                        </details>
                    }.into_any())
                }).collect_view()
            }))}
            <p role="status" hidden=move || !loading.get()>{status_catalog.text("loadingFallback")}</p>
            <p role="status" hidden=move || !failed.get()>
                {match language { "en" => "Could not load the notices. Reopen a section to try again.",
                    "de" => "Hinweise konnten nicht geladen werden. Öffne einen Abschnitt erneut.",
                    _ => "Kunne ikke laste varslene. Åpne en seksjon på nytt." }}
            </p>
        </section>
    }
}

fn notice_view(
    catalog: &Catalog,
    notice: &Notice,
    reference: &Reference,
    tree: &str,
    language: &str,
) -> impl IntoView {
    let localized = |value: &str| {
        value
            .replace("{tree}", tree)
            .replace("{limit}", &number(notice.limit, 1, language))
    };
    let title = localized(&reference.title);
    let name = localized(&reference.name);
    let mechanism = localized(&reference.mechanism);
    let advice = localized(&reference.advice)
        .replace(
            "{measured}",
            &number(notice.measured.unwrap_or_default(), 2, language),
        )
        .replace(
            "{angle}",
            &notice
                .affected
                .first()
                .and_then(|plant| plant.angle)
                .unwrap_or_default()
                .to_string(),
        );
    let distance_label = if notice.id == "external-walnut-spatial-planning" {
        match language {
            "de" => "Abstand vom Mittelpunkt der Gilde",
            "en" => "Distance from the guild centre",
            _ => "Avstand fra midten av lauget",
        }
    } else {
        match language {
            "de" => "Gemessener Abstand",
            "en" => "Measured distance",
            _ => "Målt avstand",
        }
    };
    view! {
        <article class=format!("warning-detail {}", notice.severity.class()) data-guild-notice=notice.id.clone()>
            <h3>{title}</h3>
            <p>{name} " · " {notice.severity.label(language)}</p>
            <p>{catalog.text("antagonistBufferLabel")} " ≥ " {number(notice.limit, 1, language)} " m"
                {notice.measured.map(|distance| format!(" · {distance_label} {} m", number(distance, 2, language)))}
            </p>
            <p><strong>{catalog.text("antagonistAffectedInGuild")}</strong></p>
            <ul>{notice.affected.iter().map(|plant| {
                let location = format!("{} m{}", number(plant.distance, 1, language), plant.angle.map_or_else(String::new, |angle| format!(" · {angle}°")));
                view! { <li>{plant.name.clone()} " · " {location}</li> }
            }).collect_view()}</ul>
            <p><strong>{catalog.text("antagonistMechanism")}</strong></p>
            <p>{mechanism}</p>
            {(!advice.is_empty()).then(|| view! { <p><strong>{catalog.text("antagonistSpatialAdvice")}</strong></p><p>{advice}</p> })}
            {(!reference.sources.is_empty()).then(|| view! {
                <details>
                    <summary>{catalog.text("antagonistScientificSources")}</summary>
                    <ul>{reference.sources.iter().map(|citation| view! {
                        <li>{citation.href.as_ref().map_or_else(|| citation.label.clone().into_any(), |url| view! {
                            <a href=url.clone() target="_blank" rel="noopener noreferrer">{citation.label.clone()}</a>
                        }.into_any())}</li>
                    }).collect_view()}</ul>
                </details>
            })}
        </article>
    }
}
