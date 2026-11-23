use crate::{
    calendar::Season,
    catalog::{Catalog, Species},
    drawing::number,
    ground_cover::Cover,
};
use leptos::prelude::*;
use std::{collections::HashSet, sync::Arc};
use wasm_bindgen::JsCast;

type Point = [f64; 2];

#[derive(Clone, Copy, PartialEq)]
pub struct Probe {
    pub point: Point,
    pub pick: f64,
}

#[derive(Clone, Copy)]
pub struct Inspector {
    pub probe: RwSignal<Option<Probe>>,
    pub active: RwSignal<bool>,
}

impl Inspector {
    pub fn new() -> Self {
        let inspector = Self {
            probe: RwSignal::new(None),
            active: RwSignal::new(false),
        };
        let release = window_event_listener(leptos::ev::keyup, move |ev| {
            if ev.key() == "Shift" {
                inspector.clear_hover();
            }
        });
        let blur = window_event_listener(leptos::ev::blur, move |_| inspector.clear_hover());
        on_cleanup(move || {
            release.remove();
            blur.remove();
        });
        inspector
    }

    pub fn clear_hover(self) {
        if !self.active.get_untracked() && self.probe.with_untracked(Option::is_some) {
            self.probe.set(None);
        }
    }

    pub fn toggle(self) {
        self.active.update(|value| *value = !*value);
        self.probe.set(None);
    }

    pub fn stop(self) {
        if self.active.get_untracked() {
            self.toggle();
        } else {
            self.clear_hover();
        }
    }

    pub fn pointer(self, ev: &web_sys::PointerEvent, corner: Point, width: f64, pressed: bool) {
        let pinned = self.active.get_untracked();
        if !pressed && pinned && ev.pointer_type() == "touch" {
            return;
        }
        if !pinned && (pressed || !ev.shift_key()) {
            if !pressed {
                self.clear_hover();
            }
            return;
        }
        let Some(svg) = ev
            .current_target()
            .and_then(|node| node.dyn_into::<web_sys::SvgElement>().ok())
        else {
            return;
        };
        let rect = svg.get_bounding_client_rect();
        let size = rect.width().min(rect.height());
        if size <= 0.0 {
            return;
        }
        let x = f64::from(ev.client_x()) - rect.x() - (rect.width() - size) / 2.0;
        let y = f64::from(ev.client_y()) - rect.y() - (rect.height() - size) / 2.0;
        if !(0.0..=size).contains(&x) || !(0.0..=size).contains(&y) {
            self.clear_hover();
            return;
        }
        let scale = width / size;
        self.probe.set(Some(Probe {
            point: [corner[0] + x * scale, corner[1] + y * scale],
            pick: 9.0 * scale,
        }));
    }

    pub fn keyboard(self, ev: &web_sys::KeyboardEvent, corner: Point, width: f64) -> bool {
        if !self.active.get_untracked() {
            return false;
        }
        let movement = match ev.key().as_str() {
            "Escape" => {
                self.toggle();
                ev.prevent_default();
                return true;
            }
            "ArrowLeft" => [-1.0, 0.0],
            "ArrowRight" => [1.0, 0.0],
            "ArrowUp" => [0.0, -1.0],
            "ArrowDown" => [0.0, 1.0],
            _ => return false,
        };
        ev.prevent_default();
        let step = width * if ev.shift_key() { 0.1 } else { 0.025 };
        let mut point = self
            .probe
            .get_untracked()
            .map_or_else(|| corner.map(|v| v + width / 2.0), |probe| probe.point);
        for i in 0..2 {
            point[i] = (point[i] + movement[i] * step).clamp(corner[i], corner[i] + width);
        }
        self.probe.set(Some(Probe {
            point,
            pick: width * 0.02,
        }));
        true
    }

    pub fn marker(self) -> impl IntoView {
        move || {
            self.probe.get().map(|probe| view! {
            <circle class="spot-marker" cx=probe.point[0] cy=probe.point[1]
                r=probe.pick * 0.7 pointer-events="none" vector-effect="non-scaling-stroke" />
        })
        }
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Kind {
    Trunk,
    Plant,
    Canopy,
    Cover,
    Drift,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Trunk => "spotKindTrunk",
            Self::Plant => "spotKindPlant",
            Self::Canopy => "spotKindCanopy",
            Self::Cover => "spotKindCover",
            Self::Drift => "spotKindDrift",
        }
    }
}

#[derive(PartialEq)]
pub struct Entry {
    kind: Kind,
    name: String,
    color: String,
    distance: f64,
    out_of_season: bool,
}

