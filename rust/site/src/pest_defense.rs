use crate::{
    catalog::{Catalog, Species},
    guild::Guild,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Evidence {
    FieldProven,
    Scientific,
}

impl Evidence {
    pub fn label(self, catalog: &Catalog) -> String {
        catalog.text(match self {
            Self::FieldProven => "evidenceLevelFieldProven",
            Self::Scientific => "evidenceLevelScientific",
        })
    }
}

#[derive(Deserialize)]
pub struct Citation {
    pub label: String,
    pub href: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub title: String,
    pub mechanism: String,
    keywords: Vec<String>,
    tree_ids: Option<Vec<String>>,
    evidence: Evidence,
    companion_ids: Vec<String>,
    #[serde(default)]
    companion_evidence: HashMap<String, Evidence>,
    companion_roles: HashMap<String, String>,
    pub citations: Vec<Citation>,
}

impl Rule {
    fn applies(&self, tree: &Species) -> bool {
        self.tree_ids
            .as_ref()
            .is_none_or(|trees| trees.contains(&tree.id))
    }

    pub fn companion_evidence(&self, id: &str) -> Evidence {
        self.companion_evidence
            .get(id)
            .copied()
            .unwrap_or(self.evidence)
    }

    pub fn companion_role(&self, id: &str) -> String {
        self.companion_roles
            .get(id)
            .unwrap_or(&self.mechanism)
            .clone()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Defense {
    pub rule: usize,
    pub evidence: Evidence,
    pub plants: Vec<usize>,
}

pub fn resolve(catalog: &Catalog, tree: &Species, pest: &str, zone: &str) -> Option<Defense> {
    let pest = pest.trim().to_lowercase();
    catalog
        .pest_defense_rules
        .iter()
        .enumerate()
        .find_map(|(index, rule)| {
            if !rule.applies(tree) || !rule.keywords.iter().any(|keyword| pest.contains(keyword)) {
                return None;
            }
            let plants: Vec<_> = rule
                .companion_ids
                .iter()
                .filter_map(|id| {
                    catalog.plants.iter().position(|plant| {
                        id == &plant.id
                            && plant.climate_zones.iter().any(|value| value == zone)
                            && catalog.compatible_with_star(plant, tree)
                    })
                })
                .collect();
            if plants.is_empty() {
                return None;
            }
            let evidence = if plants
                .iter()
                .any(|&i| rule.companion_evidence(&catalog.plants[i].id) == Evidence::FieldProven)
            {
                Evidence::FieldProven
            } else {
                Evidence::Scientific
            };
            Some(Defense {
                rule: index,
                evidence,
                plants,
            })
        })
}

pub fn available(catalog: &Catalog, guild: &Guild, defense: &Defense) -> Vec<usize> {
    defense
        .plants
        .iter()
        .copied()
        .filter(|&i| {
            let plant = &catalog.plants[i];
            !plant.retired
                && (guild.plant_ids.contains(&plant.id)
                    || catalog.guild_incompatibilities(plant, guild).is_empty())
        })
        .collect()
}
