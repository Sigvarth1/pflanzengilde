use crate::{
    catalog::{Catalog, Species},
    compatibility::{acid_intolerant, acid_tree, acidophile, calcicole},
    drawing::number,
    garden::{Bed, Star},
    garden_layout::Companion,
};
use serde::{Deserialize, Serialize};

pub(crate) const JUGLONE_DISTANCE: f64 = 18.0;
const SOIL_DISTANCE: f64 = 2.5;
const ALLIUM_DISTANCE: f64 = 1.8;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

impl Severity {
    pub fn weight(self) -> u32 {
        match self {
            Self::Critical => 12,
            Self::Warning => 4,
            Self::Info => 1,
        }
    }

    pub fn label(self, language: &str) -> &'static str {
        match (self, language) {
            (Self::Critical, "nb") => "Kritisk",
            (Self::Critical, "de") => "Kritisch",
            (Self::Critical, _) => "Critical",
            (Self::Warning, "nb") => "Advarsel",
            (Self::Warning, "de") => "Warnung",
            (Self::Warning, _) => "Warning",
            (Self::Info, _) => "Info",
        }
    }

    pub fn class(self) -> &'static str {
        match self {
            Self::Critical => "conflict-critical",
            Self::Warning => "conflict-warning",
            Self::Info => "conflict-info",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PestWall {
    #[default]
    NotBlocked,
    Blocked,
    Reduced,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Kind {
    Juglone,
    AlliumLegume,
    FennelAllelopathy,
    WormwoodAllelopathy,
    EdaphicPh,
    PestHost,
    TrunkCollision,
}

impl Kind {
    pub fn aggressor_first(self) -> bool {
        matches!(
            self,
            Self::AlliumLegume | Self::FennelAllelopathy | Self::WormwoodAllelopathy
        )
    }

    fn citation_key(self) -> Option<&'static str> {
        match self {
            Self::Juglone => Some("conflictSources:juglone"),
            Self::AlliumLegume => Some("conflictSources:allium"),
            Self::FennelAllelopathy => Some("conflictSources:fennel"),
            Self::WormwoodAllelopathy => Some("conflictSources:wormwood"),
            Self::EdaphicPh => Some("conflictSources:soil"),
            Self::PestHost | Self::TrunkCollision => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BedEffect {
    ReducedByBed,
    BedLeafLitter,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Party {
    pub id: String,
    pub species_id: String,
    pub point: [f64; 2],
    pub tree: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: Kind,
    pub severity: Severity,
    pub a: Party,
    pub b: Party,
    pub distance_m: f64,
    pub required_distance_m: f64,
    pub bed_effect: Option<BedEffect>,
    #[serde(skip)]
    pest: Option<usize>,
}

impl Conflict {
    pub fn guide(&self) -> Option<&'static str> {
        self.bed_effect.map(|_| {
            if self.kind == Kind::Juglone {
                "bed-juglone"
            } else {
                "bed-walls"
            }
        })
    }

    pub fn citations(&self, catalog: &Catalog) -> Vec<String> {
        if let Some(rule) = self.pest {
            return catalog.pest_conflicts[rule].citations.clone();
        }
        self.kind
            .citation_key()
            .and_then(|key| catalog.labels.get(key))
            .into_iter()
            .flat_map(|sources| sources.lines().map(str::to_owned))
            .collect()
    }

    pub fn text(&self, catalog: &Catalog, language: &str) -> (String, String) {
        let find = |party: &Party| {
            let species = if party.tree {
                &catalog.trees
            } else {
                &catalog.plants
            };
            species.iter().find(|item| item.id == party.species_id)
        };
        let a = find(&self.a).map_or(self.a.species_id.as_str(), |item| item.name.as_str());
        let plant_b = find(&self.b);
        let b = plant_b.map_or(self.b.species_id.as_str(), |item| item.name.as_str());
        let botanical = plant_b.map_or("", |item| item.botanical_name.as_str());
        let dist = number(self.distance_m, 1, language);
        let min = number(self.required_distance_m, 1, language);
        let format = |key, vars: &[(&str, &str)]| {
            vars.iter().fold(catalog.text(key), |text, &(var, value)| {
                text.replace(var, value)
            })
        };
        let (title, mut description) = match self.kind {
            Kind::Juglone => {
                let (title, desc) = if self.b.tree {
                    (
                        "gardenAntagonistJugloneTreeTitle",
                        "gardenAntagonistJugloneTreeDesc",
                    )
                } else {
                    (
                        "gardenAntagonistJugloneCompTitle",
                        "gardenAntagonistJugloneCompDesc",
                    )
                };
                let description = match self.bed_effect {
                    Some(BedEffect::BedLeafLitter) => {
                        format("gardenBedJugloneLitter", &[("{name}", b)])
                    }
                    Some(BedEffect::ReducedByBed) => {
                        format("gardenBedJugloneReduced", &[("{name}", b)])
                    }
                    None => format(
                        desc,
                        &[("{name}", b), ("{botanical}", botanical), ("{dist}", &dist)],
                    ),
                };
                (format(title, &[("{name}", b)]), description)
            }
            Kind::AlliumLegume => {
                let (title, description) = if self.b.tree {
                    (
                        "gardenAntagonistAlliumTreeTitle",
                        "gardenAntagonistAlliumTreeDesc",
                    )
                } else {
                    (
                        "gardenAntagonistAlliumLegumeTitle",
                        "gardenAntagonistAlliumLegumeDesc",
                    )
                };
                let vars = &[
                    ("{allium}", a),
                    ("{legume}", b),
                    ("{tree}", b),
                    ("{dist}", dist.as_str()),
                ];
                (format(title, vars), format(description, vars))
            }
            Kind::FennelAllelopathy => (
                format(
                    "gardenAntagonistFennelTitle",
                    &[("{fennel}", a), ("{other}", b)],
                ),
                format(
                    "gardenAntagonistFennelDesc",
                    &[("{other}", b), ("{dist}", &dist)],
                ),
            ),
            Kind::WormwoodAllelopathy => (
                format(
                    "gardenAntagonistWormwoodTitle",
                    &[("{wormwood}", a), ("{other}", b)],
                ),
                format(
                    "gardenAntagonistWormwoodDesc",
                    &[("{other}", b), ("{dist}", &dist)],
                ),
            ),
            Kind::EdaphicPh => {
                let vars = &[("{acid}", a), ("{calc}", b), ("{dist}", dist.as_str())];
                (
                    format("gardenAntagonistSoilPhTitle", vars),
                    format("gardenAntagonistSoilPhDesc", vars),
                )
            }
            Kind::PestHost => {
                let rule = &catalog.pest_conflicts[self.pest.expect("pest conflicts have a rule")];
                (
                    format!("{}: {b} ↔ {a}", rule.title),
                    format(
                        "gardenAntagonistPestHostDesc",
                        &[
                            ("{mechanism}", &rule.mechanism),
                            ("{dist}", &dist),
                            ("{safe}", &rule.safe_distance_m.to_string()),
                        ],
                    ),
                )
            }
            Kind::TrunkCollision => (
                format("gardenAntagonistTrunkTitle", &[("{a}", a), ("{b}", b)]),
                format(
                    "gardenAntagonistTrunkDesc",
                    &[("{dist}", &dist), ("{min}", &min)],
                ),
            ),
        };
        if self.bed_effect.is_some()
            && matches!(
                self.kind,
                Kind::FennelAllelopathy | Kind::WormwoodAllelopathy
            )
        {
            description.push(' ');
            description.push_str(&catalog.text("gardenBedReducedNote"));
        }
        (title, description)
    }
}

pub struct Compartments<'a>(Vec<(&'a Bed, f64)>);

impl<'a> Compartments<'a> {
    pub fn new(beds: &'a [Bed]) -> Self {
        let mut shapes: Vec<_> = beds.iter().map(|bed| (bed, bed.shape.area())).collect();
        shapes.sort_by(|a, b| a.1.total_cmp(&b.1));
        Self(shapes)
    }

    pub fn at(&self, point: [f64; 2]) -> Option<&str> {
        self.0
            .iter()
            .find(|(bed, _)| bed.shape.contains(point))
            .map(|(bed, _)| bed.id.as_str())
    }
}

#[derive(Clone, Copy)]
struct Entry<'a> {
    id: &'a str,
    species: &'a Species,
    point: [f64; 2],
    tree: bool,
    compartment: Option<&'a str>,
}

impl Entry<'_> {
    fn party(self) -> Party {
        Party {
            id: self.id.into(),
            species_id: self.species.id.clone(),
            point: self.point,
            tree: self.tree,
        }
    }
}

