use crate::{
    catalog::{Catalog, Species},
    compatibility::ph_pair,
    garden::{Garden, Settings, Star},
    garden_conflicts::{self, distance, Compartments, Conflict},
    garden_layout::{self, Companion},
};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

pub type Selections = HashMap<String, Vec<String>>;
const MAX_SWAPS: usize = 24;
const DEFAULTS: [&str; 5] = [
    "plant-comfrey",
    "plant-white-clover",
    "plant-chives",
    "plant-daffodil",
    "plant-yarrow",
];

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Substitution {
    pub id: String,
    pub removed_plant_id: String,
    pub added_plant_ids: Vec<String>,
    pub served_tree_ids: Vec<String>,
    pub instance_id: String,
    pub conflict: Conflict,
    pub conflict_ids: Vec<String>,
    pub roles_preserved: Vec<String>,
    pub roles_lost: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub companions: Vec<Companion>,
    pub stats: Stats,
    pub effective_companion_ids: Selections,
    pub substitutions: Vec<Substitution>,
    pub suggestions: Vec<Substitution>,
    pub unresolved: Vec<Conflict>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub star_plant_count: usize,
    pub companion_plant_count: usize,
    pub unoptimized_companion_count: usize,
    pub plants_saved: usize,
    pub savings_percent: usize,
    pub covered_roles: usize,
    pub critical_conflicts_count: usize,
    pub warning_conflicts_count: usize,
}

impl Resolution {
    pub fn apply(&self, garden: &mut Garden, sub: &Substitution) {
        for tree in garden
            .star_plants
            .iter_mut()
            .filter(|tree| sub.served_tree_ids.contains(&tree.instance_id))
        {
            let mut ids = self
                .effective_companion_ids
                .get(&tree.instance_id)
                .cloned()
                .unwrap_or_default();
            swap(&mut ids, &sub.removed_plant_id, &sub.added_plant_ids);
            if !ids.is_empty() {
                tree.selected_plant_ids = ids;
            }
        }
        let removed: HashSet<_> = sub
            .served_tree_ids
            .iter()
            .map(|id| pin_key(id, &sub.removed_plant_id))
            .collect();
        garden
            .pinned_companions
            .retain(|pin| !removed.contains(pin));
    }
}

pub fn pin_key(tree: &str, plant: &str) -> String {
    format!("{tree}::{plant}")
}

pub fn selected_ids(tree: &Star, species: &Species) -> Vec<String> {
    if !tree.selected_plant_ids.is_empty() {
        tree.selected_plant_ids.clone()
    } else {
        species.recommended_companions.clone()
    }
}

fn swap(ids: &mut Vec<String>, removed: &str, added: &[String]) {
    let replacements: Vec<_> = added
        .iter()
        .filter(|id| !ids.contains(id))
        .cloned()
        .collect();
    if let Some(index) = ids.iter().position(|id| id == removed) {
        ids.splice(index..=index, replacements);
    } else {
        ids.extend(replacements);
    }
}

struct Run {
    companions: Vec<Companion>,
    conflicts: Vec<Conflict>,
    score: u32,
    companion_by_id: HashMap<String, usize>,
}

impl Run {
    fn get(&self, id: &str) -> Option<&Companion> {
        self.companion_by_id.get(id).map(|&i| &self.companions[i])
    }
}

struct Slot<'a> {
    key: String,
    plant: &'a Companion,
    served: Vec<String>,
    conflicts: Vec<&'a Conflict>,
    pinned: bool,
}

impl Slot<'_> {
    fn weight(&self) -> u32 {
        self.conflicts
            .iter()
            .map(|conflict| conflict.severity.weight())
            .sum()
    }

    fn apply(&self, current: &Selections, ids: &[String]) -> Selections {
        let mut next = current.clone();
        for tree in &self.served {
            let mut picks = next.get(tree).cloned().unwrap_or_default();
            swap(&mut picks, &self.plant.plant_id, ids);
            if !picks.is_empty() {
                next.insert(tree.clone(), picks);
            }
        }
        next
    }
}

struct Candidate {
    ids: Vec<String>,
    score: i32,
    key: String,
}

impl Candidate {
    fn new(ids: Vec<String>, score: i32) -> Self {
        Self {
            key: ids.join(","),
            ids,
            score,
        }
    }
}

struct Attempt {
    ids: Vec<String>,
    next: Selections,
    run: Run,
}

struct Tree<'a> {
    star: &'a Star,
    species: &'a Species,
    compartment: Option<String>,
}