pub fn query<'a>(
    probe: Probe,
    stars: impl IntoIterator<Item = (&'a Species, Point)>,
    plants: impl IntoIterator<Item = (&'a Species, Point)>,
    covers: &[Cover],
    season: Option<Season>,
) -> Vec<Entry> {
    let distance = |[x, y]: Point| (x - probe.point[0]).hypot(y - probe.point[1]);
    let mut entries = Vec::new();
    let mut cover_ids = HashSet::new();
    for cover in covers {
        let inside = cover.rings.iter().fold(false, |inside, ring| {
            inside ^ point_in_ring(probe.point, ring)
        });
        if inside {
            cover_ids.insert(cover.plant_id.as_str());
            entries.push(Entry {
                kind: if cover.drift {
                    Kind::Drift
                } else {
                    Kind::Cover
                },
                name: cover.name.clone(),
                color: cover.color.clone(),
                distance: 0.0,
                out_of_season: !cover.in_season(season),
            });
        }
    }
    let mut add = |species: &Species, kind, distance| {
        entries.push(Entry {
            kind,
            name: species.name.clone(),
            color: species.color.clone(),
            distance,
            out_of_season: false,
        })
    };
    for (species, center) in stars {
        let dist = distance(center);
        if dist <= probe.pick {
            add(species, Kind::Trunk, dist);
        } else if dist <= species.mature_radius_m {
            add(species, Kind::Canopy, dist);
        }
    }
    for (species, center) in plants {
        let dist = distance(center);
        if !cover_ids.contains(species.id.as_str())
            && dist <= probe.pick.max(species.spread_m / 2.0)
        {
            add(species, Kind::Plant, dist);
        }
    }
    entries.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.distance.total_cmp(&b.distance)));
    entries
}

fn point_in_ring([x, y]: Point, ring: &[Point]) -> bool {
    let Some(mut previous) = ring.last().copied() else {
        return false;
    };
    let mut inside = false;
    for &next in ring {
        if (previous[1] > y) != (next[1] > y)
            && x < (next[0] - previous[0]) * (y - previous[1]) / (next[1] - previous[1])
                + previous[0]
        {
            inside = !inside;
        }
        previous = next;
    }
    inside
}

pub fn grid_query(
    catalog: &Catalog,
    probe: Probe,
    stars: &[crate::garden::Star],
    companions: &[crate::garden_layout::Companion],
    covers: &[Cover],
    season: Option<Season>,
) -> Vec<Entry> {
    query(
        probe,
        stars.iter().filter_map(|star| {
            let species = catalog.trees.iter().find(|tree| tree.id == star.tree_id)?;
            Some((species, [star.x_m, star.y_m]))
        }),
        companions.iter().filter_map(|companion| {
            let species = catalog
                .plants
                .iter()
                .find(|plant| plant.id == companion.plant_id)?;
            Some((species, [companion.x_m, companion.y_m]))
        }),
        covers,
        season,
    )
}

#[component]
pub fn SpotControl(
    catalog: Arc<Catalog>,
    inspector: Inspector,
    language: &'static str,
) -> impl IntoView {
    let name = match language {
        "de" => "Punkt prüfen",
        "en" => "Inspect spot",
        _ => "Undersøk punkt",
    };
    let instruction = match language {
        "de" => "Tippe auf die Karte oder nutze die Pfeiltasten, wenn die Karte den Fokus hat.",
        "en" => "Tap the map or use the arrow keys when the map has focus.",
        _ => "Trykk på kartet, eller bruk piltastene når kartet har fokus.",
    };
    view! {
        <div class="spot-control">
            <button type="button" aria-pressed=move || inspector.active.get().to_string()
                on:click=move |_| inspector.toggle()>{name}</button>
            <small>{move || if inspector.active.get() { instruction.to_owned() } else { catalog.text("spotHint") }}</small>
        </div>
    }
}

#[component]
pub fn SpotReadout(
    catalog: Arc<Catalog>,
    inspector: Inspector,
    inspect: Callback<Probe, Vec<Entry>>,
    language: &'static str,
    #[prop(default = false)] from_trunk: bool,
) -> impl IntoView {
    let measured = Memo::new(move |_| {
        inspector
            .probe
            .get()
            .map(|probe| (probe, inspect.run(probe)))
    });
    view! {
        <div class="spot-readout" role="status" hidden=move || measured.with(Option::is_none)>
            {move || measured.with(|value| value.as_ref().map(|(probe, entries)| {
                let [x, y] = probe.point;
                let description = if from_trunk {
                    catalog.text("spotFromTrunk").replace("{m}", &number(x.hypot(y), 1, language))
                } else {
                    let coordinate = |value: f64| number(if value.abs() < 0.05 { 0.0 } else { value }, 1, language);
                    format!("x {} m · y {} m", coordinate(x), coordinate(y))
                };
                view! {
                    <strong>{catalog.text("spotTitle")}</strong>
                    <p>{description}</p>
                    {entries.is_empty().then(|| view! { <p>{catalog.text("spotNothing")}</p> })}
                    <ul>{entries.iter().map(|entry| {
                        let mut label = catalog.text(entry.kind.label());
                        if entry.out_of_season { label.push_str(&format!(" ({})", catalog.text("groundCoverOutOfSeason"))); }
                        view! {
                            <li class:out-of-season=entry.out_of_season><span class="map-key" style:background-color=entry.color.clone()></span>
                                <strong>{entry.name.clone()}</strong>" · "{label}
                            </li>
                        }
                    }).collect_view()}</ul>
                }
            }))}
        </div>
    }
}