pub fn distance([x, y]: [f64; 2], [a, b]: [f64; 2]) -> f64 {
    (x - a).hypot(y - b)
}

fn rounded(value: f64, scale: f64) -> f64 {
    let scaled = value.abs() * scale;
    let floor = scaled.floor();
    let fraction = scaled - floor;
    let up = fraction > 0.5 || (fraction == 0.5 && value.abs().mul_add(scale, -scaled) >= 0.0);
    ((floor + f64::from(u8::from(up))) / scale).copysign(value)
}

pub(crate) fn juglone_sensitive_tree(tree: &Species) -> bool {
    !tree.juglone_producer
        && matches!(
            tree.id.as_str(),
            "tree-apple"
                | "tree-pear"
                | "tree-alder"
                | "tree-linden"
                | "shrub-blueberry"
                | "shrub-rhododendron"
                | "herb-rhubarb"
        )
}

pub fn near_incompatible(
    catalog: &Catalog,
    plant: &Species,
    tree: &Species,
    dist: f64,
    wall: bool,
) -> bool {
    if tree.juglone_producer
        && plant.juglone_sensitive
        && dist < JUGLONE_DISTANCE
        && (!wall || dist < tree.mature_radius_m)
    {
        return true;
    }
    let pest = || {
        catalog.pest_conflicts.iter().any(|rule| {
            (!wall || rule.bed_wall == PestWall::NotBlocked)
                && dist < rule.safe_distance_m
                && rule.tree_ids.contains(&tree.id)
                && rule.host_ids.contains(&plant.id)
        })
    };
    if wall {
        return pest();
    }
    (dist < SOIL_DISTANCE
        && ((acid_tree(tree) && calcicole(plant)) || (acid_intolerant(tree) && acidophile(plant))))
        || pest()
        || (dist < ALLIUM_DISTANCE
            && tree.activity.tree_category == "NITROGEN_FIXING_TREE"
            && plant.allium())
}