impl Tree<'_> {
    fn point(&self) -> [f64; 2] {
        [self.star.x_m, self.star.y_m]
    }
}

struct Context<'a> {
    catalog: &'a Catalog,
    garden: &'a Garden,
    settings: &'a Settings,
    sources: &'a Map<String, Value>,
    compartments: Compartments<'a>,
    trees: Vec<Tree<'a>>,
    by_tree: HashMap<&'a str, usize>,
    plants: HashMap<&'a str, &'a Species>,
    pinned: HashSet<&'a str>,
    banned: HashMap<String, HashSet<String>>,
    rejected: HashMap<String, HashSet<String>>,
}

impl<'a> Context<'a> {
    fn new(
        catalog: &'a Catalog,
        garden: &'a Garden,
        settings: &'a Settings,
        sources: &'a Map<String, Value>,
    ) -> Self {
        let compartments = Compartments::new(&garden.infrastructure.raised_beds);
        let species: HashMap<_, _> = catalog.trees.iter().map(|p| (p.id.as_str(), p)).collect();
        let trees: Vec<_> = garden
            .star_plants
            .iter()
            .filter_map(|star| {
                let species = *species.get(star.tree_id.as_str())?;
                Some(Tree {
                    star,
                    species,
                    compartment: compartments.at([star.x_m, star.y_m]).map(str::to_owned),
                })
            })
            .collect();
        let by_tree = trees
            .iter()
            .enumerate()
            .map(|(i, tree)| (tree.star.instance_id.as_str(), i))
            .collect();
        Self {
            catalog,
            garden,
            settings,
            sources,
            compartments,
            trees,
            by_tree,
            plants: catalog.plants.iter().map(|p| (p.id.as_str(), p)).collect(),
            pinned: garden
                .pinned_companions
                .iter()
                .map(String::as_str)
                .collect(),
            banned: HashMap::new(),
            rejected: HashMap::new(),
        }
    }

