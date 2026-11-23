use crate::{
    catalog::{Catalog, Species},
    guild::Guild,
};
use serde_json::{json, Value};

pub enum ImportedPlan {
    Guild(Guild),
    Garden,
}

fn resolve<'a>(
    species: &'a [Species],
    reference: &Value,
    fallback_id: Option<&str>,
    partial_name: bool,
) -> Option<&'a Species> {
    let id = reference["id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .or(fallback_id);
    if let Some(plant) = id.and_then(|id| species.iter().find(|plant| plant.id == id)) {
        return Some(plant);
    }
    if let Some(botanical) = reference["botanicalName"].as_str() {
        let name = botanical.trim().to_lowercase();
        if let Some(plant) = species
            .iter()
            .find(|plant| !name.is_empty() && plant.botanical_name.trim().to_lowercase() == name)
        {
            return Some(plant);
        }
    }
    let names: Vec<_> = reference["commonName"]
        .as_str()
        .into_iter()
        .chain(
            reference["commonName"]
                .as_object()
                .into_iter()
                .flat_map(|names| names.values().filter_map(Value::as_str)),
        )
        .map(|name| name.trim().to_lowercase())
        .filter(|name| !name.is_empty())
        .collect();
    if names.is_empty() {
        return None;
    }
    species.iter().find(|plant| {
        plant.names.iter().any(|alias| {
            let alias = alias.to_lowercase();
            names.iter().any(|name| {
                if partial_name {
                    alias.contains(name)
                } else {
                    &alias == name
                }
            })
        })
    })
}

pub fn import(catalog: &Catalog, text: &str, current: &Guild) -> Option<ImportedPlan> {
    let data: Value = serde_json::from_str(text).ok()?;
    if data
        .get("starPlants")
        .and_then(Value::as_array)
        .is_some_and(|trees| !trees.is_empty())
    {
        return Some(ImportedPlan::Garden);
    }
    let tree = resolve(
        &catalog.trees,
        &data["starTree"],
        data["treeId"].as_str(),
        true,
    )?;
    let mut guild = current.clone();
    guild.tree_id.clone_from(&tree.id);
    guild.plant_ids.clear();
    for id in data["plantIds"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if catalog.plants.iter().any(|plant| plant.id == id)
            && !guild.plant_ids.iter().any(|plant| plant == id)
        {
            guild.plant_ids.push(id.to_owned());
        }
    }
    for reference in data["companionPlants"]
        .as_array()
        .or_else(|| data["plants"].as_array())
        .into_iter()
        .flatten()
    {
        if let Some(plant) = resolve(&catalog.plants, reference, None, false) {
            if !guild.plant_ids.contains(&plant.id) {
                guild.plant_ids.push(plant.id.clone());
            }
        }
    }
    guild.update_location(
        data["groundType"]
            .as_str()
            .or_else(|| data["soilType"].as_str()),
        data["climateZone"]
            .as_str()
            .or_else(|| data["zone"].as_str()),
        data["hemisphere"].as_str(),
    );
    Some(ImportedPlan::Guild(guild))
}

pub fn export(
    catalog: &Catalog,
    guild: &Guild,
    language: &str,
    created: &str,
) -> Option<(String, String)> {
    let tree = catalog.trees.iter().find(|tree| tree.id == guild.tree_id)?;
    let plants: Vec<_> = guild.plant_ids.iter()
        .filter_map(|id| catalog.plants.iter().find(|plant| &plant.id == id))
        .map(|p| json!({
            "id": p.id, "commonName": p.name, "botanicalName": p.botanical_name, "layer": p.layer,
            "roles": p.role_ids, "preferredZone": p.zone, "preferredSector": p.sector,
            "recommendedDistanceM": format!("{}–{} m", p.min_distance_m, p.max_distance_m),
        })).collect();
    let data = json!({
        "version": "1.0", "title": format!("{} Permaculture Plant Guild", tree.name),
        "language": language, "created": created, "hemisphere": guild.hemisphere,
        "groundType": guild.soil_type, "climateZone": guild.climate_zone,
        "starTree": {
            "id": tree.id, "commonName": tree.name, "botanicalName": tree.botanical_name,
            "matureDripLineM": tree.mature_radius_m, "sunPreference": tree.sun_preference,
        },
        "companionPlants": plants,
    });
    Some((
        format!("{}_guild_plan.json", tree.id),
        serde_json::to_string_pretty(&data).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn species(id: &str, name: &str) -> Value {
        json!({
            "id": id, "name": name, "names": [name, name, name],
            "botanicalName": "", "description": "", "imageUrl": "",
            "climateZones": [], "color": ""
        })
    }

    #[test]
    fn imports_localized_references_and_round_trips_the_plan() {
        let catalog: Catalog = serde_json::from_value(json!({
            "trees": [species("tree-apple", "Apple Tree")],
            "plants": [
                species("plant-yarrow", "Yarrow"),
                species("plant-chives", "Chives"),
                species("plant-white-clover", "White Clover")
            ],
            "defaultPlantIds": [],
            "pestConflicts": [],
            "labels": {}
        }))
        .unwrap();
        let defaults = Guild::from_defaults(&catalog);
        let raw = json!({
            "starTree": {"id": "tree-apple", "commonName": {"de": "Apfelbaum"}},
            "soilType": "LOAM",
            "hemisphere": "SOUTHERN",
            "plantIds": [null, "plant-yarrow"],
            "companionPlants": [
                {"id": "plant-chives", "commonName": null},
                {"commonName": {"en": "White Clover"}}
            ]
        });
        let Some(ImportedPlan::Guild(guild)) = import(&catalog, &raw.to_string(), &defaults) else {
            panic!("Legacy file should load as a guild");
        };
        assert_eq!(guild.tree_id, "tree-apple");
        assert_eq!(
            guild.plant_ids,
            ["plant-yarrow", "plant-chives", "plant-white-clover"]
        );
        assert_eq!(guild.hemisphere, "SOUTHERN");

        let created = "2026-11-23T12:00:00.000Z";
        let (filename, exported) = export(&catalog, &guild, "en", created).unwrap();
        assert_eq!(filename, "tree-apple_guild_plan.json");
        assert_eq!(
            serde_json::from_str::<Value>(&exported).unwrap()["created"],
            created
        );
        let Some(ImportedPlan::Guild(reloaded)) = import(&catalog, &exported, &defaults) else {
            panic!("Exported file should load as a guild");
        };
        assert_eq!(reloaded.tree_id, guild.tree_id);
        assert_eq!(reloaded.plant_ids, guild.plant_ids);
        assert_eq!(reloaded.hemisphere, guild.hemisphere);
    }
}