pub fn analyze(
    catalog: &Catalog,
    stars: &[Star],
    companions: &[Companion],
    compartments: &Compartments<'_>,
) -> Vec<Conflict> {
    let tree_lookup: std::collections::HashMap<_, _> =
        catalog.trees.iter().map(|p| (p.id.as_str(), p)).collect();
    let plant_lookup: std::collections::HashMap<_, _> =
        catalog.plants.iter().map(|p| (p.id.as_str(), p)).collect();
    let entry = |id, species, point, tree| Entry {
        id,
        species,
        point,
        tree,
        compartment: compartments.at(point),
    };
    let trees: Vec<_> = stars
        .iter()
        .filter_map(|tree| {
            Some(entry(
                tree.instance_id.as_str(),
                *tree_lookup.get(tree.tree_id.as_str())?,
                [tree.x_m, tree.y_m],
                true,
            ))
        })
        .collect();
    let plants: Vec<_> = companions
        .iter()
        .filter_map(|plant| {
            Some(entry(
                plant.instance_id.as_str(),
                *plant_lookup.get(plant.plant_id.as_str())?,
                [plant.x_m, plant.y_m],
                false,
            ))
        })
        .collect();

    let mut conflicts = Vec::new();
    let mut push = |a: Entry<'_>,
                    b: Entry<'_>,
                    prefix: &str,
                    kind,
                    mut severity,
                    required,
                    pest: Option<usize>| {
        let separation = distance(a.point, b.point);
        if separation >= required {
            return;
        }
        if kind == Kind::TrunkCollision && separation < 1.5 {
            severity = Severity::Critical;
        }
        let distance_m = rounded(separation, 10.0);
        let mut bed_effect = None;
        if a.compartment != b.compartment {
            match kind {
                Kind::Juglone => {
                    severity = Severity::Info;
                    bed_effect = Some(if distance_m < a.species.mature_radius_m {
                        BedEffect::BedLeafLitter
                    } else {
                        BedEffect::ReducedByBed
                    });
                }
                Kind::AlliumLegume | Kind::EdaphicPh => return,
                Kind::FennelAllelopathy | Kind::WormwoodAllelopathy
                    if distance_m >= a.species.spread_m / 2.0 =>
                {
                    severity = if severity == Severity::Critical {
                        Severity::Warning
                    } else {
                        Severity::Info
                    };
                    bed_effect = Some(BedEffect::ReducedByBed);
                }
                Kind::PestHost => match pest.map_or(PestWall::NotBlocked, |index| {
                    catalog.pest_conflicts[index].bed_wall
                }) {
                    PestWall::Blocked => return,
                    PestWall::Reduced => {
                        severity = Severity::Info;
                        bed_effect = Some(BedEffect::ReducedByBed);
                    }
                    PestWall::NotBlocked => {}
                },
                _ => {}
            }
        }
        conflicts.push(Conflict {
            id: format!("{prefix}-{}-{}", a.id, b.id),
            kind,
            severity,
            a: a.party(),
            b: b.party(),
            distance_m,
            required_distance_m: rounded(required, 10.0),
            bed_effect,
            pest,
        });
    };
    for &walnut in trees.iter().filter(|t| t.species.juglone_producer) {
        for &tree in trees
            .iter()
            .filter(|t| t.id != walnut.id && juglone_sensitive_tree(t.species))
        {
            push(
                walnut,
                tree,
                "juglone-tree",
                Kind::Juglone,
                Severity::Critical,
                JUGLONE_DISTANCE,
                None,
            );
        }
        for &plant in plants.iter().filter(|t| t.species.juglone_sensitive) {
            push(
                walnut,
                plant,
                "juglone-comp",
                Kind::Juglone,
                Severity::Critical,
                JUGLONE_DISTANCE,
                None,
            );
        }
    }
    for &allium in plants.iter().filter(|t| t.species.allium()) {
        for &legume in plants.iter().filter(|t| t.species.legume()) {
            push(
                allium,
                legume,
                "allium-legume",
                Kind::AlliumLegume,
                Severity::Warning,
                ALLIUM_DISTANCE,
                None,
            );
        }
        for &tree in trees
            .iter()
            .filter(|t| t.species.activity.tree_category == "NITROGEN_FIXING_TREE")
        {
            push(
                allium,
                tree,
                "allium-ntree",
                Kind::AlliumLegume,
                Severity::Warning,
                ALLIUM_DISTANCE,
                None,
            );
        }
    }
    for &fennel in plants.iter().filter(|p| p.species.fennel()) {
        for &other in plants
            .iter()
            .filter(|p| p.id != fennel.id && (!p.species.fennel() || fennel.id <= p.id))
        {
            push(
                fennel,
                other,
                "fennel-allelopathy",
                Kind::FennelAllelopathy,
                Severity::Warning,
                1.5,
                None,
            );
        }
    }
    for &wormwood in plants.iter().filter(|p| p.species.wormwood()) {
        for &other in plants.iter().filter(|p| {
            p.id != wormwood.id
                && !p.species.fennel()
                && (!p.species.wormwood() || wormwood.id <= p.id)
        }) {
            push(
                wormwood,
                other,
                "wormwood-allelopathy",
                Kind::WormwoodAllelopathy,
                Severity::Warning,
                1.2,
                None,
            );
        }
    }
    let acid = trees
        .iter()
        .filter(|p| acid_tree(p.species))
        .chain(plants.iter().filter(|p| acidophile(p.species)));
    let lime = trees
        .iter()
        .filter(|p| acid_intolerant(p.species))
        .chain(plants.iter().filter(|p| calcicole(p.species)));
    for &low in acid {
        for &high in lime.clone() {
            push(
                low,
                high,
                "edaphic-ph",
                Kind::EdaphicPh,
                Severity::Warning,
                SOIL_DISTANCE,
                None,
            );
        }
    }
    for (index, rule) in catalog.pest_conflicts.iter().enumerate() {
        let hosts = plants
            .iter()
            .filter(|p| rule.host_ids.contains(&p.species.id))
            .chain(
                trees
                    .iter()
                    .filter(|p| rule.host_star_ids.contains(&p.species.id)),
            );
        for &tree in trees
            .iter()
            .filter(|t| rule.tree_ids.contains(&t.species.id))
        {
            for &host in hosts.clone() {
                push(
                    tree,
                    host,
                    &rule.id,
                    Kind::PestHost,
                    rule.severity,
                    rule.safe_distance_m,
                    Some(index),
                );
            }
        }
    }
    for (index, &a) in trees.iter().enumerate() {
        for &b in trees.iter().skip(index + 1) {
            let min = (0.4 * (a.species.mature_radius_m + b.species.mature_radius_m)).max(1.5);
            push(
                a,
                b,
                "trunk-collision",
                Kind::TrunkCollision,
                Severity::Warning,
                min,
                None,
            );
        }
    }
    conflicts
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ShadePocket {
    pub point: [f64; 2],
    pub radius: f64,
    pub trees: [String; 2],
}

pub const SHADE_PLANTS: [&str; 4] = [
    "plant-comfrey",
    "plant-sweet-cicely",
    "plant-hosta",
    "plant-wild-garlic",
];

pub fn shade(catalog: &Catalog, stars: &[Star]) -> Vec<ShadePocket> {
    let radii: std::collections::HashMap<_, _> = catalog
        .trees
        .iter()
        .map(|tree| (tree.id.as_str(), tree.mature_radius_m))
        .collect();
    let known: Vec<_> = stars
        .iter()
        .filter_map(|star| Some((star, *radii.get(star.tree_id.as_str())?)))
        .collect();
    let mut pockets = Vec::new();
    for (index, &(a, r1)) in known.iter().enumerate() {
        for &(b, r2) in known.iter().skip(index + 1) {
            let separation = distance([a.x_m, a.y_m], [b.x_m, b.y_m]);
            let radius = r1 + r2;
            if separation < radius * 0.85 {
                pockets.push(ShadePocket {
                    point: [
                        rounded((a.x_m + b.x_m) / 2.0, 100.0),
                        rounded((a.y_m + b.y_m) / 2.0, 100.0),
                    ],
                    radius: rounded((radius - separation) / 2.0, 10.0).max(0.8),
                    trees: [a.instance_id.clone(), b.instance_id.clone()],
                });
            }
        }
    }
    pockets
}