    fn tree(&self, id: &str) -> Option<&Tree<'a>> {
        self.by_tree.get(id).map(|&i| &self.trees[i])
    }

    fn evaluate(&self, current: Option<&Selections>) -> Run {
        let companions = if let Some(ids) = current {
            garden_layout::place_with_ids(
                self.catalog,
                &self.garden.star_plants,
                &self.garden.infrastructure,
                self.sources,
                self.settings.southern,
                Some(ids),
            )
        } else {
            garden_layout::place(
                self.catalog,
                &self.garden.star_plants,
                &self.garden.infrastructure,
                self.sources,
                self.settings.southern,
            )
        };
        let conflicts = garden_conflicts::analyze(
            self.catalog,
            &self.garden.star_plants,
            &companions,
            &self.compartments,
        );
        let score = conflicts.iter().map(|c| c.severity.weight()).sum();
        let companion_by_id = if conflicts.is_empty() {
            HashMap::new()
        } else {
            companions
                .iter()
                .enumerate()
                .map(|(index, p)| (p.instance_id.clone(), index))
                .collect()
        };
        Run {
            companions,
            conflicts,
            score,
            companion_by_id,
        }
    }

    fn is_pinned(&self, plant: &str, trees: &[String]) -> bool {
        !self.garden.auto_resolve_enabled
            || (!self.pinned.is_empty()
                && trees
                    .iter()
                    .any(|id| self.pinned.contains(pin_key(id, plant).as_str())))
    }

    fn slots<'run>(&self, run: &'run Run) -> Vec<Slot<'run>> {
        let mut involved = HashMap::<&str, i32>::new();
        let mut fixable = Vec::new();
        for conflict in &run.conflicts {
            let a = run.get(&conflict.a.id);
            let b = run.get(&conflict.b.id);
            if a.is_none() && b.is_none() {
                continue;
            }
            for p in [a, b].into_iter().flatten() {
                *involved.entry(p.instance_id.as_str()).or_default() += 1;
            }
            fixable.push((conflict, a, b));
        }
        fixable.sort_by(|a, b| id_order(&a.0.id, &b.0.id));
        let mut slots = Vec::<Slot>::new();
        let mut indexes = HashMap::new();
        for (conflict, a, b) in fixable {
            let culprit = match (a, b) {
                (Some(a), Some(b)) => {
                    let a_pinned = self.is_pinned(&a.plant_id, &a.servicing_tree_ids);
                    let b_pinned = self.is_pinned(&b.plant_id, &b.servicing_tree_ids);
                    if a_pinned != b_pinned {
                        if a_pinned {
                            b
                        } else {
                            a
                        }
                    } else if conflict.kind.aggressor_first() {
                        a
                    } else {
                        let rank = |own: &Companion, other: &Companion| {
                            let plant = self.plants[own.plant_id.as_str()];
                            let foreign = other
                                .servicing_tree_ids
                                .iter()
                                .filter_map(|id| self.tree(id))
                                .any(|tree| {
                                    !self.catalog.compatible_with_star(plant, tree.species)
                                });
                            let recommended = own
                                .servicing_tree_ids
                                .iter()
                                .filter_map(|id| self.tree(id))
                                .any(|tree| {
                                    plant.recommended_for_trees.contains(&tree.species.id)
                                        || tree.species.recommended_companions.contains(&plant.id)
                                });
                            i32::from(foreign) * 100
                                + involved[own.instance_id.as_str()] * 10
                                + if recommended { 0 } else { 3 }
                        };
                        let score_a = rank(a, b);
                        let score_b = rank(b, a);
                        if score_a > score_b
                            || (score_a == score_b
                                && a.instance_id
                                    .encode_utf16()
                                    .cmp(b.instance_id.encode_utf16())
                                    .is_lt())
                        {
                            a
                        } else {
                            b
                        }
                    }
                }
                (Some(plant), None) | (None, Some(plant)) => plant,
                (None, None) => unreachable!(),
            };
            let mut served = culprit.servicing_tree_ids.clone();
            served.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            let key = format!("{}@{}", culprit.plant_id, served.join("+"));
            let index = *indexes.entry(key.clone()).or_insert_with(|| {
                let index = slots.len();
                slots.push(Slot {
                    pinned: self.is_pinned(&culprit.plant_id, &served),
                    key,
                    plant: culprit,
                    served,
                    conflicts: Vec::new(),
                });
                index
            });
            slots[index].conflicts.push(conflict);
        }
        slots.sort_by(|a, b| {
            b.weight()
                .cmp(&a.weight())
                .then_with(|| id_order(&a.key, &b.key))
        });
        slots
    }

    fn roles<'b>(&'b self, tree: &str, ids: &[String], exclude: Option<&str>) -> HashSet<&'b str> {
        let own = self
            .tree(tree)
            .into_iter()
            .flat_map(|item| item.species.dual_roles.iter());
        let companions = ids
            .iter()
            .filter(|id| Some(id.as_str()) != exclude)
            .filter_map(|id| self.plants.get(id.as_str()))
            .flat_map(|item| &item.role_ids);
        own.chain(companions).map(String::as_str).collect()
    }

    fn candidates(&self, slot: &Slot<'_>, current: &Selections, run: &Run) -> Vec<Candidate> {
        let removed = self.plants[slot.plant.plant_id.as_str()];
        let served: Vec<_> = slot.served.iter().filter_map(|id| self.tree(id)).collect();
        let picks = |tree: &str| current.get(tree).map_or(&[][..], Vec::as_slice);
        let point = [slot.plant.x_m, slot.plant.y_m];
        let mut required = HashSet::new();
        let mut mates = HashSet::new();
        for tree in &served {
            let id = tree.star.instance_id.as_str();
            let ids = picks(id);
            let roles = self.roles(id, ids, Some(&removed.id));
            required.extend(
                removed
                    .role_ids
                    .iter()
                    .filter(|r| !roles.contains(r.as_str())),
            );
            mates.extend(
                ids.iter()
                    .filter(|id| *id != &removed.id)
                    .filter_map(|id| self.plants.get(id.as_str()).copied())
                    .map(|p| p.id.as_str()),
            );
        }
        let mates: Vec<_> = mates.into_iter().map(|id| self.plants[id]).collect();
        let nearby: Vec<_> = run
            .companions
            .iter()
            .filter(|p| {
                p.instance_id != slot.plant.instance_id && distance([p.x_m, p.y_m], point) < 4.5
            })
            .filter_map(|p| self.plants.get(p.plant_id.as_str()).copied())
            .collect();
        let distances: Vec<_> = self
            .trees
            .iter()
            .map(|tree| {
                let dist = if slot.served.contains(&tree.star.instance_id) {
                    0.0
                } else {
                    served
                        .iter()
                        .fold(distance(tree.point(), point), |dist, item| {
                            let reach = item.species.mature_radius_m * 1.3 + 0.5;
                            dist.min((distance(tree.point(), item.point()) - reach).max(0.0))
                        })
                };
                let wall = !served.is_empty()
                    && served
                        .iter()
                        .all(|item| item.compartment != tree.compartment);
                (tree.species, dist, wall)
            })
            .collect();
        let replacements = self.rejected.get(&slot.key);
        let soil = self.settings.soil.as_ref();
        let climate = self.settings.climate.as_ref();
        let eligible = |plant: &&Species| {
            !(plant.retired
                || plant.id == removed.id
                || replacements.is_some_and(|reject| reject.contains(&plant.id))
                || self.catalog.trees.iter().any(|tree| tree.id == plant.id)
                || plant.fennel()
                || plant.wormwood()
                || climate.is_some_and(|zone| !plant.climate_zones.contains(zone))
                || soil.is_some_and(|soil| plant.unsuitable_soils.contains(soil))
                || served.iter().any(|tree| {
                    let id = &tree.star.instance_id;
                    (climate.is_none()
                        && !tree
                            .species
                            .climate_zones
                            .iter()
                            .any(|zone| plant.climate_zones.contains(zone)))
                        || !self.catalog.compatible_with_star(plant, tree.species)
                        || self
                            .banned
                            .get(id)
                            .is_some_and(|ids| ids.contains(&plant.id))
                        || picks(id).contains(&plant.id)
                })
                || distances.iter().any(|&(tree, dist, wall)| {
                    garden_conflicts::near_incompatible(self.catalog, plant, tree, dist, wall)
                })
                || mates.iter().chain(&nearby).any(|p| ph_pair(plant, p))
                || nearby
                    .iter()
                    .any(|p| (plant.allium() && p.legume()) || (plant.legume() && p.allium())))
        };
        let base = |plant: &Species| -> i32 {
            let match_roles = removed
                .role_ids
                .iter()
                .filter(|r| plant.role_ids.contains(r))
                .count() as i32;
            let extra_roles = plant
                .role_ids
                .iter()
                .filter(|r| !removed.role_ids.contains(r))
                .count() as i32;
            let recommended: i32 = served
                .iter()
                .map(|tree| {
                    i32::from(plant.recommended_for_trees.contains(&tree.species.id)) * 40
                        + i32::from(tree.species.recommended_companions.contains(&plant.id)) * 40
                })
                .sum();
            let caution = (plant.allium() && mates.iter().any(|p| p.legume()))
                || (plant.legume() && mates.iter().any(|p| p.allium()));
            match_roles * 50
                + extra_roles * 2
                + recommended
                + 20 * i32::from(plant.layer == removed.layer)
                + 10 * i32::from(plant.zone == removed.zone)
                + 15 * i32::from(garden_layout::collar(plant) == garden_layout::collar(removed))
                + 10 * i32::from(soil.is_some_and(|soil| plant.suitable_soils.contains(soil)))
                - 25 * i32::from(caution)
        };
        let mut pool: Vec<_> = self
            .catalog
            .plants
            .iter()
            .filter(eligible)
            .map(|plant| (plant, base(plant)))
            .collect();
        pool.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| id_order(&a.0.id, &b.0.id)));
        let matches = |plant: &Species| {
            required
                .iter()
                .filter(|&&role| plant.role_ids.contains(role))
                .count()
        };
        let mut ranked: Vec<_> = pool
            .iter()
            .map(|&(plant, score)| {
                let count = matches(plant);
                Candidate::new(
                    vec![plant.id.clone()],
                    2 * (score
                        + if count == required.len() {
                            1000
                        } else {
                            count as i32 * 100
                        }),
                )
            })
            .collect();
        if !required.is_empty() && !pool.iter().any(|(p, _)| matches(p) == required.len()) {
            for &(first, score) in pool.iter().take(8) {
                let missing: Vec<_> = required
                    .iter()
                    .filter(|&&role| !first.role_ids.contains(role))
                    .copied()
                    .collect();
                if missing.is_empty() || missing.len() == required.len() {
                    continue;
                }
                if let Some(&(second, second_score)) = pool.iter().find(|(plant, _)| {
                    plant.id != first.id
                        && missing.iter().all(|&role| plant.role_ids.contains(role))
                }) {
                    ranked.push(Candidate::new(
                        vec![first.id.clone(), second.id.clone()],
                        1990 + score + second_score,
                    ));
                }
            }
        }
        ranked.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| id_order(&a.key, &b.key)));
        if served
            .iter()
            .all(|tree| picks(&tree.star.instance_id).len() > 1)
        {
            ranked.push(Candidate::new(Vec::new(), i32::MIN));
        }
        ranked
    }

    fn locked_score(&self, run: &Run) -> u32 {
        run.conflicts
            .iter()
            .filter(|c| {
                [c.a.id.as_str(), c.b.id.as_str()]
                    .into_iter()
                    .filter_map(|id| run.get(id))
                    .all(|p| self.is_pinned(&p.plant_id, &p.servicing_tree_ids))
            })
            .map(|c| c.severity.weight())
            .sum()
    }

    fn candidate_clear(after: &Run, slot: &Slot<'_>, added: &[String]) -> bool {
        let changed = |id: &str| {
            after.get(id).is_some_and(|p| {
                added.contains(&p.plant_id)
                    && p.servicing_tree_ids
                        .iter()
                        .any(|id| slot.served.contains(id))
            })
        };
        after
            .conflicts
            .iter()
            .all(|c| !changed(&c.a.id) && !changed(&c.b.id))
    }

    fn try_slot(
        &mut self,
        slot: &Slot<'_>,
        current: &Selections,
        run: &Run,
        max_tries: usize,
    ) -> Option<Attempt> {
        let mut fallback: Option<Attempt> = None;
        let mut candidates = self.candidates(slot, current, run);
        if candidates.len() > max_tries {
            let removal = candidates
                .last()
                .is_some_and(|candidate| candidate.ids.is_empty())
                .then(|| candidates.pop())
                .flatten();
            candidates.truncate(max_tries);
            candidates.extend(removal);
        }
        let locked = self.locked_score(run);
        for candidate in candidates {
            let next = slot.apply(current, &candidate.ids);
            let next_run = self.evaluate(Some(&next));
            let clear = Self::candidate_clear(&next_run, slot, &candidate.ids);
            if clear && next_run.score < run.score {
                return Some(Attempt {
                    ids: candidate.ids,
                    next,
                    run: next_run,
                });
            }
            self.rejected
                .entry(slot.key.clone())
                .or_default()
                .extend(candidate.ids.iter().cloned());
            if clear
                && fallback
                    .as_ref()
                    .is_none_or(|result| next_run.score < result.run.score)
                && self.locked_score(&next_run) <= locked
            {
                fallback = Some(Attempt {
                    ids: candidate.ids,
                    next,
                    run: next_run,
                });
            }
        }
        fallback
    }

    fn describe(
        &self,
        slot: &Slot<'_>,
        current: &Selections,
        next: &Selections,
        ids: &[String],
    ) -> Substitution {
        let removed = self.plants[slot.plant.plant_id.as_str()];
        let mut lost = HashSet::new();
        for tree in &slot.served {
            let before = self.roles(tree, current.get(tree).map_or(&[], Vec::as_slice), None);
            let after = self.roles(tree, next.get(tree).map_or(&[], Vec::as_slice), None);
            lost.extend(
                removed.role_ids.iter().filter(|role| {
                    before.contains(role.as_str()) && !after.contains(role.as_str())
                }),
            );
        }
        Substitution {
            id: slot.key.clone(),
            removed_plant_id: removed.id.clone(),
            added_plant_ids: ids.to_vec(),
            served_tree_ids: slot.served.clone(),
            instance_id: slot.plant.instance_id.clone(),
            conflict: slot.conflicts[0].clone(),
            conflict_ids: slot.conflicts.iter().map(|c| c.id.clone()).collect(),
            roles_preserved: removed
                .role_ids
                .iter()
                .filter(|role| !lost.contains(role))
                .cloned()
                .collect(),
            roles_lost: removed
                .role_ids
                .iter()
                .filter(|role| lost.contains(role))
                .cloned()
                .collect(),
        }
    }

    fn ban(&mut self, slot: &Slot<'_>) {
        for tree in &slot.served {
            self.banned
                .entry(tree.clone())
                .or_default()
                .insert(slot.plant.plant_id.clone());
        }
    }

    fn stats(&self, current: &Selections, run: &Run) -> Stats {
        let companion_plant_count = run.companions.len();
        let unoptimized_companion_count = self
            .trees
            .iter()
            .map(|tree| {
                current
                    .get(&tree.star.instance_id)
                    .into_iter()
                    .flatten()
                    .collect::<HashSet<_>>()
                    .len()
            })
            .sum::<usize>();
        let plants_saved = unoptimized_companion_count.saturating_sub(companion_plant_count);
        let savings_percent = if unoptimized_companion_count == 0 {
            0
        } else {
            (plants_saved as f64 / unoptimized_companion_count as f64 * 100.0).round() as usize
        };
        let mut roles: HashSet<_> = run
            .companions
            .iter()
            .filter_map(|companion| self.plants.get(companion.plant_id.as_str()))
            .flat_map(|plant| &plant.role_ids)
            .collect();
        for tree in &self.trees {
            roles.extend(&tree.species.dual_roles);
            let name = tree.species.botanical_name.trim().to_lowercase();
            if let Some(twin) = self
                .catalog
                .plants
                .iter()
                .find(|plant| plant.botanical_name.trim().to_lowercase() == name)
            {
                roles.extend(&twin.role_ids);
            }
        }
        Stats {
            star_plant_count: self.trees.len(),
            companion_plant_count,
            unoptimized_companion_count,
            plants_saved,
            savings_percent,
            covered_roles: roles.len(),
            critical_conflicts_count: run
                .conflicts
                .iter()
                .filter(|conflict| conflict.severity == garden_conflicts::Severity::Critical)
                .count(),
            warning_conflicts_count: run
                .conflicts
                .iter()
                .filter(|conflict| conflict.severity == garden_conflicts::Severity::Warning)
                .count(),
        }
    }
}

