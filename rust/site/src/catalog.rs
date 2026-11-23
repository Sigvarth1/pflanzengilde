use crate::calendar::Activity;
use crate::placement::{Sector, Zone};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Species {
    pub id: String,
    pub name: String,
    pub names: [String; 3],
    pub botanical_name: String,
    pub description: String,
    pub image_url: String,
    #[serde(default)]
    pub photo_credit: Option<String>,
    pub climate_zones: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub unsuitable_soils: Vec<String>,
    #[serde(default)]
    pub suitable_soils: Vec<String>,
    #[serde(default)]
    pub preferred_soils: Vec<String>,
    #[serde(default)]
    pub category_label: String,
    #[serde(default)]
    pub bloom_label: String,
    #[serde(default)]
    pub dual_roles: Vec<String>,
    #[serde(default)]
    pub retired: bool,
    #[serde(default)]
    pub area_plant: bool,
    pub color: String,
    #[serde(default)]
    pub mature_radius_m: f64,
    #[serde(default)]
    pub height_m: Option<f64>,
    #[serde(default)]
    pub layer_label: String,
    #[serde(default)]
    pub spread_m: f64,
    #[serde(default)]
    pub zone: Zone,
    #[serde(default)]
    pub sector: Sector,
    #[serde(default)]
    pub sector_label: String,
    #[serde(default)]
    pub layer: String,
    #[serde(default)]
    pub nitrogen_fixer: bool,
    #[serde(default)]
    pub juglone_producer: bool,
    #[serde(default)]
    pub juglone_sensitive: bool,
    #[serde(default)]
    pub juglone_label: String,
    #[serde(default)]
    pub role_ids: Vec<String>,
    #[serde(default)]
    pub min_distance_m: f64,
    #[serde(default)]
    pub max_distance_m: f64,
    #[serde(default)]
    pub sun_preference: String,
    #[serde(default)]
    pub sun_label: String,
    #[serde(default)]
    pub activity: Activity,
    #[serde(default)]
    pub recommended_companions: Vec<String>,
    #[serde(default)]
    pub recommended_for_trees: Vec<String>,
    #[serde(default)]
    pub pest_defense_for: Vec<String>,
    #[serde(default)]
    pub vulnerabilities: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PestConflict {
    pub id: String,
    pub tree_ids: Vec<String>,
    pub host_ids: Vec<String>,
    pub host_star_ids: Vec<String>,
    pub safe_distance_m: f64,
    pub severity: crate::garden_conflicts::Severity,
    #[serde(default)]
    pub bed_wall: crate::garden_conflicts::PestWall,
    pub title: String,
    pub mechanism: String,
    pub citations: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub trees: Vec<Species>,
    pub plants: Vec<Species>,
    pub default_plant_ids: Vec<String>,
    #[serde(default)]
    pub presets: HashMap<String, Preset>,
    pub pest_conflicts: Vec<PestConflict>,
    #[serde(default)]
    pub guild_hosts: Vec<crate::guild_antagonisms::HostRule>,
    #[serde(default)]
    pub pest_defense_rules: Vec<crate::pest_defense::Rule>,
    pub labels: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub tree_id: String,
    pub plant_ids: Vec<String>,
}

impl Catalog {
    pub fn text(&self, key: &str) -> String {
        self.labels.get(key).cloned().unwrap_or_else(|| key.into())
    }
}
