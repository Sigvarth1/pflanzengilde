use crate::{
    catalog::{Catalog, Species},
    garden::{Garden, Settings, Star},
    garden_conflicts,
    garden_resolution::{self, Resolution},
    guild::Guild,
    placement::Metrics,
};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    f64::consts::{PI, TAU},
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Pattern {
    #[default]
    #[serde(alias = "SINGLE")]
    Line,
    Grid,
    Triangle,
}

impl Pattern {
    pub fn key(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Grid => "grid",
            Self::Triangle => "triangle",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Line => "gardenPlanPatternLine",
            Self::Grid => "gardenPlanPatternGrid",
            Self::Triangle => "gardenPlanPatternTriangle",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub pattern: Pattern,
    pub count: usize,
    pub spacing_m: f64,
    pub orientation_deg: f64,
    pub grid_cols: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pattern: Pattern::Line,
            count: 1,
            spacing_m: 0.0,
            orientation_deg: 90.0,
            grid_cols: None,
        }
    }
}

pub struct Spacing {
    pub optimal: f64,
    pub input_min: f64,
    pub input_max: f64,
}

fn fixed(value: f64, digits: i32) -> f64 {
    let scale = 10.0_f64.powi(digits);
    let magnitude = value.abs();
    let scaled = magnitude * scale;
    let floor = scaled.floor();
    let fraction = scaled - floor;
    let up = fraction > 0.5 || (fraction == 0.5 && magnitude.mul_add(scale, -scaled) >= 0.0);
    ((floor + if up { 1.0 } else { 0.0 }) / scale).copysign(value)
}

pub fn spacing(star: &Species) -> Spacing {
    let radius = star.mature_radius_m;
    let trunk = (radius * 2.0 * 0.4).max(1.5);
    let optimal = trunk.max(1.2).max(fixed(radius * 2.0 * 0.95, 1));
    Spacing {
        optimal,
        input_min: trunk.max(fixed(radius * 1.2, 1)),
        input_max: 10.0_f64.max(radius * 3.5).max(optimal),
    }
}

pub fn points(tree: &Species, config: Config) -> Vec<[f64; 2]> {
    let count = config.count.clamp(1, 6);
    if count == 1 {
        return vec![[0.0; 2]];
    }
    let spacing = if config.spacing_m.is_finite() && config.spacing_m > 0.0 {
        config.spacing_m
    } else {
        spacing(tree).optimal
    };
    let orientation = (config.orientation_deg - 90.0) * PI / 180.0;
    let cols = config
        .grid_cols
        .filter(|&n| n > 0)
        .unwrap_or_else(|| (count as f64).sqrt().ceil() as usize);
    let centered = |index: usize, size: usize| index as f64 - (size - 1) as f64 / 2.0;
    (0..count)
        .map(|index| {
            let p = match config.pattern {
                Pattern::Line => {
                    let offset = centered(index, count) * spacing;
                    [offset * orientation.cos(), offset * orientation.sin()]
                }
                Pattern::Triangle if count == 3 => {
                    let angle = orientation + index as f64 * TAU / 3.0;
                    let radius = spacing / 3.0_f64.sqrt();
                    [radius * angle.cos(), radius * angle.sin()]
                }
                Pattern::Triangle | Pattern::Grid => {
                    let stagger = config.pattern == Pattern::Triangle;
                    let cols = if stagger {
                        (count as f64).sqrt().ceil() as usize
                    } else {
                        cols
                    };
                    let row = index / cols;
                    let rows = count.div_ceil(cols);
                    let x = centered(index % cols, cols) * spacing
                        + if stagger && row % 2 != 0 {
                            spacing * 0.5
                        } else {
                            0.0
                        };
                    let row_height = if stagger {
                        spacing * (3.0_f64.sqrt() / 2.0)
                    } else {
                        spacing
                    };
                    [x, centered(row, rows) * row_height]
                }
            };
            p.map(|value| fixed(value, 2))
        })
        .collect()
}

