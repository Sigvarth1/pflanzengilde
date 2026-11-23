use crate::{
    catalog::{Catalog, Species},
    guild::Guild,
    placement::{cm, Metrics, Placement, Zone},
};
use std::{collections::HashMap, f64::consts::PI};

#[derive(Clone, PartialEq)]
pub struct Alternative {
    pub plant: usize,
    pub shared_roles: Vec<String>,
    pub reduction_m: f64,
    pub reduction_percent: u32,
}

#[derive(PartialEq)]
pub struct Conflict {
    pub plants: [String; 2],
    pub distance_m: f64,
    pub required_m: f64,
    pub overlap_m: f64,
    pub overlap: u32,
    pub critical: bool,
}

#[derive(PartialEq)]
pub struct Saturation {
    pub zone: Zone,
    pub percent: u32,
    pub plants: usize,
    pub zone_area: f64,
    pub plant_area: f64,
}

#[derive(PartialEq)]
pub struct Report {
    pub conflicts: Vec<Conflict>,
    pub crowded: Vec<Saturation>,
    pub alternatives: HashMap<String, Vec<Alternative>>,
}

pub fn alternatives(
    catalog: &Catalog,
    tree: &Species,
    original: &Species,
    guild: &Guild,
) -> Vec<Alternative> {
    let others: Vec<_> = guild
        .plant_ids
        .iter()
        .filter(|id| **id != original.id)
        .cloned()
        .collect();
    let mut candidates: Vec<_> = catalog
        .plants
        .iter()
        .enumerate()
        .filter_map(|(index, plant)| {
            if plant.retired
                || plant.id == original.id
                || others.contains(&plant.id)
                || plant.unsuitable_soils.contains(&guild.soil_type)
                || !plant.climate_zones.contains(&guild.climate_zone)
            {
                return None;
            }
            let roles: Vec<_> = plant
                .role_ids
                .iter()
                .filter(|role| original.role_ids.contains(role))
                .cloned()
                .collect();
            let reduction = original.spread_m - plant.spread_m;
            let vertical = matches!(original.layer.as_str(), "SHRUB" | "HERBACEOUS")
                && matches!(plant.layer.as_str(), "GROUND_COVER" | "BULB_ROOT");
            if roles.is_empty()
                || (reduction < 0.15 && !vertical)
                || !catalog.incompatibilities(plant, tree, &others).is_empty()
            {
                return None;
            }
            let percent = if original.spread_m > 0.0 {
                (reduction / original.spread_m * 100.0).max(0.0).round() as u32
            } else {
                0
            };
            let recommended = tree.recommended_companions.contains(&plant.id)
                || plant.recommended_for_trees.contains(&tree.id);
            let score = roles.len() as u32 * 20
                + percent
                + if recommended { 15 } else { 0 }
                + if plant.suitable_soils.contains(&guild.soil_type) {
                    10
                } else {
                    0
                };
            Some((
                score,
                Alternative {
                    plant: index,
                    shared_roles: roles,
                    reduction_m: reduction.max(0.0),
                    reduction_percent: percent,
                },
            ))
        })
        .collect();
    candidates.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    candidates
        .into_iter()
        .take(3)
        .map(|(_, option)| option)
        .collect()
}

pub fn analyze(
    catalog: &Catalog,
    tree: &Species,
    guild: &Guild,
    positions: &[Placement<'_>],
    metrics: &Metrics,
) -> Report {
    let clumps: Vec<_> = positions
        .iter()
        .filter(|position| !position.species.area_plant)
        .collect();
    let zones = [Zone::Bulb, Zone::Mid, Zone::Drip, Zone::Outer];
    let crowded = zones
        .into_iter()
        .enumerate()
        .filter_map(|(i, zone)| {
            let zone_area = PI * (metrics.rings[i + 1].powi(2) - metrics.rings[i].powi(2));
            let (plants, plant_area) = clumps.iter().filter(|plant| plant.zone == zone).fold(
                (0, 0.0),
                |(count, area), plant| {
                    (
                        count + 1,
                        area + PI * (plant.species.spread_m / 2.0).powi(2),
                    )
                },
            );
            let percent = (plant_area / zone_area.max(0.1) * 100.0).round() as u32;
            (percent > 85).then_some(Saturation {
                zone,
                percent,
                plants,
                zone_area,
                plant_area,
            })
        })
        .collect();

    let mut conflicts = Vec::new();
    let mut suggestions = HashMap::new();
    for (i, &left) in clumps.iter().enumerate() {
        for &right in clumps.iter().skip(i + 1) {
            let Some(conflict) = conflict(left, right) else {
                continue;
            };
            for plant in [left.species, right.species] {
                suggestions
                    .entry(plant.id.clone())
                    .or_insert_with(|| alternatives(catalog, tree, plant, guild));
            }
            conflicts.push(conflict);
        }
    }
    conflicts.sort_by_key(|item| (!item.critical, std::cmp::Reverse(item.overlap)));
    Report {
        conflicts,
        crowded,
        alternatives: suggestions,
    }
}

fn conflict(left: &Placement<'_>, right: &Placement<'_>) -> Option<Conflict> {
    let required_m = (left.species.spread_m + right.species.spread_m) / 2.0;
    let distance_m = crate::placement::polar_distance(left, right.distance, right.angle);
    if distance_m >= required_m {
        return None;
    }
    let overlap = ((required_m - distance_m) / required_m * 100.0).round() as u32;
    let competing = left.species.competes_with(right.species);
    (overlap >= if competing { 30 } else { 65 }).then(|| Conflict {
        plants: [left.species.id.clone(), right.species.id.clone()],
        distance_m: cm(distance_m),
        required_m: cm(required_m),
        overlap_m: cm(required_m - distance_m),
        overlap,
        critical: competing && overlap >= 50,
    })
}
