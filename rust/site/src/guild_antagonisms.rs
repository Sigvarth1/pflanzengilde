use crate::{
    catalog::{Catalog, Species},
    compatibility::{acid_intolerant, acid_tree, acidophile, calcicole},
    garden_conflicts::{juglone_sensitive_tree, Severity, JUGLONE_DISTANCE},
    placement::{polar_distance, Placement},
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostRule {
    id: String,
    tree_ids: Vec<String>,
    host_ids: Vec<String>,
    external: bool,
    severity: Severity,
    distance: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Scope {
    Guild,
    Landscape,
    Resolved,
}

#[derive(Clone, PartialEq)]
pub struct Affected {
    pub name: String,
    pub distance: f64,
    pub angle: Option<u16>,
}

impl Affected {
    fn tree(species: &Species) -> Self {
        Self {
            name: species.name.clone(),
            distance: 0.0,
            angle: None,
        }
    }

    fn plant(species: &Species, position: Option<&Placement<'_>>) -> Self {
        Self {
            name: species.name.clone(),
            distance: position.map_or(2.0, |position| position.distance),
            angle: Some(position.map_or(0, |position| position.angle)),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Notice {
    pub id: String,
    pub scope: Scope,
    pub severity: Severity,
    pub limit: f64,
    pub measured: Option<f64>,
    pub affected: Vec<Affected>,
}

pub fn analyze(
    catalog: &Catalog,
    tree: &Species,
    plants: &[&Species],
    positions: &[Placement<'_>],
) -> Vec<Notice> {
    let mut notices = Vec::new();
    let mut record = |id: &str, scope, severity, limit, measured, affected| {
        notices.push(Notice {
            id: id.into(),
            scope,
            severity,
            limit,
            measured,
            affected,
        });
    };
    let at = |position: &Placement| Affected::plant(position.species, Some(position));
    let at_star = || vec![Affected::tree(tree)];
    let select = |predicate: fn(&Species) -> bool| {
        positions
            .iter()
            .filter(|p| predicate(p.species))
            .collect::<Vec<_>>()
    };
    let sensitive: Vec<_> = plants
        .iter()
        .filter(|p| p.juglone_sensitive)
        .map(|p| {
            Affected::plant(
                p,
                positions
                    .iter()
                    .find(|position| position.species.id == p.id),
            )
        })
        .collect();
    if tree.juglone_producer {
        if !sensitive.is_empty() {
            let furthest = sensitive
                .iter()
                .map(|plant| plant.distance)
                .max_by(f64::total_cmp);
            record(
                "internal-walnut-sensitive-companions",
                Scope::Guild,
                Severity::Critical,
                JUGLONE_DISTANCE,
                furthest,
                sensitive,
            );
        }
    } else if !sensitive.is_empty() || juglone_sensitive_tree(tree) {
        let affected: Vec<_> = juglone_sensitive_tree(tree)
            .then(|| Affected::tree(tree))
            .into_iter()
            .chain(sensitive)
            .collect();
        let extent = affected
            .iter()
            .fold(tree.mature_radius_m, |max, item| max.max(item.distance));
        record(
            "external-walnut-spatial-planning",
            Scope::Landscape,
            Severity::Critical,
            JUGLONE_DISTANCE,
            Some(extent + JUGLONE_DISTANCE),
            affected,
        );
    }

    let alliums = select(Species::allium);
    let legumes = select(Species::legume);
    let mut closest = None;
    for a in &alliums {
        for b in &legumes {
            let distance = polar_distance(a, b.distance, b.angle);
            if closest.is_none_or(|(best, _, _)| distance < best) {
                closest = Some((distance, *a, *b));
            }
        }
    }
    if let Some((distance, a, b)) = closest {
        let (id, scope, severity) = if distance < 1.8 {
            (
                "internal-allium-legume-proximity",
                Scope::Guild,
                Severity::Warning,
            )
        } else {
            (
                "resolved-allium-legume-spacing",
                Scope::Resolved,
                Severity::Info,
            )
        };
        record(id, scope, severity, 1.8, Some(distance), vec![at(a), at(b)]);
    }

    if tree.id == "tree-pear" || tree.botanical_name.to_ascii_lowercase().contains("pyrus") {
        record(
            "external-pear-juniper-rust",
            Scope::Landscape,
            Severity::Warning,
            300.0,
            None,
            at_star(),
        );
    }
    fn ribes(species: &Species) -> bool {
        species.id.contains("currant")
            || species
                .botanical_name
                .to_ascii_lowercase()
                .starts_with("ribes")
    }
    if ribes(tree) || plants.iter().any(|species| ribes(species)) {
        record(
            "external-ribes-white-pine-rust",
            Scope::Landscape,
            Severity::Info,
            300.0,
            None,
            ribes(tree)
                .then(|| Affected::tree(tree))
                .into_iter()
                .chain(select(ribes).into_iter().map(at))
                .collect(),
        );
    }
    if matches!(
        tree.activity.tree_category.as_str(),
        "FRUIT_TREE" | "BERRY_SHRUB"
    ) {
        let limit = ((tree.mature_radius_m + 2.0) * 10.0).round() / 10.0;
        record(
            "external-solanaceae-fruit-tree-verticillium",
            Scope::Landscape,
            Severity::Warning,
            limit,
            None,
            at_star(),
        );
    }
    if tree.activity.tree_category == "NITROGEN_FIXING_TREE" {
        if let Some(near) = alliums
            .iter()
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
        {
            let (id, scope, severity, affected) = if near.distance < 1.8 {
                (
                    "internal-allium-nfixing-tree-proximity",
                    Scope::Guild,
                    Severity::Warning,
                    at_star()
                        .into_iter()
                        .chain(alliums.iter().filter(|p| p.distance < 1.8).map(|p| at(p)))
                        .collect(),
                )
            } else {
                (
                    "resolved-allium-nfixing-tree-spacing",
                    Scope::Resolved,
                    Severity::Info,
                    vec![Affected::tree(tree), at(near)],
                )
            };
            record(id, scope, severity, 1.8, Some(near.distance), affected);
        }
    }
    for (source, limit, warning, harmony) in [
        (
            select(Species::fennel),
            1.5,
            "internal-fennel-allelopathy",
            "resolved-fennel-allelopathy-spacing",
        ),
        (
            select(Species::wormwood),
            1.2,
            "internal-wormwood-allelopathy",
            "resolved-wormwood-allelopathy-spacing",
        ),
    ] {
        let Some(&source) = source.first() else {
            continue;
        };
        let distance = |peer: &Placement| polar_distance(source, peer.distance, peer.angle);
        let peers = positions
            .iter()
            .filter(|p| p.species.id != source.species.id);
        let nearby: Vec<_> = peers.clone().filter(|p| distance(p) < limit).collect();
        if nearby.is_empty() {
            let closest = peers.min_by(|a, b| distance(a).total_cmp(&distance(b)));
            record(
                harmony,
                Scope::Resolved,
                Severity::Info,
                limit,
                closest.map(distance),
                std::iter::once(at(source))
                    .chain(closest.into_iter().map(at))
                    .collect(),
            );
        } else {
            let shortest = nearby.iter().map(|p| distance(p)).min_by(f64::total_cmp);
            record(
                warning,
                Scope::Guild,
                Severity::Warning,
                limit,
                shortest,
                std::iter::once(at(source))
                    .chain(nearby.into_iter().map(at))
                    .collect(),
            );
        }
    }
    let acids = select(acidophile);
    let limes = select(calcicole);
    if (acid_tree(tree) || !acids.is_empty()) && (acid_intolerant(tree) || !limes.is_empty()) {
        record(
            "internal-edaphic-ph-antagonism",
            Scope::Guild,
            Severity::Warning,
            2.5,
            None,
            (acid_tree(tree) || acid_intolerant(tree))
                .then(|| Affected::tree(tree))
                .into_iter()
                .chain(acids.into_iter().chain(limes).map(at))
                .collect(),
        );
    }

    fn tea(species: &Species) -> bool {
        species.id.contains("tea")
            || species
                .botanical_name
                .to_ascii_lowercase()
                .contains("camellia")
    }
    if tea(tree) || plants.iter().any(|species| tea(species)) {
        record(
            "external-tea-calcium-incompatibility",
            Scope::Landscape,
            Severity::Warning,
            5.0,
            None,
            tea(tree)
                .then(|| Affected::tree(tree))
                .into_iter()
                .chain(select(tea).into_iter().map(at))
                .collect(),
        );
    }

    for rule in &catalog.guild_hosts {
        if rule.tree_ids.contains(&tree.id) {
            let hosts: Vec<_> = positions
                .iter()
                .filter(|p| rule.host_ids.contains(&p.species.id))
                .map(at)
                .collect();
            if !hosts.is_empty() {
                record(
                    &rule.id,
                    if rule.external {
                        Scope::Landscape
                    } else {
                        Scope::Guild
                    },
                    rule.severity,
                    rule.distance,
                    None,
                    at_star().into_iter().chain(hosts).collect(),
                );
            }
        }
    }
    notices
}
