use crate::catalog::Catalog;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use web_sys::UrlSearchParams;

pub const SOILS: [(&str, &str); 6] = [
    ("LOAM", "soilLoam"),
    ("CLAY", "soilClay"),
    ("SANDY", "soilSandy"),
    ("CHALKY", "soilChalky"),
    ("ACIDIC", "soilAcidic"),
    ("SILT", "soilSilt"),
];
pub const CLIMATES: [(&str, &str); 4] = [
    ("BOREAL", "zoneBoreal"),
    ("TEMPERATE", "zoneTemperate"),
    ("SUBTROPICAL", "zoneSubtropical"),
    ("TROPICAL", "zoneTropical"),
];

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Guild {
    pub tree_id: String,
    pub plant_ids: Vec<String>,
    pub soil_type: String,
    pub climate_zone: String,
    pub hemisphere: String,
    pub tree_age: String,
}

impl Guild {
    pub fn from_defaults(catalog: &Catalog) -> Self {
        Self {
            tree_id: catalog
                .trees
                .first()
                .map(|tree| tree.id.clone())
                .unwrap_or_default(),
            plant_ids: catalog.default_plant_ids.clone(),
            soil_type: "LOAM".into(),
            climate_zone: "TEMPERATE".into(),
            hemisphere: "NORTHERN".into(),
            tree_age: "YOUNG".into(),
        }
    }

    pub fn recover(
        catalog: &Catalog,
        mut guild: Self,
        saved: Option<&str>,
        params: &UrlSearchParams,
        route_tree: Option<&str>,
    ) -> Self {
        if let Some(code) = params
            .get("g")
            .and_then(|code| Self::decode(&code, catalog, guild.clone()))
        {
            return code;
        }
        let ids = params
            .get("plants")
            .map(|list| list.split(',').map(str::to_string).collect())
            .unwrap_or_else(|| {
                params
                    .get_all("plant")
                    .iter()
                    .filter_map(|value| value.as_string())
                    .collect::<Vec<_>>()
            });
        let tree_id = params
            .get("tree")
            .filter(|value| !value.is_empty())
            .or_else(|| route_tree.map(str::to_owned))
            .unwrap_or_default();
        if !tree_id.is_empty() || !ids.is_empty() {
            if catalog.trees.iter().any(|tree| tree.id == tree_id) {
                guild.tree_id = tree_id;
            }
            let mut ids: Vec<_> = ids.iter().map(|id| id.trim()).collect();
            ids.sort_unstable();
            let selected: Vec<_> = catalog
                .plants
                .iter()
                .filter(|plant| !plant.retired && ids.binary_search(&plant.id.as_str()).is_ok())
                .map(|plant| plant.id.clone())
                .collect();
            if !selected.is_empty() {
                guild.plant_ids = selected;
            }
            guild.update_location(
                params.get("soil").as_deref(),
                params.get("zone").as_deref(),
                params
                    .get("hemi")
                    .or_else(|| params.get("hemisphere"))
                    .as_deref(),
            );
            return guild;
        }
        if let Some(saved) = saved.and_then(|raw| serde_json::from_str::<Self>(raw).ok()) {
            if catalog.trees.iter().any(|tree| tree.id == saved.tree_id) {
                guild.tree_id = saved.tree_id;
            }
            // Keep old locally saved species even if the catalogue no longer offers them.
            guild.plant_ids = catalog
                .plants
                .iter()
                .filter(|plant| saved.plant_ids.contains(&plant.id))
                .map(|plant| plant.id.clone())
                .collect();
            guild.update_location(
                Some(&saved.soil_type),
                Some(&saved.climate_zone),
                Some(&saved.hemisphere),
            );
            if saved.tree_age == "ESTABLISHED" {
                guild.tree_age = saved.tree_age;
            }
        }
        guild
    }

    pub(crate) fn update_location(
        &mut self,
        soil: Option<&str>,
        climate: Option<&str>,
        hemisphere: Option<&str>,
    ) {
        if let Some(soil) = soil
            .map(str::to_ascii_uppercase)
            .filter(|v| soil_index(v) < SOILS.len())
        {
            self.soil_type = soil;
        }
        if let Some(climate) = climate
            .map(str::to_ascii_uppercase)
            .filter(|v| climate_index(v) < CLIMATES.len())
        {
            self.climate_zone = climate;
        }
        if let Some(hemisphere) = hemisphere
            .map(str::to_ascii_uppercase)
            .filter(|value| matches!(value.as_str(), "NORTHERN" | "SOUTHERN"))
        {
            self.hemisphere = hemisphere;
        }
    }

