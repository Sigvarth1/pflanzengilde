use crate::{
    catalog::{Catalog, Species},
    garden::{Infrastructure, Shape, Star},
    placement::{Sector, Zone},
};
use garden_core::{relax, round_cm, Outline, RECORD_LEN};
use serde::Serialize;
use std::{collections::HashMap, f64::consts::PI};

const EPS: f64 = 1e-9;

struct Tree<'a> {
    star: &'a Star,
    species: &'a Species,
    compartment: Option<usize>,
    plants: Vec<usize>,
    requested_count: usize,
    other_ids: Vec<String>,
    outer_angle: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Companion {
    pub instance_id: String,
    pub plant_id: String,
    pub x_m: f64,
    pub y_m: f64,
    pub servicing_tree_ids: Vec<String>,
    pub is_merged: bool,
    pub current_light_condition: &'static str,
    pub is_key_pest_defense: bool,
    #[serde(skip)]
    species: usize,
    #[serde(skip)]
    trees: Vec<usize>,
}

struct Boundary<'a> {
    shape: &'a Shape,
    centroid: [f64; 2],
    vertices: Vec<f64>,
    area: f64,
}

impl<'a> Boundary<'a> {
    fn new(shape: &'a Shape) -> Self {
        let (mut centroid, mut vertices) = ([0.0; 2], Vec::new());
        let area;
        match shape {
            Shape::Circle { cx_m, cy_m, r_m } => {
                centroid = [*cx_m, *cy_m];
                area = PI * r_m * r_m;
            }
            Shape::Rect { x_m, y_m, w_m, h_m } => {
                centroid = [x_m + w_m / 2.0, y_m + h_m / 2.0];
                area = (w_m * h_m).abs();
                vertices.extend(shape.points().into_iter().flatten());
            }
            Shape::Polygon { points } => {
                let mut cross_sum = 0.0;
                let mut mean = [0.0; 2];
                for (i, [x, y]) in points.iter().copied().enumerate() {
                    let [nx, ny] = points[(i + 1) % points.len()];
                    let cross = x * ny - nx * y;
                    cross_sum += cross;
                    centroid[0] += (x + nx) * cross;
                    centroid[1] += (y + ny) * cross;
                    mean[0] += x;
                    mean[1] += y;
                    vertices.extend([x, y]);
                }
                area = cross_sum.abs() / 2.0;
                centroid = if cross_sum.abs() < EPS {
                    mean.map(|v| v / points.len().max(1) as f64)
                } else {
                    centroid.map(|v| v / (3.0 * cross_sum))
                };
            }
        }
        Self {
            shape,
            centroid,
            vertices,
            area,
        }
    }

    fn outline(&self) -> Outline<'_> {
        match self.shape {
            Shape::Circle { r_m, .. } => Outline::Circle {
                center: self.centroid,
                radius: *r_m,
            },
            _ => Outline::Polygon {
                center: self.centroid,
                vertices: &self.vertices,
                rectangle: matches!(self.shape, Shape::Rect { .. }),
            },
        }
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        self.shape.contains([x, y])
    }
}