pub fn stars(tree: &Species, config: Config, ids: &[String], stamp: &str) -> Vec<Star> {
    points(tree, config)
        .into_iter()
        .enumerate()
        .map(|(i, [x, y])| {
            Star::new(
                tree,
                format!("star-tree-{}-{stamp}-{i}", tree.id),
                x,
                y,
                ids.to_vec(),
            )
        })
        .collect()
}

pub fn settings(guild: &Guild, language: &str) -> Settings {
    Settings {
        soil: Some(guild.soil_type.clone()),
        climate: Some(guild.climate_zone.clone()),
        southern: guild.hemisphere == "SOUTHERN",
        english: language == "en",
        established: guild.tree_age == "ESTABLISHED",
    }
}

#[derive(Clone, PartialEq)]
pub struct Preview {
    pub stars: Vec<Star>,
    pub resolution: Resolution,
    pub shades: Vec<garden_conflicts::ShadePocket>,
    pub bounds: [[f64; 2]; 2],
}

pub fn preview(
    catalog: &Catalog,
    tree: &Species,
    config: Config,
    guild: &Guild,
    auto_resolve: bool,
    language: &str,
) -> Preview {
    let garden = Garden {
        star_plants: stars(tree, config, &guild.plant_ids, "radial"),
        auto_resolve_enabled: auto_resolve,
        ..Garden::default()
    };
    let resolution =
        garden_resolution::resolve(catalog, &garden, &settings(guild, language), &Map::new());
    let shades = garden_conflicts::shade(catalog, &garden.star_plants);
    let mut bounds = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
    let mut add = |point: [f64; 2], reach: f64| {
        for (axis, value) in point.into_iter().enumerate() {
            bounds[0][axis] = bounds[0][axis].min(value - reach);
            bounds[1][axis] = bounds[1][axis].max(value + reach);
        }
    };
    let reach = Metrics::new(tree.mature_radius_m).rings[3].max(tree.mature_radius_m);
    for star in &garden.star_plants {
        add([star.x_m, star.y_m], reach);
    }
    for companion in &resolution.companions {
        let radius = catalog
            .plants
            .iter()
            .find(|p| p.id == companion.plant_id)
            .map_or(0.3, |p| 0.3_f64.max(p.spread_m / 2.0));
        add([companion.x_m, companion.y_m], radius);
    }
    Preview {
        stars: garden.star_plants,
        resolution,
        shades,
        bounds: bounds.map(|point| point.map(|v| fixed(v, 2))),
    }
}

fn separation(catalog: &Catalog, a: &Species, b: &Species) -> f64 {
    let mut required = 1.5_f64
        .max((a.mature_radius_m + b.mature_radius_m) * 0.4)
        .max(spacing(a).optimal)
        .max(spacing(b).optimal);
    if (a.juglone_producer && garden_conflicts::juglone_sensitive_tree(b))
        || (b.juglone_producer && garden_conflicts::juglone_sensitive_tree(a))
    {
        required = required.max(garden_conflicts::JUGLONE_DISTANCE);
    }
    for pest in &catalog.pest_conflicts {
        if pest.tree_ids.contains(&a.id) && pest.host_star_ids.contains(&b.id)
            || pest.tree_ids.contains(&b.id) && pest.host_star_ids.contains(&a.id)
        {
            required = required.max(pest.safe_distance_m);
        }
    }
    required
}

#[derive(Clone, Copy)]
struct Offset {
    i: i32,
    j: i32,
    squared: i32,
    angle: f64,
}

thread_local! {
    static LATTICE: RefCell<(i32, Vec<Offset>)> = const { RefCell::new((0, Vec::new())) };
}

