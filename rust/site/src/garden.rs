use crate::catalog::Species;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

pub const STORAGE_KEY: &str = "permaculture_garden_grid_v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase"
)]
pub enum Shape {
    Rect {
        x_m: f64,
        y_m: f64,
        w_m: f64,
        h_m: f64,
    },
    Circle {
        cx_m: f64,
        cy_m: f64,
        r_m: f64,
    },
    Polygon {
        points: Vec<[f64; 2]>,
    },
}

impl Shape {
    pub fn contains(&self, [x, y]: [f64; 2]) -> bool {
        match self {
            Self::Rect { x_m, y_m, w_m, h_m } => {
                x >= *x_m && x <= x_m + w_m && y >= *y_m && y <= y_m + h_m
            }
            Self::Circle { cx_m, cy_m, r_m } => (x - cx_m).hypot(y - cy_m) <= *r_m,
            Self::Polygon { points } => garden_core::contour::contains([x, y], points),
        }
    }

    pub fn area(&self) -> f64 {
        match self {
            Self::Rect { w_m, h_m, .. } => (w_m * h_m).abs(),
            Self::Circle { r_m, .. } => std::f64::consts::PI * r_m * r_m,
            Self::Polygon { points } => garden_core::contour::signed_area(points).abs(),
        }
    }

    pub fn points(&self) -> Vec<[f64; 2]> {
        match self {
            Self::Rect {
                x_m: x,
                y_m: y,
                w_m: w,
                h_m: h,
            } => vec![[*x, *y], [x + w, *y], [x + w, y + h], [*x, y + h]],
            Self::Circle {
                cx_m: x,
                cy_m: y,
                r_m: r,
            } => vec![[x - r, y - r], [x + r, y + r]],
            Self::Polygon { points } => points.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bed {
    pub id: String,
    pub shape: Shape,
    pub height_m: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Infrastructure {
    pub outline: Option<Shape>,
    pub raised_beds: Vec<Bed>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Star {
    pub instance_id: String,
    pub tree_id: String,
    pub x_m: f64,
    pub y_m: f64,
    #[serde(default)]
    pub selected_plant_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

impl Star {
    pub fn new(
        tree: &Species,
        instance_id: String,
        x_m: f64,
        y_m: f64,
        selected_plant_ids: Vec<String>,
    ) -> Self {
        Self {
            instance_id,
            tree_id: tree.id.clone(),
            x_m,
            y_m,
            selected_plant_ids,
            custom_name: None,
            other: Map::new(),
        }
    }

    pub fn with_source(&mut self, sources: &Map<String, Value>) {
        if let Some(source) = sources.get(&self.tree_id) {
            self.other
                .entry("starTree")
                .or_insert_with(|| source.clone());
        }
    }
}

fn default_resolution() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuildTemplate {
    pub id: String,
    pub source_name: String,
    pub star_tree: Value,
    pub selected_plant_ids: Vec<String>,
    pub imported_at: String,
}

impl GuildTemplate {
    pub fn tree_id(&self) -> Option<&str> {
        self.star_tree.get("id").and_then(Value::as_str)
    }
}

fn read_templates<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<GuildTemplate>, D::Error> {
    // A malformed older template should leave the rest of the saved garden readable.
    let list = Value::deserialize(deserializer)?;
    let Value::Array(entries) = list else {
        return Ok(Vec::new());
    };
    Ok(entries
        .into_iter()
        .filter_map(|entry| serde_json::from_value(entry).ok())
        .collect())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Garden {
    pub garden_id: String,
    pub garden_name: String,
    pub star_plants: Vec<Star>,
    pub infrastructure: Infrastructure,
    pub auto_shade_enabled: bool,
    #[serde(default = "default_resolution")]
    pub auto_resolve_enabled: bool,
    pub pinned_companions: Vec<String>,
    #[serde(
        default,
        deserialize_with = "read_templates",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub loaded_guild_templates: Vec<GuildTemplate>,
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

impl Default for Garden {
    fn default() -> Self {
        Self {
            garden_id: String::new(),
            garden_name: String::new(),
            star_plants: Vec::new(),
            infrastructure: Infrastructure::default(),
            auto_shade_enabled: false,
            auto_resolve_enabled: true,
            pinned_companions: Vec::new(),
            loaded_guild_templates: Vec::new(),
            other: Map::new(),
        }
    }
}

impl Garden {
    pub fn saved(raw: Option<&str>) -> Self {
        let mut garden: Self = raw
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default();
        if garden.garden_name.trim().is_empty() {
            garden.garden_name = garden
                .other
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into();
        }
        garden
    }

    pub fn remove(&mut self, instance: &str) {
        self.star_plants.retain(|tree| tree.instance_id != instance);
        let prefix = format!("{instance}::");
        self.pinned_companions
            .retain(|pin| !pin.starts_with(&prefix));
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub soil: Option<String>,
    pub climate: Option<String>,
    pub southern: bool,
    pub english: bool,
    pub established: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_legacy_templates_alongside_the_saved_garden() {
        let template = json!({
            "id": "tmpl-example",
            "sourceName": "Saved guild",
            "starTree": {"id": "tree-apple", "commonName": {"en": "Apple"}},
            "selectedPlantIds": ["plant-yarrow"],
            "importedAt": "2026-01-01T00:00:00.000Z"
        });
        let raw = json!({
            "gardenId": "old-garden",
            "gardenName": "Garden",
            "loadedGuildTemplates": [null, template]
        });
        let garden = Garden::saved(Some(&raw.to_string()));
        assert_eq!(garden.garden_id, "old-garden");
        assert_eq!(garden.garden_name, "Garden");
        assert_eq!(garden.loaded_guild_templates.len(), 1);
        assert_eq!(
            garden.loaded_guild_templates[0].tree_id(),
            Some("tree-apple")
        );
        assert_eq!(
            serde_json::to_value(garden).unwrap()["loadedGuildTemplates"][0],
            template
        );
    }
}
