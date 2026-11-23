use crate::{
    calendar::Season,
    catalog::{Catalog, Species},
    guild::Guild,
};
use std::collections::HashSet;

pub const ROLES: [&str; 7] = [
    "POLLINATOR_MAGNET",
    "LIVING_MULCH",
    "GRASS_BARRIER",
    "PEST_REPELLER",
    "BIOMASS_PRODUCER",
    "NITROGEN_FIXER",
    "DYNAMIC_ACCUMULATOR",
];
pub(crate) const ALL_ROLES: [&str; 9] = [
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Filter {
    pub role: &'static str,
    pub season: Season,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Coverage {
    pub role: &'static str,
    pub across: [Vec<usize>; 5],
}

#[derive(Clone, PartialEq, Eq)]
pub struct Gap {
    pub filter: Filter,
    pub suggestions: Vec<usize>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Redundant {
    pub plant: usize,
    pub covering: Vec<usize>,
    pub alternatives: Vec<usize>,
}

impl Species {
    pub fn covers(&self, filter: Filter) -> bool {
        if !self.role_ids.iter().any(|role| role == filter.role) {
            return false;
        }
        let seasons = match filter.role {
            "POLLINATOR_MAGNET" => &self.activity.flowering,
            "LIVING_MULCH" => &self.activity.foliage,
            "PEST_REPELLER" => &self.activity.pest_deterrence,
            "BIOMASS_PRODUCER" => &self.activity.chop,
            _ => &self.activity.active,
        };
        seasons.contains(&filter.season)
    }

    pub fn suitable(&self, guild: &Guild) -> bool {
        self.climate_zones.contains(&guild.climate_zone)
            && !self.unsuitable_soils.contains(&guild.soil_type)
    }
}

fn layer_rank(layer: &str) -> u8 {
    match layer {
        "GROUND_COVER" | "BULB_ROOT" => 0,
        "HERBACEOUS" => 1,
        "VINE" => 2,
        "SHRUB" => 3,
        "SUB_CANOPY" => 4,
        "CANOPY" => 5,
        _ => 6,
    }
}

fn selected(catalog: &Catalog, guild: &Guild) -> Vec<usize> {
    guild
        .plant_ids
        .iter()
        .filter_map(|id| catalog.plants.iter().position(|plant| &plant.id == id))
        .collect()
}

pub fn coverage(catalog: &Catalog, guild: &Guild) -> Vec<Coverage> {
    let selected = selected(catalog, guild);
    ROLES
        .into_iter()
        .map(|role| Coverage {
            role,
            across: Season::ALL.map(|season| {
                selected
                    .iter()
                    .copied()
                    .filter(|&i| catalog.plants[i].covers(Filter { role, season }))
                    .collect()
            }),
        })
        .collect()
}

pub fn candidates(
    catalog: &Catalog,
    guild: &Guild,
    filter: Filter,
    environment: bool,
) -> Vec<usize> {
    let star = catalog.trees.iter().find(|tree| tree.id == guild.tree_id);
    let mut result: Vec<_> = catalog
        .plants
        .iter()
        .enumerate()
        .filter(|(_, plant)| {
            !plant.retired
                && !guild.plant_ids.contains(&plant.id)
                && plant.covers(filter)
                && (!environment || plant.suitable(guild))
                && catalog.guild_incompatibilities(plant, guild).is_empty()
        })
        .map(|(index, _)| index)
        .collect();
    result.sort_by_key(|&index| {
        let plant = &catalog.plants[index];
        let recommended = star.is_some_and(|tree| {
            tree.recommended_companions.contains(&plant.id)
                || plant.recommended_for_trees.contains(&tree.id)
        });
        (
            !recommended,
            layer_rank(&plant.layer),
            std::cmp::Reverse(plant.role_ids.len()),
            index,
        )
    });
    result
}

pub fn gaps(
    catalog: &Catalog,
    guild: &Guild,
    coverage: &[Coverage],
    environment: bool,
) -> Vec<Gap> {
    use Season::*;
    let selected = selected(catalog, guild);
    let mut result = Vec::new();
    for row in coverage {
        let seasons = match row.role {
            "POLLINATOR_MAGNET" => &[EarlySpring, LateSpring, Summer, Autumn][..],
            "LIVING_MULCH" | "PEST_REPELLER" => &[Winter],
            "GRASS_BARRIER" => &[Summer],
            _ => continue,
        };
        if !selected
            .iter()
            .any(|&i| catalog.plants[i].role_ids.iter().any(|id| id == row.role))
        {
            continue;
        }
        for &season in seasons {
            let slot = Season::ALL
                .iter()
                .position(|&entry| entry == season)
                .expect("known season");
            if !row.across[slot].is_empty() {
                continue;
            }
            let filter = Filter {
                role: row.role,
                season,
            };
            let mut suggestions = candidates(catalog, guild, filter, environment);
            suggestions.truncate(4);
            if !suggestions.is_empty() {
                result.push(Gap {
                    filter,
                    suggestions,
                });
            }
        }
    }
    result
}

/// An evidence-backed protection should remain even when its ecological roles appear elsewhere.
pub fn redundant(catalog: &Catalog, guild: &Guild, environment: bool) -> Vec<Redundant> {
    let selected = selected(catalog, guild);
    if selected.len() < 2 {
        return Vec::new();
    }
    let absent: HashSet<_> = ALL_ROLES
        .into_iter()
        .filter(|role| {
            !selected
                .iter()
                .any(|&i| catalog.plants[i].role_ids.iter().any(|id| id == role))
        })
        .collect();
    let mut result = Vec::new();
    for &index in &selected {
        let plant = &catalog.plants[index];
        if plant.pest_defense_for.contains(&guild.tree_id) || plant.role_ids.is_empty() {
            continue;
        }
        let mut covering = Vec::new();
        let mut seen = HashSet::new();
        let mut fully_covered = true;
        for id in &plant.role_ids {
            let Some(&role) = ALL_ROLES.iter().find(|&&role| role == id) else {
                continue;
            };
            for season in Season::ALL {
                let filter = Filter { role, season };
                if !plant.covers(filter) {
                    continue;
                }
                let mut found = false;
                for i in selected
                    .iter()
                    .copied()
                    .filter(|&i| catalog.plants[i].id != plant.id)
                {
                    if catalog.plants[i].covers(filter) {
                        found = true;
                        if seen.insert(i) {
                            covering.push(i);
                        }
                    }
                }
                if !found {
                    fully_covered = false;
                    break;
                }
            }
            if !fully_covered {
                break;
            }
        }
        if !fully_covered || covering.is_empty() {
            continue;
        }
        let mut remainder = guild.clone();
        remainder.plant_ids.retain(|id| id != &plant.id);
        let alternatives = catalog
            .plants
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                !candidate.retired
                    && !guild.plant_ids.contains(&candidate.id)
                    && candidate
                        .role_ids
                        .iter()
                        .any(|role| absent.contains(role.as_str()))
                    && (!environment || candidate.suitable(guild))
                    && catalog
                        .guild_incompatibilities(candidate, &remainder)
                        .is_empty()
            })
            .take(3)
            .map(|(index, _)| index)
            .collect();
        result.push(Redundant {
            plant: index,
            covering,
            alternatives,
        });
    }
    result
}
