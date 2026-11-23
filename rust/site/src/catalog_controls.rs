use crate::catalog::Catalog;
use leptos::prelude::*;
use std::sync::Arc;

const TABS: [&str; 11] = [
    "ALL",
    "RECOMMENDED",
    "NITROGEN_FIXER",
    "DYNAMIC_ACCUMULATOR",
    "POLLINATOR_MAGNET",
    "PEST_REPELLER",
    "LIVING_MULCH",
    "GRASS_BARRIER",
    "ANTIFUNGAL",
    "BIOMASS_PRODUCER",
    "EDIBLE_UNDERSTORY",
];

#[component]
pub fn CatalogControls(
    catalog: Arc<Catalog>,
    role: RwSignal<&'static str>,
    soil: RwSignal<bool>,
    zone: RwSignal<bool>,
    on_change: Callback<()>,
) -> impl IntoView {
    let options = TABS
        .into_iter()
        .map(|value| {
            let label = match value {
                "ALL" => catalog.text("tabAll"),
                "RECOMMENDED" => catalog.text("tabRecommended"),
                _ => catalog.text(&format!("role:{value}")),
            };
            view! {
                <button type="button" data-plant-role=value
                    aria-pressed=move || (role.get() == value).to_string()
                    on:click=move |_| {
                        role.set(value);
                        on_change.run(());
                    }>
                    {label}
                </button>
            }
        })
        .collect_view();
    view! {
        <div class="catalog-filters">
            <div class="catalog-roles" role="group" aria-label=catalog.text("functionalRole")>{options}</div>
            <div class="catalog-site-filters">
                <label>
                    <input type="checkbox" prop:checked=soil on:change=move |ev| soil.set(event_target_checked(&ev)) />
                    {catalog.text("filterOptimalSoil")}
                </label>
                <label>
                    <input type="checkbox" prop:checked=zone on:change=move |ev| zone.set(event_target_checked(&ev)) />
                    {catalog.text("filterOptimalZone")}
                </label>
            </div>
        </div>
    }
}

#[component]
pub fn Presets(catalog: Arc<Catalog>, on_select: Callback<&'static str>) -> impl IntoView {
    let open = RwSignal::new(false);
    let options = [
        ("apple", "presetApple", "presetAppleSub"),
        ("walnut", "presetWalnut", "presetWalnutSub"),
        ("apricot", "presetApricot", "presetApricotSub"),
        ("minimal", "presetMinimal", "presetMinimalSub"),
    ]
    .into_iter()
    .filter(|(id, _, _)| catalog.presets.contains_key(*id))
    .map(|(id, label, subtitle)| {
        view! {
            <button type="button" on:click=move |_| {
                on_select.run(id);
                open.set(false);
            }>
                <strong>{catalog.text(label)}</strong> " · " {catalog.text(subtitle)}
            </button>
        }
    })
    .collect_view();
    view! {
        <div class="guild-presets">
            <button type="button" aria-controls="guild-preset-options"
                aria-expanded=move || open.get().to_string()
                on:click=move |_| open.update(|value| *value = !*value)>
                {catalog.text("presets")}
            </button>
            <div id="guild-preset-options" role="group" aria-label=catalog.text("presetsTitle")
                hidden=move || !open.get()>
                {options}
            </div>
        </div>
    }
}
