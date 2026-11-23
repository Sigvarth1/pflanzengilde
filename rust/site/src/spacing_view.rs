use crate::{
    catalog::Catalog,
    drawing::number,
    guild::Guild,
    spacing::{self, Report},
};
use leptos::prelude::*;
use std::sync::Arc;

#[component]
pub fn SpacingWarnings(
    catalog: Arc<Catalog>,
    guild: RwSignal<Guild>,
    report: Arc<Report>,
    language: &'static str,
) -> impl IntoView {
    let swap_catalog = catalog.clone();
    let swap = Callback::new(move |(old, replacement): (String, usize)| {
        let accepted = guild.with_untracked(|state| {
            let Some(tree) = swap_catalog
                .trees
                .iter()
                .find(|tree| tree.id == state.tree_id)
            else {
                return false;
            };
            let Some(original) = swap_catalog.plants.iter().find(|plant| plant.id == old) else {
                return false;
            };
            state.plant_ids.contains(&old)
                && spacing::alternatives(&swap_catalog, tree, original, state)
                    .iter()
                    .any(|item| item.plant == replacement)
        });
        if accepted {
            crate::update_guild(guild, |state| {
                state.plant_ids.retain(|id| id != &old);
                state
                    .plant_ids
                    .push(swap_catalog.plants[replacement].id.clone());
            });
        }
    });
    let items = report.conflicts.iter().filter_map(|conflict| {
        let [a, b] = &conflict.plants;
        let first = catalog.plants.iter().find(|plant| &plant.id == a)?;
        let second = catalog.plants.iter().find(|plant| &plant.id == b)?;
        let severity = catalog.text(if conflict.critical { "spacingCriticalAlert" } else { "spacingWarningAlert" });
        let distance = format!("{} m", number(conflict.distance_m, 2, language));
        let required = format!("{} m", number(conflict.required_m, 2, language));
        let overlap = format!("{}% ({} m)", conflict.overlap, number(conflict.overlap_m, 2, language));
        let choices = [first, second].into_iter().filter_map(|original| {
            let suggestions = report.alternatives.get(&original.id)?;
            if suggestions.is_empty() { return None; }
            let options = suggestions.iter().map(|option| {
                let replacement = &catalog.plants[option.plant];
                let old = original.id.clone();
                let index = option.plant;
                let roles = option.shared_roles.iter().map(|role| catalog.text(&format!("role:{role}"))).collect::<Vec<_>>().join(" · ");
                let benefit = if option.reduction_m >= 0.15 {
                    format!("{} m → {} m (−{}%)", number(original.spread_m, 2, language),
                        number(replacement.spread_m, 2, language), option.reduction_percent)
                } else { replacement.layer_label.clone() };
                view! {
                    <li>
                        <strong>{replacement.name.clone()}</strong>
                        <p>{benefit} " · " {roles}</p>
                        <button type="button" data-spacing-swap=replacement.id.clone()
                            on:click=move |_| swap.run((old.clone(), index))>
                            {catalog.text("spacingSwapAction")} " " {replacement.name.clone()}
                        </button>
                    </li>
                }
            }).collect_view();
            let label = catalog.text("spacingBannerAlternativesFor")
                .replace("{plant}", &original.name)
                .replace("{spread}", &number(original.spread_m, 2, language));
            Some(view! { <div><h4>{label}</h4><ul class="spacing-options">{options}</ul></div> })
        }).collect_view();
        Some(view! {
            <details class="warning-detail" class:conflict-critical=conflict.critical class:conflict-warning=!conflict.critical
                data-spacing-overlap=conflict.overlap>
                <summary>{first.name.clone()} " × " {second.name.clone()} " · " {severity}</summary>
                <p>{catalog.text("spacingOverlapRatio")} " " {overlap}
                    " · " {catalog.text("spacingMeasuredDistance")} " " {distance}
                    " (" {catalog.text("spacingRequiredLabel")} " ≥ " {required} ")"</p>
                <p>{catalog.text("spacingConflictDesc")}</p>
                {choices}
            </details>
        })
    }).collect_view();
    let crowded = report.crowded.iter().map(|zone| {
        view! {
            <li data-spacing-zone=zone.zone.label()>
                <strong>{catalog.text(zone.zone.label())} " · " {format!("{}%", zone.percent)}</strong>
                <p>{zone.plants.to_string()} " " {catalog.text("spacingBannerPlantsAcross")} " "
                    {number(zone.zone_area, 1, language)} " m² · "
                    {number(zone.plant_area, 1, language)} " m² " {catalog.text("spacingBannerTotalCanopyArea")}</p>
            </li>
        }
    }).collect_view();

    view! {
        <section class="spacing-warnings">
            <h3>{catalog.text("spacingOverlapTitle")}</h3>
            <p>{catalog.text("spacingOverlapSub")}</p>
            {items}
            {(!report.crowded.is_empty()).then(|| view! {
                <h4>{catalog.text("spacingZoneSaturatedTitle")}</h4>
                <p>{catalog.text("spacingZoneSaturatedDesc")}</p>
                <ul>{crowded}</ul>
            })}
        </section>
    }
}
