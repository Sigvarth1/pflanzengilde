use crate::{
    catalog::{Catalog, Species},
    garden::{Garden, Shape},
    garden_layout::Companion,
    ground_cover::Cover,
};
use std::{
    collections::{HashMap, HashSet},
    f64::consts::{PI, TAU},
};

pub use garden_core::contour::{signed_area, Point};

pub enum Geometry {
    Ring(Vec<Point>),
    Circle { center: Point, radius: f64 },
}

impl Geometry {
    pub fn polygon(&self, count: usize) -> Vec<Point> {
        match self {
            Self::Ring(points) => points.clone(),
            Self::Circle {
                center: [x, y],
                radius,
            } => (0..count)
                .map(|i| {
                    let angle = i as f64 / count as f64 * TAU;
                    [
                        rounded(x + radius * angle.cos(), 1000.0),
                        rounded(y + radius * angle.sin(), 1000.0),
                    ]
                })
                .collect(),
        }
    }

    pub fn center(&self) -> Point {
        match self {
            Self::Circle { center, .. } => *center,
            Self::Ring(points) => {
                let [x, y] = points
                    .iter()
                    .fold([0.0, 0.0], |[x, y], p| [x + p[0], y + p[1]]);
                [
                    x / points.len().max(1) as f64,
                    y / points.len().max(1) as f64,
                ]
            }
        }
    }
}

pub struct Instance<'a> {
    pub id: &'a str,
    pub species: &'a Species,
    pub code: String,
    pub reference: String,
    pub label: &'a str,
    pub point: Point,
    pub width: f64,
    pub light: &'static str,
    pub shared: bool,
    pub pest_defense: bool,
    pub served: Vec<String>,
}

pub struct Bed {
    pub name: String,
    pub shape: Geometry,
    pub height: f64,
    pub area: f64,
}

pub struct GroundArea<'a> {
    pub species: &'a Species,
    pub code: String,
    pub rings: Vec<Vec<Point>>,
    pub polygons: Vec<Vec<usize>>,
    pub dots: Vec<Point>,
    pub opacity: f64,
    pub area: f64,
}

fn polygons(rings: &[Vec<Point>]) -> Vec<Vec<usize>> {
    let inside = |inner: usize, outer: usize| {
        let [x, y] = rings[inner][0];
        let [a, b] = rings[inner][1];
        garden_core::contour::contains([(x + a) / 2.0, (y + b) / 2.0], &rings[outer])
    };
    let depth: Vec<_> = (0..rings.len())
        .map(|i| (0..rings.len()).filter(|&j| i != j && inside(i, j)).count())
        .collect();
    let areas: Vec<_> = rings.iter().map(|r| signed_area(r).abs()).collect();
    let mut polygons: Vec<_> = (0..rings.len())
        .filter(|&i| depth[i] % 2 == 0)
        .map(|i| vec![i])
        .collect();
    for i in (0..rings.len()).filter(|&i| depth[i] % 2 != 0) {
        if let Some(parent) = polygons
            .iter_mut()
            .filter(|polygon| depth[polygon[0]] + 1 == depth[i] && inside(i, polygon[0]))
            .min_by(|a, b| areas[a[0]].total_cmp(&areas[b[0]]))
        {
            parent.push(i);
        }
    }
    polygons
}

pub struct Drawing<'a> {
    pub name: &'a str,
    pub width: f64,
    pub height: f64,
    origin: Point,
    pub trees: Vec<Instance<'a>>,
    pub plants: Vec<Instance<'a>>,
    pub ground: Vec<GroundArea<'a>>,
    pub outline: Option<(Geometry, f64)>,
    pub beds: Vec<Bed>,
    pub dimensions: Vec<(Point, Point, f64)>,
}

pub fn rounded(value: f64, scale: f64) -> f64 {
    (value * scale + 0.5).floor() / scale
}

fn convert(shape: &Shape, project: &impl Fn(Point) -> Point) -> (Geometry, f64) {
    match shape {
        Shape::Circle { cx_m, cy_m, r_m } => (
            Geometry::Circle {
                center: project([*cx_m, *cy_m]),
                radius: rounded(*r_m, 1000.0),
            },
            rounded(PI * r_m * r_m, 100.0),
        ),
        _ => {
            let original = shape.points();
            let area = rounded(signed_area(&original).abs(), 100.0);
            (
                Geometry::Ring(original.into_iter().map(project).collect()),
                area,
            )
        }
    }
}