fn id_order(a: &str, b: &str) -> std::cmp::Ordering {
    // Catalogue and generated instance IDs are ASCII. Sort by the same
    // punctuation order as the previous planner's default locale.
    let weight = |unit: u8| match unit {
        b'_' => 1,
        b'-' => 2,
        b',' => 3,
        b':' => 4,
        b'@' => 5,
        b'+' => 6,
        _ => unit,
    };
    a.bytes()
        .map(|unit| weight(unit.to_ascii_lowercase()))
        .cmp(b.bytes().map(|unit| weight(unit.to_ascii_lowercase())))
        .then_with(|| b.cmp(a))
}

pub fn resolve(
    catalog: &Catalog,
    garden: &Garden,
    settings: &Settings,
    sources: &Map<String, Value>,
) -> Resolution {
    let mut context = Context::new(catalog, garden, settings, sources);
    let mut current: Selections = context
        .trees
        .iter()
        .map(|tree| {
            let mut list = selected_ids(tree.star, tree.species);
            if list.is_empty() {
                list.extend(DEFAULTS.map(str::to_owned));
            }
            (tree.star.instance_id.clone(), list)
        })
        .collect();
    let mut run = context.evaluate(None);
    let mut substitutions = Vec::new();
    let mut exhausted = HashSet::new();
    for _ in 0..MAX_SWAPS * 4 {
        if substitutions.len() >= MAX_SWAPS {
            break;
        }
        let slots = context.slots(&run);
        let open: Vec<_> = slots
            .into_iter()
            .filter(|slot| !slot.pinned && !exhausted.contains(&slot.key))
            .collect();
        if open.is_empty() {
            break;
        }
        if open.len() > 1 {
            let mut batch = current.clone();
            let mut picks = Vec::new();
            for slot in &open {
                if let Some(best) = context.candidates(slot, &batch, &run).into_iter().next() {
                    batch = slot.apply(&batch, &best.ids);
                    picks.push((slot, best.ids));
                }
            }
            if picks.len() > 1 {
                let after = context.evaluate(Some(&batch));
                if after.score < run.score
                    && picks
                        .iter()
                        .all(|(slot, ids)| Context::candidate_clear(&after, slot, ids))
                {
                    for (slot, ids) in picks {
                        substitutions.push(context.describe(slot, &current, &batch, &ids));
                        context.ban(slot);
                    }
                    current = batch;
                    run = after;
                    continue;
                }
            }
        }
        let slot = &open[0];
        if let Some(Attempt {
            ids,
            next,
            run: after,
        }) = context.try_slot(slot, &current, &run, 6)
        {
            substitutions.push(context.describe(slot, &current, &next, &ids));
            context.ban(slot);
            current = next;
            run = after;
        } else {
            exhausted.insert(slot.key.clone());
        }
    }
    let mut suggestions = Vec::new();
    for slot in context.slots(&run).into_iter().filter(|slot| {
        slot.pinned || exhausted.contains(&slot.key) || substitutions.len() >= MAX_SWAPS
    }) {
        if let Some(result) = context.try_slot(&slot, &current, &run, 4) {
            suggestions.push(context.describe(&slot, &current, &result.next, &result.ids));
        }
    }
    let stats = context.stats(&current, &run);
    Resolution {
        companions: run.companions,
        stats,
        effective_companion_ids: current,
        substitutions,
        suggestions,
        unresolved: run.conflicts,
    }
}