    fn decode(code: &str, catalog: &Catalog, mut guild: Self) -> Option<Self> {
        let bytes = URL_SAFE_NO_PAD
            .decode(code.trim().trim_end_matches('='))
            .ok()?;
        let (&first, rest) = bytes.split_first()?;
        let (&second, mut mask) = rest.split_first()?;
        let tree_field = usize::from(first & 31);
        let tree_index = match (second >> 5, tree_field) {
            (1, 31) => None,
            (1, 30) => {
                let (extra, remaining) = read_varint(mask)?;
                mask = remaining;
                30usize.checked_add(extra)
            }
            _ => Some(tree_field),
        };
        let tree = tree_index.and_then(|index| catalog.trees.get(index));
        let plants = decode_plant_mask(catalog, mask);
        if tree.is_none() && plants.is_empty() {
            return None;
        }
        if let Some(tree) = tree {
            guild.tree_id.clone_from(&tree.id);
        }
        if !plants.is_empty() {
            guild.plant_ids = plants;
        }
        guild.update_location(
            SOILS.get(usize::from(first >> 5)).map(|(value, _)| *value),
            CLIMATES
                .get(usize::from(second & 7))
                .map(|(value, _)| *value),
            Some(if second & 8 != 0 {
                "SOUTHERN"
            } else {
                "NORTHERN"
            }),
        );
        Some(guild)
    }

    pub fn encode(&self, catalog: &Catalog, language: &str) -> String {
        let tree = catalog
            .trees
            .iter()
            .position(|tree| tree.id == self.tree_id);
        let field = tree.map(|index| index.min(30)).unwrap_or(31) as u8;
        let mut bytes = vec![
            field | ((soil_index(&self.soil_type) as u8) << 5),
            32 | climate_index(&self.climate_zone) as u8
                | if self.hemisphere == "SOUTHERN" { 8 } else { 0 }
                | if language == "en" { 16 } else { 0 },
        ];
        if let Some(index) = tree.filter(|index| *index >= 30) {
            push_varint(&mut bytes, index - 30);
        }
        bytes.extend(encode_plant_mask(catalog, &self.plant_ids));
        URL_SAFE_NO_PAD.encode(bytes)
    }
}

pub(crate) fn soil_index(value: &str) -> usize {
    SOILS
        .iter()
        .position(|(soil, _)| *soil == value)
        .unwrap_or(7)
}

pub(crate) fn climate_index(value: &str) -> usize {
    CLIMATES
        .iter()
        .position(|(zone, _)| *zone == value)
        .unwrap_or(7)
}

pub(crate) fn read_varint(bytes: &[u8]) -> Option<(usize, &[u8])> {
    let mut value = 0usize;
    for (i, &byte) in bytes.iter().take(5).enumerate() {
        let part = usize::from(byte & 127);
        let shift = 7 * i;
        if part > usize::MAX >> shift {
            return None;
        }
        value |= part << shift;
        if byte & 128 == 0 {
            return Some((value, &bytes[i + 1..]));
        }
    }
    None
}

pub(crate) fn push_varint(bytes: &mut Vec<u8>, mut value: usize) {
    while value >= 128 {
        bytes.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    bytes.push(value as u8);
}

pub(crate) fn decode_plant_mask(catalog: &Catalog, mask: &[u8]) -> Vec<String> {
    catalog
        .plants
        .iter()
        .enumerate()
        .filter(|(i, plant)| {
            !plant.retired
                && mask
                    .get(i / 8)
                    .is_some_and(|byte| byte & (1 << (i % 8)) != 0)
        })
        .map(|(_, plant)| plant.id.clone())
        .collect()
}

pub(crate) fn encode_plant_mask(catalog: &Catalog, ids: &[String]) -> Vec<u8> {
    let mut mask = Vec::new();
    for (i, plant) in catalog.plants.iter().enumerate() {
        if ids.contains(&plant.id) {
            let byte = i / 8;
            if mask.len() <= byte {
                mask.resize(byte + 1, 0);
            }
            mask[byte] |= 1 << (i % 8);
        }
    }
    mask
}
