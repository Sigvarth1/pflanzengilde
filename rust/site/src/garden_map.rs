use crate::{
    calendar::Season,
    catalog::Catalog,
    garden::{Bed, Shape, Star},
    garden_conflicts::ShadePocket,
    garden_editor,
    garden_layout::Companion,
    garden_page::{commit_shape, instance_id, save, Draft},
    garden_resolution::Resolution,
    ground_cover::Cover,
    ground_cover_view,
    infrastructure::{Gesture, Point, Tool},
};
use leptos::prelude::*;
use std::sync::Arc;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, PartialEq)]
struct Frame {
    center: Point,
    width: f64,
}

impl Frame {
    fn corner(self) -> Point {
        self.center.map(|value| value - self.width / 2.0)
    }

    fn coordinates(self, event: [i32; 2], svg: &web_sys::SvgElement) -> Option<Point> {
        let rect = svg.get_bounding_client_rect();
        let size = rect.width().min(rect.height());
        if size <= 0.0 {
            return None;
        }
        let [x0, y0] = self.corner();
        let [x, y] = event.map(f64::from);
        let x = x - rect.x() - (rect.width() - size) / 2.0;
        let y = y - rect.y() - (rect.height() - size) / 2.0;
        Some([x0 + x * self.width / size, y0 + y * self.width / size])
    }

    fn view_box(self) -> String {
        let [x, y] = self.corner();
        format!("{x} {y} {0} {0}", self.width)
    }
}

#[derive(Clone)]
enum Mode {
    Draw(Gesture),
    Trees {
        primary: Point,
        start: Point,
        originals: Vec<Star>,
        delta: Point,
    },
    Pan {
        start: Point,
    },
    Select {
        start: Point,
        end: Point,
        extend: bool,
    },
}

#[derive(Clone)]
struct Motion {
    pointer: i32,
    pixels: [i32; 2],
    frame: Frame,
    mode: Mode,
}

impl Motion {
    fn drift(&self, pixels: [i32; 2]) -> bool {
        (0..2).any(|axis| (pixels[axis] - self.pixels[axis]).abs() > 4)
    }
}

#[derive(Clone)]
enum Clipboard {
    Trees(Vec<Star>),
    Bed(Bed),
}

pub(crate) fn translate_shape(shape: &mut Shape, [dx, dy]: Point) {
    match shape {
        Shape::Rect { x_m, y_m, .. } => {
            *x_m += dx;
            *y_m += dy;
        }
        Shape::Circle { cx_m, cy_m, .. } => {
            *cx_m += dx;
            *cy_m += dy;
        }
        Shape::Polygon { points } => {
            for [x, y] in points {
                *x += dx;
                *y += dy;
            }
        }
    }
}

fn shape_view(shape: &Shape, bed: bool) -> AnyView {
    let class = if bed { "grid-bed" } else { "grid-outline" };
    match shape {
        Shape::Rect { x_m, y_m, w_m, h_m } => view! {
            <rect x=*x_m y=*y_m width=*w_m height=*h_m class=class />
        }
        .into_any(),
        Shape::Circle { cx_m, cy_m, r_m } => view! {
            <circle cx=*cx_m cy=*cy_m r=*r_m class=class />
        }
        .into_any(),
        Shape::Polygon { points } => {
            let points = points
                .iter()
                .map(|[x, y]| format!("{x},{y}"))
                .collect::<Vec<_>>()
                .join(" ");
            view! { <polygon points=points class=class /> }.into_any()
        }
    }
}

fn move_delta(primary: Point, start: Point, now: Point, originals: &[Star]) -> Point {
    std::array::from_fn(|axis| {
        let ideal = primary[axis] + now[axis] - start[axis];
        let desired = (ideal * 4.0 + 0.5).floor() / 4.0 - primary[axis];
        let limits = originals
            .iter()
            .fold([-f64::INFINITY, f64::INFINITY], |limits, tree| {
                let x = [tree.x_m, tree.y_m][axis];
                [limits[0].max(-300.0 - x), limits[1].min(300.0 - x)]
            });
        if limits[0] > limits[1] {
            0.0
        } else {
            desired.clamp(limits[0], limits[1])
        }
    })
}