struct Codes<'a> {
    prefix: &'static str,
    names: HashMap<&'a str, (usize, usize)>,
}

impl<'a> Codes<'a> {
    fn next(&mut self, species: &'a str) -> (String, String) {
        let next = self.names.len() + 1;
        let (code, count) = self.names.entry(species).or_insert((next, 0));
        *count += 1;
        (
            format!("{}{code}", self.prefix),
            format!("{}{code}-{count}", self.prefix),
        )
    }
}

impl<'a> Drawing<'a> {
    pub fn new(
        catalog: &'a Catalog,
        garden: &'a Garden,
        placed: &'a [Companion],
        covers: &[Cover],
        language: &str,
    ) -> Self {
        let tree_by_id: HashMap<_, _> = catalog
            .trees
            .iter()
            .map(|tree| (tree.id.as_str(), tree))
            .collect();
        let plant_by_id: HashMap<_, _> = catalog
            .plants
            .iter()
            .map(|plant| (plant.id.as_str(), plant))
            .collect();
        let trees: Vec<_> = garden
            .star_plants
            .iter()
            .filter_map(|tree| {
                tree_by_id
                    .get(tree.tree_id.as_str())
                    .map(|&species| (tree, species))
            })
            .collect();
        let plants: Vec<_> = placed
            .iter()
            .filter_map(|plant| {
                plant_by_id
                    .get(plant.plant_id.as_str())
                    .map(|&species| (plant, species))
            })
            .collect();

        let mut extent = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        let mut add = |x: f64, y: f64, r: f64| {
            extent[0] = extent[0].min(x - r);
            extent[1] = extent[1].min(y - r);
            extent[2] = extent[2].max(x + r);
            extent[3] = extent[3].max(y + r);
        };
        for (tree, species) in &trees {
            add(tree.x_m, tree.y_m, species.mature_radius_m);
        }
        for (plant, species) in &plants {
            add(plant.x_m, plant.y_m, species.spread_m / 2.0);
        }
        for [x, y] in covers.iter().flat_map(|cover| cover.rings.iter().flatten()) {
            add(*x, *y, 0.0);
        }
        for shape in garden.infrastructure.outline.iter().chain(
            garden
                .infrastructure
                .raised_beds
                .iter()
                .map(|bed| &bed.shape),
        ) {
            for [x, y] in shape.points() {
                add(x, y, 0.0);
            }
        }
        if !extent[0].is_finite() {
            extent = [-5.0, -5.0, 5.0, 5.0];
        }
        let start = extent[0].floor() - 1.0;
        let south = extent[3].ceil() + 1.0;
        let project = |[x, y]: Point| [rounded(x - start, 1000.0), rounded(south - y, 1000.0)];

        let mut codes = Codes {
            prefix: if language == "de" { "B" } else { "T" },
            names: HashMap::new(),
        };
        let trees: Vec<_> = trees
            .into_iter()
            .map(|(tree, species)| {
                let (code, reference) = codes.next(&species.id);
                Instance {
                    id: &tree.instance_id,
                    species,
                    code,
                    reference,
                    label: tree.custom_name.as_deref().unwrap_or(&species.name),
                    point: project([tree.x_m, tree.y_m]),
                    width: rounded(species.mature_radius_m * 2.0, 1000.0),
                    light: "",
                    shared: false,
                    pest_defense: false,
                    served: Vec::new(),
                }
            })
            .collect();
        let refs: HashMap<_, _> = trees
            .iter()
            .map(|tree| (tree.id, &tree.reference))
            .collect();
        let mut codes = Codes {
            prefix: "P",
            names: HashMap::new(),
        };
        let covered: HashSet<_> = covers.iter().map(|cover| cover.plant_id.as_str()).collect();
        let plants = plants
            .into_iter()
            .filter(|(_, species)| !covered.contains(species.id.as_str()))
            .map(|(plant, species)| {
                let (code, reference) = codes.next(&species.id);
                Instance {
                    id: &plant.instance_id,
                    species,
                    code,
                    reference,
                    label: &species.name,
                    point: project([plant.x_m, plant.y_m]),
                    width: rounded(species.spread_m, 1000.0),
                    light: plant.current_light_condition,
                    shared: plant.is_merged,
                    pest_defense: plant.is_key_pest_defense,
                    served: plant
                        .servicing_tree_ids
                        .iter()
                        .filter_map(|id| refs.get(id.as_str()).map(|value| (*value).clone()))
                        .collect(),
                }
            })
            .collect();
        let ground = covers
            .iter()
            .filter_map(|cover| {
                let species = *plant_by_id.get(cover.plant_id.as_str())?;
                Some((cover, species))
            })
            .enumerate()
            .map(|(index, (cover, species))| {
                let rings: Vec<Vec<Point>> = cover
                    .rings
                    .iter()
                    .map(|ring| ring.iter().copied().map(project).collect())
                    .collect();
                let polygons = polygons(&rings);
                let area = polygons
                    .iter()
                    .map(|polygon| {
                        let area = |ring: usize| signed_area(&rings[ring]).abs();
                        area(polygon[0]) - polygon.iter().skip(1).copied().map(area).sum::<f64>()
                    })
                    .sum();
                GroundArea {
                    species,
                    code: format!("G{}", index + 1),
                    rings,
                    polygons,
                    dots: cover.dots.iter().copied().map(project).collect(),
                    opacity: cover.opacity,
                    area: rounded(area, 100.0),
                }
            })
            .collect();

        let beds = garden
            .infrastructure
            .raised_beds
            .iter()
            .enumerate()
            .map(|(i, bed)| {
                let (shape, area) = convert(&bed.shape, &project);
                let caption = match language {
                    "de" => "Hochbeet",
                    "nb" => "Høybed",
                    _ => "Raised bed",
                };
                Bed {
                    name: bed
                        .name
                        .clone()
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| format!("{caption} {}", i + 1)),
                    shape,
                    height: bed.height_m,
                    area,
                }
            })
            .collect();
        let outline = garden
            .infrastructure
            .outline
            .as_ref()
            .map(|shape| convert(shape, &project));
        let mut seen = HashSet::new();
        let mut dimensions = Vec::new();
        for (i, a) in trees.iter().enumerate() {
            let mut nearest: Vec<_> = trees
                .iter()
                .enumerate()
                .filter(|(j, _)| i != *j)
                .map(|(j, b)| (j, (b.point[0] - a.point[0]).hypot(b.point[1] - a.point[1])))
                .collect();
            nearest.sort_by(|a, b| a.1.total_cmp(&b.1));
            for (j, distance) in nearest.into_iter().take(2) {
                if seen.insert((i.min(j), i.max(j))) {
                    dimensions.push((a.point, trees[j].point, rounded(distance, 1000.0)));
                }
            }
        }
        Self {
            name: &garden.garden_name,
            width: extent[2].ceil() + 1.0 - start,
            height: south - extent[1].floor() + 1.0,
            origin: [start, south],
            trees,
            plants,
            ground,
            outline,
            beds,
            dimensions,
        }
    }

    pub fn empty(&self) -> bool {
        self.trees.is_empty()
            && self.plants.is_empty()
            && self.ground.is_empty()
            && self.outline.is_none()
            && self.beds.is_empty()
    }

    pub fn project(&self, [x, y]: Point) -> Point {
        [
            rounded(x - self.origin[0], 1000.0),
            rounded(self.origin[1] - y, 1000.0),
        ]
    }
}

static UI_LANGUAGE: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// Remembers the page language so number formatting deep in views can follow it.
pub fn set_ui_language(language: &'static str) {
    let _ = UI_LANGUAGE.set(language);
}

/// Formats a number with the decimal separator of the page language.
pub fn num(value: f64, decimals: usize) -> String {
    number(value, decimals, UI_LANGUAGE.get().copied().unwrap_or("nb"))
}

pub fn number(value: f64, decimals: usize, lang: &str) -> String {
    let out = format!("{value:.decimals$}");
    if lang == "en" {
        out
    } else {
        out.replace('.', ",")
    }
}

pub fn safe_color(color: &str) -> &str {
    if let Some(hex) = color.strip_prefix('#') {
        if matches!(hex.len(), 3 | 6) && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return color;
        }
    }
    "#486745"
}