fn compartment(beds: &[Boundary<'_>], x: f64, y: f64) -> Option<usize> {
    beds.iter()
        .enumerate()
        .filter(|(_, bed)| bed.contains(x, y))
        .min_by(|(_, a), (_, b)| a.area.total_cmp(&b.area))
        .map(|(i, _)| i)
}

fn distance(a: &Star, b: &Star) -> f64 {
    (b.x_m - a.x_m).hypot(b.y_m - a.y_m)
}

fn js_round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

fn hash(input: &str) -> i32 {
    input.encode_utf16().fold(0i32, |hash, value| {
        hash.wrapping_mul(31).wrapping_add(i32::from(value))
    })
}

pub(crate) fn collar(plant: &Species) -> bool {
    plant.allium()
        || plant.layer == "BULB_ROOT"
        || plant.zone == Zone::Bulb
        || plant.id == "plant-daffodil"
}

fn role(plant: &Species, id: &str) -> bool {
    plant.role_ids.iter().any(|role| role == id)
}

fn insectary(plant: &Species) -> bool {
    role(plant, "POLLINATOR_MAGNET")
        || ["Achillea", "Foeniculum", "Daucus", "Aster"]
            .iter()
            .any(|genus| plant.botanical_name.contains(genus))
}

fn radius(plant: &Species, insectary: bool) -> f64 {
    if insectary {
        return 4.5;
    }
    if plant.nitrogen_fixer {
        if plant.layer == "CANOPY" || plant.botanical_name.contains("Alnus") {
            return 4.0;
        }
        if plant.layer == "SHRUB"
            || ["Elaeagnus", "Hippophae"]
                .iter()
                .any(|genus| plant.botanical_name.contains(genus))
        {
            return 2.5;
        }
        return 1.8;
    }
    if [
        "DYNAMIC_ACCUMULATOR",
        "BIOMASS_PRODUCER",
        "PEST_REPELLER",
        "ANTIFUNGAL",
    ]
    .iter()
    .any(|id| role(plant, id))
    {
        return 1.8;
    }
    if collar(plant) {
        0.8
    } else {
        1.5
    }
}

fn crosses_wall(plant: &Species) -> bool {
    !plant.role_ids.is_empty()
        && plant.role_ids.iter().all(|id| {
            id == "POLLINATOR_MAGNET"
                || (id == "PEST_REPELLER" && plant.id != "plant-sorghum-sudangrass")
        })
}

fn light(x: f64, y: f64, trees: &[Tree<'_>], south: bool) -> &'static str {
    let mut shade = false;
    let mut crowns = 0;
    for tree in trees {
        let dy = y - tree.star.y_m;
        let d = (x - tree.star.x_m).hypot(dy);
        let radius = tree.species.mature_radius_m;
        crowns += usize::from(d <= radius + EPS);
        let poleward = if south { dy > 0.0 } else { dy < 0.0 };
        shade |= poleward && d <= radius * 1.3 + EPS;
    }
    match (crowns, shade) {
        (0, false) => "FULL_SUN",
        (1, true) | (2.., _) => "FULL_SHADE",
        _ => "PARTIAL_SUN",
    }
}

fn base_angle(plant: &Species, south: bool) -> f64 {
    if plant.allium() {
        return if south { 60.0 } else { 120.0 };
    }
    if plant.legume() {
        return if south { 240.0 } else { 300.0 };
    }
    match (plant.sector, south) {
        (Sector::NorthShade, false) | (Sector::SouthSun, true) => 0.0,
        (Sector::NorthShade, true) | (Sector::SouthSun, false) => 180.0,
        (Sector::EastMorning, _) => 90.0,
        (Sector::WestWind, _) => 270.0,
        (Sector::Any, _) => f64::from(hash(&plant.id).unsigned_abs() % 360),
    }
}

fn individual(tree: &Tree<'_>, plant: &Species, species_index: usize, south: bool) -> (f64, f64) {
    let index = tree
        .plants
        .iter()
        .position(|&i| i == species_index)
        .unwrap_or(0);
    let canopy = tree.species.mature_radius_m;
    let r = if collar(plant) {
        (canopy * 0.22 + (index % 3) as f64 * 0.08).clamp(0.45, 0.75)
    } else {
        match plant.zone {
            Zone::Bulb => (canopy * 0.35 + (index % 2) as f64 * 0.2).max(0.6),
            Zone::Mid => (canopy * 0.65 + ((index % 3) as f64 - 1.0) * 0.25).max(0.9),
            Zone::Drip => (canopy * 0.95 + ((index % 3) as f64 - 1.0) * 0.3).max(1.2),
            Zone::Outer => (canopy * 1.25 + (index % 2) as f64 * 0.35).max(1.5),
            Zone::Collar => canopy * 0.7,
        }
    };
    let spread = (index as f64 - (tree.requested_count - 1) as f64 / 2.0)
        * (360.0 / tree.requested_count.max(6) as f64);
    let key = format!(
        "{}|{}|n:{}",
        plant.id,
        tree.star.tree_id,
        tree.other_ids.join(";")
    );
    let jitter = f64::from(hash(&key).unsigned_abs() % 31) - 15.0;
    let angle = if plant.zone == Zone::Outer
        || role(plant, "PEST_REPELLER")
        || role(plant, "GRASS_BARRIER")
    {
        tree.outer_angle
    } else {
        None
    };
    let deg =
        (angle.unwrap_or_else(|| base_angle(plant, south) + spread * 0.4) + jitter + 360.0) % 360.0;
    let rad = (deg - 90.0) * PI / 180.0;
    (
        round_cm(tree.star.x_m + r * rad.cos()),
        round_cm(tree.star.y_m + r * rad.sin()),
    )
}

fn prepare_trees<'a>(
    catalog: &'a Catalog,
    stars: &'a [Star],
    beds: &[Boundary<'_>],
    sources: &serde_json::Map<String, serde_json::Value>,
    overrides: Option<&HashMap<String, Vec<String>>>,
) -> Vec<Tree<'a>> {
    let indexed: HashMap<_, _> = catalog
        .plants
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();
    let known: Vec<_> = stars
        .iter()
        .filter_map(|star| {
            catalog
                .trees
                .iter()
                .find(|species| species.id == star.tree_id)
                .map(|species| (star, species))
        })
        .collect();
    known
        .iter()
        .map(|&(star, species)| {
            let defaults: Vec<_> = sources
                .get(&star.tree_id)
                .and_then(|source| source.get("recommendedCompanions"))
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .collect();
            let ids: Vec<_> =
                if let Some(ids) = overrides.and_then(|ids| ids.get(&star.instance_id)) {
                    ids.iter().map(String::as_str).collect()
                } else if !star.selected_plant_ids.is_empty() {
                    star.selected_plant_ids.iter().map(String::as_str).collect()
                } else if !defaults.is_empty() {
                    defaults
                } else if !species.recommended_companions.is_empty() {
                    species
                        .recommended_companions
                        .iter()
                        .map(String::as_str)
                        .collect()
                } else {
                    [
                        "plant-comfrey",
                        "plant-white-clover",
                        "plant-chives",
                        "plant-daffodil",
                        "plant-yarrow",
                    ]
                    .to_vec()
                };
            let requested_count = ids
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .len();
            let mut plants: Vec<_> = ids
                .into_iter()
                .filter_map(|id| indexed.get(id).copied())
                .collect();
            plants.sort_unstable();
            plants.dedup();
            let (mut dx, mut dy) = (0.0, 0.0);
            let mut other_ids = Vec::new();
            for &(other, variety) in &known {
                if other.instance_id == star.instance_id {
                    continue;
                }
                let range =
                    (0.95 * (species.mature_radius_m + variety.mature_radius_m) + 4.5).max(9.0);
                let d = distance(star, other);
                if d > range + EPS {
                    continue;
                }
                let (x, y) = (other.x_m - star.x_m, other.y_m - star.y_m);
                if d > 0.01 + EPS {
                    dx += x / d;
                    dy += y / d;
                }
                // JavaScript stringifies negative zero as zero.
                let rx = js_round(x * 100.0) as i64;
                let ry = js_round(y * 100.0) as i64;
                other_ids.push(format!("{rx},{ry}"));
            }
            other_ids.sort();
            let outer_angle = (dx.hypot(dy) > 0.1 + EPS)
                .then(|| (((-dy).atan2(-dx) * 180.0) / PI + 450.0) % 360.0);
            Tree {
                star,
                species,
                compartment: compartment(beds, star.x_m, star.y_m),
                plants,
                requested_count,
                other_ids,
                outer_angle,
            }
        })
        .collect()
}

fn position(
    plant: &Species,
    species: usize,
    a: usize,
    b: Option<(usize, f64)>,
    c: Option<usize>,
    trees: &[Tree<'_>],
    south: bool,
) -> Companion {
    let tree = trees[a].star;
    let mut serving = vec![a];
    let (x, y) = if let Some((b, distance)) = b {
        serving.push(b);
        let other = trees[b].star;
        if let Some(c) = c {
            serving.push(c);
            let third = trees[c].star;
            (
                round_cm((tree.x_m + other.x_m + third.x_m) / 3.0),
                round_cm((tree.y_m + other.y_m + third.y_m) / 3.0),
            )
        } else {
            let (dx, dy) = if distance > 0.0 {
                (
                    (other.x_m - tree.x_m) / distance,
                    (other.y_m - tree.y_m) / distance,
                )
            } else {
                (1.0, 0.0)
            };
            let sun = if (!south && dx >= 0.0) || (south && dx <= 0.0) {
                0.35
            } else {
                -0.35
            };
            (
                round_cm((tree.x_m + other.x_m) / 2.0 + -dy * sun),
                round_cm((tree.y_m + other.y_m) / 2.0 + dx * sun),
            )
        }
    } else {
        individual(&trees[a], plant, species, south)
    };
    let ids: Vec<_> = serving
        .iter()
        .map(|&i| trees[i].star.instance_id.clone())
        .collect();
    let is_merged = ids.len() > 1;
    let instance_id = format!(
        "{}-{}{}",
        plant.id,
        if is_merged { "merged-" } else { "" },
        ids.join("-")
    );
    Companion {
        instance_id,
        plant_id: plant.id.clone(),
        x_m: x,
        y_m: y,
        servicing_tree_ids: ids,
        is_merged,
        current_light_condition: light(x, y, trees, south),
        is_key_pest_defense: serving
            .iter()
            .any(|&i| plant.pest_defense_for.contains(&trees[i].star.tree_id)),
        species,
        trees: serving,
    }
}

pub fn place(
    catalog: &Catalog,
    stars: &[Star],
    infrastructure: &Infrastructure,
    sources: &serde_json::Map<String, serde_json::Value>,
    south: bool,
) -> Vec<Companion> {
    place_with_ids(catalog, stars, infrastructure, sources, south, None)
}

pub fn place_with_ids(
    catalog: &Catalog,
    stars: &[Star],
    infrastructure: &Infrastructure,
    sources: &serde_json::Map<String, serde_json::Value>,
    south: bool,
    overrides: Option<&HashMap<String, Vec<String>>>,
) -> Vec<Companion> {
    let beds: Vec<_> = infrastructure
        .raised_beds
        .iter()
        .map(|bed| Boundary::new(&bed.shape))
        .collect();
    let boundary = infrastructure.outline.as_ref().map(Boundary::new);
    let outline = boundary.as_ref().map(Boundary::outline);
    let trees = prepare_trees(catalog, stars, &beds, sources, overrides);
    let mut companions = Vec::new();
    let mut requests = vec![Vec::new(); catalog.plants.len()];
    for (i, tree) in trees.iter().enumerate() {
        for &plant in &tree.plants {
            requests[plant].push(i);
        }
    }
    for (species, mut requesting) in requests.into_iter().enumerate() {
        if requesting.is_empty() {
            continue;
        }
        let plant = &catalog.plants[species];
        if collar(plant) {
            companions.extend(
                requesting
                    .into_iter()
                    .map(|a| position(plant, species, a, None, None, &trees, south)),
            );
            continue;
        }
        requesting.sort_by(|&a, &b| {
            let a = trees[a].star;
            let b = trees[b].star;
            let compare = |value: f64, other: f64| {
                value
                    .partial_cmp(&other)
                    .unwrap_or(std::cmp::Ordering::Equal)
            };
            compare(a.x_m, b.x_m).then_with(|| compare(a.y_m, b.y_m))
        });
        let is_insectary = insectary(plant);
        let benefit = radius(plant, is_insectary);
        let crosses = beds.is_empty() || crosses_wall(plant);
        let mut serviced = vec![false; trees.len()];
        for &a in &requesting {
            if serviced[a] {
                continue;
            }
            let mut partner = None;
            let mut min_distance = f64::INFINITY;
            for &b in &requesting {
                if b == a
                    || serviced[b]
                    || (!crosses && trees[a].compartment != trees[b].compartment)
                {
                    continue;
                }
                let d = distance(trees[a].star, trees[b].star);
                let limit = if is_insectary {
                    benefit * 2.0
                } else {
                    (trees[a].species.mature_radius_m + trees[b].species.mature_radius_m) * 0.95
                        + benefit
                };
                if d <= limit + EPS && d < min_distance - EPS {
                    partner = Some(b);
                    min_distance = d;
                }
            }
            let third = partner.filter(|_| is_insectary).and_then(|b| {
                requesting.iter().copied().find(|&c| {
                    if c == a
                        || c == b
                        || serviced[c]
                        || (!crosses && trees[a].compartment != trees[c].compartment)
                    {
                        return false;
                    }
                    let [a, b, c] = [a, b, c].map(|i| trees[i].star);
                    let (x, y) = ((a.x_m + b.x_m + c.x_m) / 3.0, (a.y_m + b.y_m + c.y_m) / 3.0);
                    [a, b, c]
                        .into_iter()
                        .all(|tree| (tree.x_m - x).hypot(tree.y_m - y) <= benefit + EPS)
                })
            });
            companions.push(position(
                plant,
                species,
                a,
                partner.map(|b| (b, min_distance)),
                third,
                &trees,
                south,
            ));
            serviced[a] = true;
            if let Some(b) = partner {
                serviced[b] = true;
            }
            if let Some(c) = third {
                serviced[c] = true;
            }
        }
    }
    finish(
        &mut companions,
        catalog,
        &trees,
        &beds,
        outline.as_ref(),
        south,
    );
    companions
}

fn settle_trunks(record: &mut [f64], trees: &[Tree<'_>], outline: Option<&Outline<'_>>) {
    let (x, y) = (record[0], record[1]);
    let spread = record[2];
    let allium = record[3] as u32 & 1 != 0;
    let clearance = |tree: &Tree<'_>| {
        if allium && tree.species.nitrogen_fixer {
            1.95
        } else {
            0.4
        }
    };
    let allowed = |x: f64, y: f64| {
        trees
            .iter()
            .all(|tree| (x - tree.star.x_m).hypot(y - tree.star.y_m) >= clearance(tree) - EPS)
            && outline.is_none_or(|outline| inside(outline, x, y, spread))
    };
    if allowed(x, y) {
        return;
    }
    let nearby: Vec<_> = trees
        .iter()
        .filter_map(|tree| {
            let (cx, cy, r) = (tree.star.x_m, tree.star.y_m, clearance(tree) + 0.01);
            ((x - cx).hypot(y - cy) < r + 4.0).then_some((cx, cy, r))
        })
        .collect();
    let mut best = (x, y, f64::INFINITY);
    let mut consider = |cx: f64, cy: f64| {
        let (cx, cy) = (round_cm(cx), round_cm(cy));
        if !allowed(cx, cy) {
            return;
        }
        let d = (cx - x).hypot(cy - y);
        if d < best.2 - EPS {
            best = (cx, cy, d);
        }
    };
    for &(cx, cy, r) in &nearby {
        let (dx, dy) = (x - cx, y - cy);
        let angle = if dx.hypot(dy) > 0.01 + EPS {
            dy.atan2(dx)
        } else {
            0.5
        };
        consider(cx + r * angle.cos(), cy + r * angle.sin());
    }
    for (i, &(ax, ay, ar)) in nearby.iter().enumerate() {
        for &(bx, by, br) in &nearby[i + 1..] {
            let (dx, dy) = (bx - ax, by - ay);
            let d = dx.hypot(dy);
            if d < 1e-6 || d > ar + br || d < (ar - br).abs() {
                continue;
            }
            let a = (ar * ar - br * br + d * d) / (2.0 * d);
            let h = (ar * ar - a * a).max(0.0).sqrt();
            let (mx, my) = (ax + a * dx / d, ay + a * dy / d);
            consider(mx + h * dy / d, my - h * dx / d);
            consider(mx - h * dy / d, my + h * dx / d);
        }
    }
    record[0] = best.0;
    record[1] = best.1;
}

fn inside(outline: &Outline<'_>, x: f64, y: f64, spread: f64) -> bool {
    outline.border(x, y).0 <= -(spread / 2.0).clamp(0.15, 0.5) + 0.01
}

fn settle_compartment(
    record: &mut [f64],
    companion: &Companion,
    trees: &[Tree<'_>],
    beds: &[Boundary<'_>],
) {
    let target = trees[companion.trees[0]].compartment;
    if companion
        .trees
        .iter()
        .any(|&i| trees[i].compartment != target)
    {
        return;
    }
    let (x, y) = (record[0], record[1]);
    if compartment(beds, x, y) == target {
        return;
    }
    let [x, y] = if let Some(bed) = target {
        beds[bed].outline().nearest_inside(x, y, 0.15)
    } else {
        let mut point = [x, y];
        for _ in 0..3 {
            let Some(bed) = beds.iter().find(|bed| bed.contains(point[0], point[1])) else {
                break;
            };
            point = bed.outline().nearest_outside(point[0], point[1], 0.1);
        }
        point
    };
    record[0] = round_cm(x);
    record[1] = round_cm(y);
}

fn finish(
    companions: &mut [Companion],
    catalog: &Catalog,
    trees: &[Tree<'_>],
    beds: &[Boundary<'_>],
    outline: Option<&Outline<'_>>,
    south: bool,
) {
    let positions: Vec<_> = trees
        .iter()
        .flat_map(|tree| {
            [
                tree.star.x_m,
                tree.star.y_m,
                if tree.species.nitrogen_fixer {
                    1.0
                } else {
                    0.0
                },
            ]
        })
        .collect();
    let mut records: Vec<_> = companions
        .iter()
        .flat_map(|item| {
            let plant = &catalog.plants[item.species];
            let flags = u8::from(plant.allium())
                | u8::from(plant.legume()) << 1
                | u8::from(plant.fennel()) << 2
                | u8::from(plant.wormwood()) << 3;
            let power = plant
                .id
                .encode_utf16()
                .fold(1i32, |multiplier, _| multiplier.wrapping_mul(31));
            [
                item.x_m,
                item.y_m,
                plant.spread_m,
                f64::from(flags),
                f64::from(hash(&plant.id)),
                f64::from(power),
            ]
        })
        .collect();
    relax(&mut records, &positions, outline);
    for (item, record) in companions
        .iter_mut()
        .zip(records.chunks_exact_mut(RECORD_LEN))
    {
        settle_trunks(record, trees, outline);
        if !beds.is_empty() {
            settle_compartment(record, item, trees, beds);
        }
        if let Some(outline) = outline {
            if !inside(outline, record[0], record[1], record[2]) {
                outline.constrain(record);
                settle_trunks(record, trees, Some(outline));
            }
            item.current_light_condition = light(record[0], record[1], trees, south);
        }
        item.x_m = record[0];
        item.y_m = record[1];
    }
}
