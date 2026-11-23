use crate::{
    catalog::Catalog,
    guild::Guild,
    pest_defense::{self, Defense},
};
use leptos::prelude::*;
use std::sync::Arc;

#[component]
pub fn PestChoices(
    catalog: Arc<Catalog>,
    guild: RwSignal<Guild>,
    tree: Memo<Option<usize>>,
    selected: RwSignal<Option<String>>,
    on_select: Callback<String>,
) -> impl IntoView {
    let heading = catalog.text("keyPests");
    let climate = Memo::new(move |_| guild.with(|guild| guild.climate_zone.clone()));
    view! {
        {move || tree.get().and_then(|tree| {
            let species = &catalog.trees[tree];
            if species.vulnerabilities.is_empty() {
                return None;
            }
            let zone = climate.get();
            let choices = species.vulnerabilities.iter().map(|pest| {
                if let Some(defense) = pest_defense::resolve(&catalog, species, pest, &zone) {
                    let evidence = defense.evidence.label(&catalog);
                    let title = format!("{} ({}) · {}", catalog.text("keyPestClickToFilter"), defense.plants.len(),
                        catalog.text("evidenceLevelTooltip").replace("{level}", &evidence));
                    let name = pest.clone();
                    let active_pest = pest.clone();
                    view! {
                        <button type="button" title=title data-pest-filter=name.clone()
                            aria-pressed=move || selected.with(|value| value.as_ref() == Some(&active_pest)).to_string()
                            on:click=move |_| on_select.run(name.clone())>
                            {pest.clone()} " · " <small>{evidence}</small>
                        </button>
                    }.into_any()
                } else {
                    view! { <span>{pest.clone()}</span> }.into_any()
                }
            }).collect_view();
            Some(view! {
                <div class="pest-choices">
                    <h3>{heading.clone()}</h3>
                    <div>{choices}</div>
                </div>
            })
        })}
    }
}

#[component]
pub fn PestAdvice(
    catalog: Arc<Catalog>,
    selected: RwSignal<Option<String>>,
    defense: Memo<Option<Defense>>,
    available: Memo<Vec<usize>>,
    language: &'static str,
) -> impl IntoView {
    let title = catalog.text("activePestBannerTitle");
    let clear = catalog.text("clearPestFilter");
    view! {
        {move || selected.get().map(|pest| {
            let advice = defense.get().map(|result| {
                let rule = &catalog.pest_defense_rules[result.rule];
                let citations = rule.citations.iter().map(|citation| view! {
                    <li>{if let Some(href) = &citation.href {
                        view! { <a href=href.clone() target="_blank" rel="noopener noreferrer">{citation.label.clone()}</a> }.into_any()
                    } else {
                        citation.label.clone().into_any()
                    }}</li>
                }).collect_view();
                let recommendations = available.with(|indices| indices.iter().map(|&i| {
                    let plant = &catalog.plants[i];
                    let role = rule.companion_role(&plant.id);
                    view! {
                        <li><strong>{plant.name.clone()}</strong> " · " {rule.companion_evidence(&plant.id).label(&catalog)}
                            <p>{role}</p>
                        </li>
                    }
                }).collect_view());
                view! {
                    <h4>{rule.title.clone()} " · " {result.evidence.label(&catalog)}</h4>
                    <p><strong>{catalog.text("scientificMechanismTitle")}</strong></p>
                    <p>{rule.mechanism.clone()}</p>
                    <h4>{catalog.text("evidenceSourcesTitle")}</h4>
                    <ul>{citations}</ul>
                    <p class="pest-note">{catalog.text("evidenceBadgeNote")}</p>
                    <p><a href=format!("/guides/pests/?lang={language}#{}", rule.id)>{catalog.text("navGuides")}</a></p>
                    {(!available.with(Vec::is_empty)).then(|| view! {
                        <h4>{catalog.text("effectiveCompanionsTitle")}</h4>
                        <ul>{recommendations}</ul>
                    })}
                }
            });
            view! {
                <div class="pest-advice" role="region" aria-label=title.clone()>
                    <div class="pest-advice-heading">
                        <h3>{title.clone()} " · " {pest}</h3>
                        <button type="button" on:click=move |_| selected.set(None)>{clear.clone()}</button>
                    </div>
                    {advice}
                </div>
            }
        })}
    }
}