fn adjust_trees(draft: RwSignal<Draft>, ids: &[String], [dx, dy]: Point) {
    if dx == 0.0 && dy == 0.0 {
        return;
    }
    save(draft, |state| {
        for tree in state
            .garden
            .star_plants
            .iter_mut()
            .filter(|tree| ids.contains(&tree.instance_id))
        {
            if dx != 0.0 {
                tree.x_m = ((tree.x_m + dx) * 100.0).round() / 100.0;
            }
            if dy != 0.0 {
                tree.y_m = ((tree.y_m + dy) * 100.0).round() / 100.0;
            }
        }
    });
}

fn refresh_selection(selected: RwSignal<String>, group: RwSignal<Vec<String>>, ids: Vec<String>) {
    selected.set(ids.first().cloned().unwrap_or_default());
    group.set(ids);
}

fn element_target(
    event: &web_sys::PointerEvent,
    selector: &str,
    attribute: &str,
) -> Option<String> {
    event
        .target()?
        .dyn_into::<web_sys::Element>()
        .ok()?
        .closest(selector)
        .ok()??
        .get_attribute(attribute)
}

#[component]
pub fn GardenMap(
    catalog: Arc<Catalog>,
    draft: RwSignal<Draft>,
    selected: RwSignal<String>,
    group: RwSignal<Vec<String>>,
    focused_plant: RwSignal<String>,
    zoom: RwSignal<f64>,
    center: RwSignal<Option<Point>>,
    companions: Memo<Vec<Companion>>,
    resolution: Memo<Resolution>,
    shade: Memo<Vec<ShadePocket>>,
    covers: Memo<Vec<Cover>>,
    show_covers: RwSignal<bool>,
    cover_season: RwSignal<Option<Season>>,
    tool: RwSignal<Option<Tool>>,
    notice: RwSignal<String>,
    language: &'static str,
    on_import: Callback<(web_sys::File, Point)>,
) -> impl IntoView {
    let title = catalog.text("gardenGridTitle");
    let inspector = crate::spot::Inspector::new();
    let selecting = RwSignal::new(false);
    let active = RwSignal::new(None::<Motion>);
    let selected_bed = RwSignal::new(None::<String>);
    let clipboard = RwSignal::new(None::<Clipboard>);
    let dropping = RwSignal::new(false);
    let fit_catalog = catalog.clone();
    let fit = Memo::new(move |_| {
        let mut bounds = [-4.0_f64, -4.0_f64, 4.0_f64, 4.0_f64];
        let mut include = |x: f64, y: f64, radius: f64| {
            bounds[0] = bounds[0].min(x - radius);
            bounds[1] = bounds[1].min(y - radius);
            bounds[2] = bounds[2].max(x + radius);
            bounds[3] = bounds[3].max(y + radius);
        };
        draft.with(|state| {
            for star in &state.garden.star_plants {
                if let Some(species) = fit_catalog
                    .trees
                    .iter()
                    .find(|species| species.id == star.tree_id)
                {
                    include(star.x_m, star.y_m, species.mature_radius_m);
                }
            }
            for form in state.garden.infrastructure.outline.iter().chain(
                state
                    .garden
                    .infrastructure
                    .raised_beds
                    .iter()
                    .map(|bed| &bed.shape),
            ) {
                for [x, y] in form.points() {
                    include(x, y, 0.0);
                }
            }
        });
        companions.with(|items| {
            for companion in items {
                let radius = fit_catalog
                    .plants
                    .iter()
                    .find(|species| species.id == companion.plant_id)
                    .map_or(0.08, |species| species.spread_m / 2.0);
                include(companion.x_m, companion.y_m, radius);
            }
        });
        if show_covers.get() {
            covers.with(|items| {
                for [x, y] in items.iter().flat_map(|cover| cover.rings.iter().flatten()) {
                    include(*x, *y, 0.0);
                }
            });
        }
        Frame {
            center: [(bounds[0] + bounds[2]) / 2.0, (bounds[1] + bounds[3]) / 2.0],
            width: (bounds[2] - bounds[0]).max(bounds[3] - bounds[1]) + 2.0,
        }
    });
    let frame = Memo::new(move |_| {
        let fitted = fit.get();
        Frame {
            center: center.get().unwrap_or(fitted.center),
            width: fitted.width / zoom.get(),
        }
    });
    let grid = move || if frame.get().width > 80.0 { 5.0 } else { 1.0 };
    Effect::new(move |_| {
        if tool.with(Option::is_some) {
            inspector.stop();
        }
        active.set(None);
    });
    let on_down = move |event: web_sys::PointerEvent| {
        if event.button() != 0 || !event.is_primary() || active.with_untracked(Option::is_some) {
            return;
        }
        let Some(svg) = event
            .current_target()
            .and_then(|target| target.dyn_into::<web_sys::SvgElement>().ok())
        else {
            return;
        };
        let current = frame.get_untracked();
        let pixels = [event.client_x(), event.client_y()];
        let Some(point) = current.coordinates(pixels, &svg) else {
            return;
        };
        let pointer = event.pointer_id();
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = svg.focus_with_options(&options);
        let mode = if let Some(tool) = tool.get_untracked() {
            Mode::Draw(Gesture {
                tool,
                points: vec![point.map(|x| x.clamp(-300.0, 300.0))],
                shift: event.shift_key(),
            })
        } else if let Some(companion) = element_target(&event, "[data-companion]", "data-companion")
        {
            selected_bed.set(None);
            companions.with_untracked(|items| {
                if let Some(item) = items.iter().find(|item| item.instance_id == companion) {
                    let prior = selected.get_untracked();
                    let tree = item
                        .servicing_tree_ids
                        .iter()
                        .find(|id| id == &&prior)
                        .or_else(|| item.servicing_tree_ids.first());
                    if let Some(id) = tree {
                        refresh_selection(selected, group, vec![id.clone()]);
                    }
                    focused_plant.set(item.plant_id.clone());
                    if let Some(editor) = document().get_element_by_id("garden-companion-editor") {
                        editor.scroll_into_view();
                    }
                }
            });
            return;
        } else if let Some(id) = element_target(&event, "[data-tree-hit]", "data-tree-hit")
            .or_else(|| element_target(&event, "[data-tree]", "data-tree"))
        {
            selected_bed.set(None);
            let mut ids = group.get_untracked();
            if !ids.contains(&id) {
                if !(event.shift_key() || selecting.get_untracked()) {
                    ids.clear();
                }
                ids.push(id.clone());
            }
            let Some((primary, originals)) = draft.with_untracked(|state| {
                let tree = state
                    .garden
                    .star_plants
                    .iter()
                    .find(|tree| tree.instance_id == id)?;
                let originals = state
                    .garden
                    .star_plants
                    .iter()
                    .filter(|tree| ids.contains(&tree.instance_id))
                    .cloned()
                    .collect();
                Some(([tree.x_m, tree.y_m], originals))
            }) else {
                return;
            };
            selected.set(id);
            group.set(ids);
            Mode::Trees {
                primary,
                start: point,
                originals,
                delta: [0.0; 2],
            }
        } else if !selecting.get_untracked() && !event.shift_key() {
            if let Some(id) = element_target(&event, "[data-bed]", "data-bed") {
                selected_bed.set(Some(id));
                refresh_selection(selected, group, Vec::new());
                return;
            }
            selected_bed.set(None);
            Mode::Pan { start: point }
        } else {
            selected_bed.set(None);
            if !event.shift_key() {
                refresh_selection(selected, group, Vec::new());
            }
            Mode::Select {
                start: point,
                end: point,
                extend: event.shift_key(),
            }
        };
        event.prevent_default();
        let _ = svg.set_pointer_capture(pointer);
        active.set(Some(Motion {
            pointer,
            pixels,
            frame: current,
            mode,
        }));
    };
    let on_move = move |event: web_sys::PointerEvent| {
        let Some(svg) = event
            .current_target()
            .and_then(|target| target.dyn_into::<web_sys::SvgElement>().ok())
        else {
            return;
        };
        let pixels = [event.client_x(), event.client_y()];
        active.update(|active| {
            let Some(motion) = active
                .as_mut()
                .filter(|motion| motion.pointer == event.pointer_id())
            else {
                return;
            };
            let Some(point) = motion.frame.coordinates(pixels, &svg) else {
                return;
            };
            let drift = motion.drift(pixels);
            match &mut motion.mode {
                Mode::Draw(draw) => {
                    draw.sample(point.map(|v| v.clamp(-300.0, 300.0)), event.shift_key())
                }
                Mode::Trees {
                    primary,
                    start,
                    originals,
                    delta,
                } if drift => {
                    *delta = move_delta(*primary, *start, point, originals);
                }
                Mode::Pan { start } if drift => {
                    center.set(Some(std::array::from_fn(|axis| {
                        motion.frame.center[axis] - (point[axis] - start[axis])
                    })));
                }
                Mode::Select { end, .. } => *end = point,
                _ => {}
            }
        });
    };
    let on_up = move |event: web_sys::PointerEvent| {
        if !active.with_untracked(|active| {
            active
                .as_ref()
                .is_some_and(|motion| motion.pointer == event.pointer_id())
        }) {
            return;
        }
        let mut motion = None;
        active.update(|active| motion = active.take());
        let Some(motion) = motion else { return };
        let Some(svg) = event
            .current_target()
            .and_then(|target| target.dyn_into::<web_sys::SvgElement>().ok())
        else {
            return;
        };
        let pixels = [event.client_x(), event.client_y()];
        let Some(point) = motion.frame.coordinates(pixels, &svg) else {
            return;
        };
        let moved = motion.drift(pixels);
        match motion.mode {
            Mode::Draw(mut draw) => {
                draw.sample(point.map(|v| v.clamp(-300.0, 300.0)), event.shift_key());
                if let Some(shape) = draw.shape(true) {
                    commit_shape(draft, draw.tool.target, shape);
                    notice.set(String::new());
                    tool.set(None);
                } else {
                    notice.set(
                        match language {
                            "de" => "Zeichne eine größere Form ohne sich kreuzende Kanten.",
                            "en" => "Draw a larger shape without crossing edges.",
                            _ => "Tegn en større form uten kryssende kanter.",
                        }
                        .into(),
                    );
                }
            }
            Mode::Trees {
                primary,
                start,
                originals,
                ..
            } if moved => {
                let delta = move_delta(primary, start, point, &originals);
                let ids: Vec<_> = originals.into_iter().map(|tree| tree.instance_id).collect();
                adjust_trees(draft, &ids, delta);
            }
            Mode::Select { start, extend, .. } if moved => {
                let [x, y] = start;
                let [a, b] = point;
                let mut ids = if extend {
                    group.get_untracked()
                } else {
                    Vec::new()
                };
                draft.with_untracked(|state| {
                    for star in &state.garden.star_plants {
                        if (x.min(a)..=x.max(a)).contains(&star.x_m)
                            && (y.min(b)..=y.max(b)).contains(&star.y_m)
                            && !ids.contains(&star.instance_id)
                        {
                            ids.push(star.instance_id.clone());
                        }
                    }
                });
                refresh_selection(selected, group, ids);
            }
            Mode::Pan { .. } if !moved => refresh_selection(selected, group, Vec::new()),
            _ => {}
        }
    };

    let edit_selection = move |key: &str, command: bool, shift: bool| -> bool {
        if key == "escape" {
            active.set(None);
            tool.set(None);
            selected_bed.set(None);
            refresh_selection(selected, group, Vec::new());
            return true;
        }
        let removing = matches!(key, "delete" | "backspace") || (command && key == "x");
        let copying = command && matches!(key, "x" | "c");
        let mut chosen = group.get_untracked();
        if chosen.is_empty() {
            let focused = selected.get_untracked();
            if !focused.is_empty() {
                chosen.push(focused);
            }
        }
        let bed = selected_bed.get_untracked();
        if command && key == "v" {
            let Some(item) = clipboard.get_untracked() else {
                return false;
            };
            match item {
                Clipboard::Trees(mut trees) => {
                    let delta = trees.iter().fold([2.0_f64; 2], |delta, tree| {
                        [
                            delta[0].min((300.0 - tree.x_m).max(0.0)),
                            delta[1].min((300.0 - tree.y_m).max(0.0)),
                        ]
                    });
                    for tree in &mut trees {
                        tree.instance_id = instance_id();
                        tree.x_m += delta[0];
                        tree.y_m += delta[1];
                    }
                    let ids = trees.iter().map(|tree| tree.instance_id.clone()).collect();
                    save(draft, |state| state.garden.star_plants.append(&mut trees));
                    refresh_selection(selected, group, ids);
                    selected_bed.set(None);
                }
                Clipboard::Bed(mut bed) => {
                    if draft
                        .with_untracked(|state| state.garden.infrastructure.raised_beds.len() >= 64)
                    {
                        return false;
                    }
                    bed.id = instance_id();
                    translate_shape(&mut bed.shape, [1.0, 1.0]);
                    if bed.shape.points().iter().flatten().any(|v| v.abs() > 300.0) {
                        return false;
                    }
                    save(draft, |state| {
                        state.garden.infrastructure.raised_beds.push(bed.clone())
                    });
                    selected_bed.set(Some(bed.id.clone()));
                    clipboard.set(Some(Clipboard::Bed(bed)));
                }
            }
        } else if (copying || removing) && (bed.is_some() || !chosen.is_empty()) {
            if let Some(id) = bed {
                if copying {
                    clipboard.set(draft.with_untracked(|state| {
                        state
                            .garden
                            .infrastructure
                            .raised_beds
                            .iter()
                            .find(|bed| bed.id == id)
                            .cloned()
                            .map(Clipboard::Bed)
                    }));
                }
                if removing {
                    save(draft, |state| {
                        state
                            .garden
                            .infrastructure
                            .raised_beds
                            .retain(|bed| bed.id != id)
                    });
                    selected_bed.set(None);
                }
            } else {
                if copying {
                    clipboard.set(Some(Clipboard::Trees(draft.with_untracked(|state| {
                        state
                            .garden
                            .star_plants
                            .iter()
                            .filter(|tree| chosen.contains(&tree.instance_id))
                            .cloned()
                            .collect()
                    }))));
                }
                if removing {
                    save(draft, |state| {
                        for id in &chosen {
                            state.garden.remove(id);
                        }
                    });
                    refresh_selection(selected, group, Vec::new());
                }
            }
        } else if !command && !chosen.is_empty() {
            let step: f64 = if shift { 1.0 } else { 0.25 };
            let delta = match key {
                "arrowleft" => [-step, 0.0],
                "arrowright" => [step, 0.0],
                "arrowup" => [0.0, -step],
                "arrowdown" => [0.0, step],
                _ => return false,
            };
            let mut bounded = delta;
            draft.with_untracked(|state| {
                for star in state
                    .garden
                    .star_plants
                    .iter()
                    .filter(|star| chosen.contains(&star.instance_id))
                {
                    for (value, pos) in bounded.iter_mut().zip([star.x_m, star.y_m]) {
                        if *value > 0.0 {
                            *value = value.min((300.0 - pos).max(0.0));
                        } else if *value < 0.0 {
                            *value = value.max((-300.0 - pos).min(0.0));
                        }
                    }
                }
            });
            adjust_trees(draft, &chosen, bounded);
        } else {
            return false;
        }
        true
    };
    let keyboard = move |event: web_sys::KeyboardEvent| {
        if edit_selection(
            &event.key().to_lowercase(),
            event.ctrl_key() || event.meta_key(),
            event.shift_key(),
        ) {
            event.prevent_default();
        }
    };

    let plants_catalog = catalog.clone();
    let stars_catalog = catalog.clone();
    let hit_catalog = catalog.clone();
    let preview_catalog = catalog.clone();
    let spot_catalog = catalog.clone();
    let inspect = Callback::new(move |probe: crate::spot::Probe| {
        draft.with(|state| {
            companions.with(|plants| {
                covers.with(|areas| {
                    crate::spot::grid_query(
                        &spot_catalog,
                        probe,
                        &state.garden.star_plants,
                        plants,
                        if show_covers.get() { areas } else { &[] },
                        cover_season.get(),
                    )
                })
            })
        })
    });
    let count_label = catalog.text("gardenGridSelected");
    view! {
        <div class="grid-tools map-modes">
            <button type="button" aria-pressed=move || (!inspector.active.get() && !selecting.get()).to_string()
                title=catalog.text("gardenGridPanTool") on:click=move |_| { inspector.stop(); selecting.set(false); tool.set(None); }>
                {match language { "de" => "Verschieben", "en" => "Pan", _ => "Panorer" }}
            </button>
            <button type="button" aria-pressed=move || (!inspector.active.get() && selecting.get()).to_string()
                title=catalog.text("gardenGridSelectTool") on:click=move |_| { inspector.stop(); selecting.set(true); tool.set(None); }>
                {match language { "de" => "Auswählen", "en" => "Select", _ => "Marker" }}
            </button>
            <button type="button" disabled=move || selected.get().is_empty() && selected_bed.with(Option::is_none)
                on:click=move |_| { edit_selection("c", true, false); }>
                {match language { "de" => "Kopieren", "en" => "Copy", _ => "Kopier" }}
            </button>
            <button type="button" disabled=move || clipboard.with(Option::is_none)
                on:click=move |_| { edit_selection("v", true, false); }>
                {match language { "de" => "Einfügen", "en" => "Paste", _ => "Lim inn" }}
            </button>
            <span>{move || group.with(|ids| (ids.len() > 1).then(|| format!("{} {count_label}", ids.len())))}</span>
        </div>
        <crate::spot::SpotControl catalog=catalog.clone() inspector language />
        <svg viewBox=move || frame.get().view_box() class="garden-map" role="img" aria-label=title tabindex="0"
            class:drawing=move || tool.with(Option::is_some) class:selecting=selecting class:dropping=dropping class:spot-mode=inspector.active
            on:pointerdown=move |event| {
                if inspector.active.get_untracked() {
                    let current = frame.get_untracked();
                    inspector.pointer(&event, current.corner(), current.width, true);
                } else { on_down(event); }
            }
            on:pointermove=move |event| {
                if active.with_untracked(Option::is_none) {
                    let current = frame.get_untracked();
                    inspector.pointer(&event, current.corner(), current.width, false);
                }
                on_move(event);
            }
            on:pointerup=on_up
            on:pointerleave=move |_| inspector.clear_hover()
            on:keydown=move |event| {
                let current = frame.get_untracked();
                if !inspector.keyboard(&event, current.corner(), current.width) { keyboard(event); }
            }
            on:pointercancel=move |event: web_sys::PointerEvent| {
                if active.with_untracked(|active| active.as_ref().is_some_and(|motion| motion.pointer == event.pointer_id())) {
                    active.set(None);
                }
            }
            on:lostpointercapture=move |event: web_sys::PointerEvent| {
                if active.with_untracked(|active| active.as_ref().is_some_and(|motion| motion.pointer == event.pointer_id())) {
                    active.set(None);
                }
            }
            on:dragover=move |event: web_sys::DragEvent| {
                event.prevent_default();
                dropping.set(true);
            }
            on:dragleave=move |_| dropping.set(false)
            on:drop=move |event: web_sys::DragEvent| {
                event.prevent_default();
                dropping.set(false);
                let Some(file) = event.data_transfer().and_then(|data| data.files()).and_then(|files| files.get(0)) else { return };
                let Some(svg) = event.current_target().and_then(|target| target.dyn_into::<web_sys::SvgElement>().ok()) else { return };
                let Some(point) = frame.get_untracked().coordinates([event.client_x(), event.client_y()], &svg) else { return };
                on_import.run((file, point.map(|v| v.clamp(-300.0, 300.0))));
            }>
            <defs><pattern id="metre-grid" width=grid height=grid patternUnits="userSpaceOnUse">
                <path d=move || format!("M {} 0 H 0 V {}", grid(), grid()) class="grid-line" />
            </pattern></defs>
            <rect x=move || frame.get().corner()[0] y=move || frame.get().corner()[1]
                width=move || frame.get().width height=move || frame.get().width fill="url(#metre-grid)" />
            <path d="M -0.2 0 H 0.2 M 0 -0.2 V 0.2" class="grid-origin" pointer-events="none"/>
            {move || draft.with(|state| {
                let outline = state.garden.infrastructure.outline.as_ref().map(|shape| shape_view(shape, false));
                let beds = state.garden.infrastructure.raised_beds.iter().map(|bed| {
                    let shape = shape_view(&bed.shape, true);
                    let id = bed.id.clone();
                    let focused = selected_bed.with(|value| value.as_deref() == Some(bed.id.as_str()));
                    view! { <g data-bed=id class:focused-bed=focused>{shape}</g> }
                }).collect_view();
                view! { <g>{outline}{beds}</g> }
            })}
            {move || show_covers.get().then(|| covers.with(|items| ground_cover_view::shapes(items, cover_season.get())))}
            {move || draft.with(|state| state.garden.star_plants.iter().filter_map(|star| {
                let species = stars_catalog.trees.iter().find(|species| species.id == star.tree_id)?;
                let active = group.with(|ids| ids.contains(&star.instance_id)) || selected.with(|id| id == &star.instance_id);
                let label = garden_editor::label(star, &stars_catalog);
                Some(view! {
                    <g class="grid-tree" class:focused=active data-tree=star.instance_id.clone()>
                        <circle cx=star.x_m cy=star.y_m r=species.mature_radius_m style:fill=species.color.clone() />
                        <circle cx=star.x_m cy=star.y_m r="0.12" />
                        <title>{label}</title>
                    </g>
                })
            }).collect_view())}
            <g class="shade-pockets" pointer-events="none">
                {move || shade.with(|pockets| pockets.iter().map(|pocket| view! {
                    <circle class="grid-shade" cx=pocket.point[0] cy=pocket.point[1] r=pocket.radius />
                }).collect_view())}
            </g>
            {move || companions.with(|items| items.iter().filter_map(|plant| {
                let species = plants_catalog.plants.iter().find(|species| species.id == plant.plant_id)?;
                let covered = show_covers.get().then(|| covers.with(|covers| {
                    covers.iter().find(|cover| cover.plant_id == plant.plant_id)
                        .map(|cover| cover.detached_anchors.iter().any(|(id, _)| id == &plant.instance_id))
                })).flatten();
                if covered == Some(false) { return None; }
                let radius = if covered.is_some() { 0.07 } else { (species.spread_m / 2.0).max(0.08) };
                let highlighted = selected.with(|focus| plant.servicing_tree_ids.contains(focus));
                Some(view! {
                    <circle class="grid-companion" class:highlighted=highlighted data-companion=plant.instance_id.clone()
                        cx=plant.x_m cy=plant.y_m r=radius fill=species.color.clone()>
                        <title>{species.name.clone()}</title>
                    </circle>
                })
            }).collect_view())}
            <g class="map-conflicts" pointer-events="none">
                {move || resolution.with(|resolved| resolved.unresolved.iter().map(|warning| {
                    view! { <line data-conflict=warning.id.clone() class=warning.severity.class()
                        x1=warning.a.point[0] y1=warning.a.point[1] x2=warning.b.point[0] y2=warning.b.point[1] /> }
                }).collect_view())}
            </g>
            {move || draft.with(|state| state.garden.star_plants.iter().map(|star| {
                let label = garden_editor::label(star, &hit_catalog);
                view! {
                    <circle class="tree-hit" data-tree-hit=star.instance_id.clone()
                        cx=star.x_m cy=star.y_m r=move || (frame.get().width / 65.0).clamp(0.14, 0.42)>
                        <title>{label}</title>
                    </circle>
                }
            }).collect_view())}
            <g class="drawing-preview" pointer-events="none">
                {move || active.with(|active| active.as_ref().and_then(|motion| {
                    match &motion.mode {
                        Mode::Draw(draw) => draw.shape(false).as_ref().map(|shape| shape_view(shape, false)),
                        Mode::Trees { originals, delta, .. } if *delta != [0.0; 2] => Some(originals.iter().filter_map(|star| {
                            let species = preview_catalog.trees.iter().find(|p| p.id == star.tree_id)?;
                            Some(view! { <circle class="moving-tree" cx=star.x_m + delta[0] cy=star.y_m + delta[1] r=species.mature_radius_m /> })
                        }).collect_view().into_any()),
                        Mode::Select { start, end, .. } => Some(view! {
                            <rect class="grid-marquee" x=start[0].min(end[0]) y=start[1].min(end[1])
                                width=(start[0] - end[0]).abs() height=(start[1] - end[1]).abs() />
                        }.into_any()),
                        _ => None,
                    }
                }))}
            </g>
            {inspector.marker()}
        </svg>
        <crate::spot::SpotReadout catalog=catalog.clone() inspector inspect language />
    }
}