fn search(step: f64, rings: i32, fits: &impl Fn(f64, f64) -> bool) -> Option<[f64; 2]> {
    LATTICE.with_borrow_mut(|(radius, offsets)| {
        if *radius < rings {
            *radius = rings;
            offsets.clear();
            for i in -rings..=rings {
                for j in -rings..=rings {
                    let squared = i * i + j * j;
                    if squared == 0 || squared > rings * rings {
                        continue;
                    }
                    let angle =
                        f64::from(j).atan2(f64::from(i)).abs() + if j < 0 { 1e-6 } else { 0.0 };
                    offsets.push(Offset {
                        i,
                        j,
                        squared,
                        angle,
                    });
                }
            }
            offsets.sort_unstable_by(|a, b| {
                a.squared
                    .cmp(&b.squared)
                    .then_with(|| a.angle.total_cmp(&b.angle))
            });
        }
        let mut best: Option<Offset> = None;
        for &candidate in offsets
            .iter()
            .take_while(|point| point.squared <= rings * rings)
        {
            let dx = f64::from(candidate.i) * step;
            let dy = f64::from(candidate.j) * step;
            if let Some(first) = best {
                if first.i == 0
                    || first.j == 0
                    || f64::from(candidate.squared).sqrt() * step
                        > f64::from(first.squared).sqrt() * step * 1.1 + 1.0
                {
                    break;
                }
                if (candidate.i == 0 || candidate.j == 0) && fits(dx, dy) {
                    best = Some(candidate);
                    break;
                }
            } else if fits(dx, dy) {
                best = Some(candidate);
            }
        }
        best.map(|offset| {
            [
                fixed(f64::from(offset.i) * step, 2),
                fixed(f64::from(offset.j) * step, 2),
            ]
        })
    })
}

pub fn place(
    catalog: &Catalog,
    existing: &[Star],
    incoming: &mut [Star],
    step: f64,
    clearance: f64,
) -> [f64; 2] {
    // Reserve all new IDs before moving the cluster, including repeat transfers.
    let mut taken: HashSet<String> = existing
        .iter()
        .map(|tree| tree.instance_id.clone())
        .collect();
    for tree in incoming.iter_mut() {
        if !taken.insert(tree.instance_id.clone()) {
            if let Some(id) = (2..)
                .map(|n| format!("{}-{n}", tree.instance_id))
                .find(|id| taken.insert(id.clone()))
            {
                tree.instance_id = id;
            }
        }
    }
    let species: HashMap<_, _> = catalog
        .trees
        .iter()
        .map(|tree| (tree.id.as_str(), tree))
        .collect();
    let mut distances = HashMap::new();
    let mut constraints = Vec::new();
    let clearance = clearance.max(0.0);
    for tree in incoming.iter() {
        let Some(a) = species.get(tree.tree_id.as_str()) else {
            continue;
        };
        for neighbour in existing {
            let Some(b) = species.get(neighbour.tree_id.as_str()) else {
                continue;
            };
            let radius = *distances
                .entry((a.id.as_str(), b.id.as_str()))
                .or_insert_with(|| separation(catalog, a, b) + clearance);
            constraints.push((tree.x_m, tree.y_m, neighbour.x_m, neighbour.y_m, radius));
        }
    }
    let fits = |dx: f64, dy: f64| {
        !constraints.iter().any(|&(x, y, a, b, limit)| {
            let east = x + dx - a;
            let south = y + dy - b;
            let threshold = limit - 1e-9;
            east.abs() < threshold && south.abs() < threshold && east.hypot(south) < threshold
        })
    };
    let offset = if fits(0.0, 0.0) {
        [0.0; 2]
    } else {
        let reach = |trees: &[Star]| {
            trees
                .iter()
                .map(|tree| tree.x_m.hypot(tree.y_m))
                .fold(0.0, f64::max)
        };
        let required = constraints.iter().map(|c| c.4).fold(0.0, f64::max);
        let bound = reach(incoming) + reach(existing) + required;
        let base = ((step / 0.5).round() * 0.5).max(0.5);
        let coarse = ((bound / 100.0 / 0.5).ceil() * 0.5).max(base);
        search(base, (bound.min(80.0) / base).ceil() as i32 + 1, &fits)
            .or_else(|| {
                (bound > 80.0)
                    .then(|| search(coarse, (bound / coarse).ceil() as i32 + 1, &fits))
                    .flatten()
            })
            .unwrap_or([fixed((bound / coarse).ceil() * coarse + coarse, 2), 0.0])
    };
    for tree in incoming {
        tree.x_m = fixed(tree.x_m + offset[0], 2);
        tree.y_m = fixed(tree.y_m + offset[1], 2);
    }
    offset
}
