use crate::{
    calendar::Season,
    catalog::{Catalog, Species},
    garden::{Infrastructure, Settings, Star},
    garden_layout::Companion,
    guild::Guild,
    placement::{Placement, Sector},
};
use garden_core::contour::{
    chaikin, even_odd_area, gaussian_blur, marching_squares, signed_area, simplify, Point, Ring,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    f64::consts::{PI, TAU},
    fmt::Write,
};

#[derive(Deserialize)]
pub struct Data {
    species: HashMap<String, Spec>,
    competition: Vec<(String, String, Verdict)>,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Verdict {
    Coexist,
    Exclude,
}

impl Data {
    fn verdict(&self, left: &str, right: &str) -> Option<Verdict> {
        self.competition
            .iter()
            .find(|(a, b, _)| (a == left && b == right) || (b == left && a == right))
            .map(|entry| entry.2)
    }
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Mode {
    Carpet,
    Drift,
    Alley,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum SeasonLayer {
    Summer,
    SpringEphemeral,
    CoolSeason,
}

impl SeasonLayer {
    fn active(self, season: Season) -> bool {
        use Season::*;
        match self {
            Self::Summer => matches!(season, LateSpring | Summer | Autumn),
            Self::SpringEphemeral => matches!(season, EarlySpring | LateSpring),
            Self::CoolSeason => matches!(season, Autumn | Winter | EarlySpring | LateSpring),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Light {
    Sun,
    Half,
    Shade,
    Any,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Drift {
    parents_per_m2: f64,
    children_mean: f64,
    sigma_m: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Spec {
    mode: Mode,
    r_outer_factor: f64,
    #[serde(default)]
    r_inner_extra_m: f64,
    light: Light,
    strategy: String,
    #[serde(default)]
    stolon_rate_m_per_yr: Option<f64>,
    edge: String,
    season_layer: SeasonLayer,
    cover_fraction: f64,
    #[serde(default)]
    dense_mat: bool,
    strip_width_m: Option<f64>,
    #[serde(default)]
    clump_clearance_extra_m: f64,
    drift: Option<Drift>,
}

impl Spec {
    fn hard(&self) -> bool {
        self.edge == "HARD"
    }

    fn guerrilla(&self) -> bool {
        self.strategy == "GUERRILLA"
    }

    fn blend(&self) -> f64 {
        if self.hard() {
            0.03
        } else if self.guerrilla() {
            self.stolon_rate_m_per_yr
                .map_or(0.25, |v| (2.0 * v).max(0.15))
        } else {
            0.08
        }
    }

    fn collar(&self, star: &Species, young: bool) -> f64 {
        let base: f64 = if young && self.season_layer != SeasonLayer::SpringEphemeral {
            0.9
        } else if self.dense_mat {
            0.5
        } else {
            0.3
        };
        if matches!(
            star.activity.tree_category.as_str(),
            "FRUIT_TREE" | "NUT_TREE" | "NITROGEN_FIXING_TREE"
        ) {
            base
        } else {
            base.min((star.mature_radius_m * 0.5).max(0.3))
        }
    }

    fn clump_hole(&self, clump: &Species, young: bool) -> f64 {
        if matches!(clump.layer.as_str(), "SUB_CANOPY" | "CANOPY") {
            return if young && self.season_layer != SeasonLayer::SpringEphemeral {
                0.9
            } else {
                0.3
            };
        }
        let growth = self
            .stolon_rate_m_per_yr
            .unwrap_or(if self.guerrilla() { 0.2 } else { 0.1 });
        (clump.spread_m / 2.0 + growth + self.clump_clearance_extra_m).max(0.1)
    }
}

#[derive(Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cover {
    pub plant_id: String,
    pub instance_ids: Vec<String>,
    pub name: String,
    pub color: String,
    pub hard_edge: bool,
    pub opacity: f64,
    #[serde(skip)]
    pub drift: bool,
    #[serde(skip)]
    season_layer: SeasonLayer,
    pub rings: Vec<Ring>,
    pub dots: Vec<Point>,
    pub area_m2: f64,
    pub detached_anchors: Vec<(String, Point)>,
}

impl Cover {
    pub fn in_season(&self, season: Option<Season>) -> bool {
        season.is_none_or(|season| self.season_layer.active(season))
    }

    pub fn path(rings: &[Ring]) -> String {
        let mut path = String::new();
        for ring in rings {
            let Some(([x, y], rest)) = ring.split_first() else {
                continue;
            };
            let _ = write!(path, "M{x:.2} {y:.2}");
            for [x, y] in rest {
                let _ = write!(path, "L{x:.2} {y:.2}");
            }
            path.push('Z');
        }
        path
    }
}

struct Tree<'a> {
    key: &'a str,
    point: Point,
    species: &'a Species,
}

struct Plant<'a> {
    id: &'a str,
    point: Point,
    species: &'a Species,
    trees: Vec<&'a str>,
}

struct Options<'a> {
    south: bool,
    climate: &'a str,
    young: bool,
    resolution: f64,
    infrastructure: Option<&'a Infrastructure>,
}

#[derive(Clone, Copy)]
struct Bounds {
    min: Point,
    max: Point,
}

impl Bounds {
    fn empty() -> Self {
        Self {
            min: [f64::INFINITY; 2],
            max: [f64::NEG_INFINITY; 2],
        }
    }

    fn around([x, y]: Point, reach: f64) -> Self {
        Self {
            min: [x - reach, y - reach],
            max: [x + reach, y + reach],
        }
    }

    fn include(&mut self, rhs: Self) {
        for axis in 0..2 {
            self.min[axis] = self.min[axis].min(rhs.min[axis]);
            self.max[axis] = self.max[axis].max(rhs.max[axis]);
        }
    }

    fn contains(self, p: Point) -> bool {
        (0..2).all(|axis| self.min[axis] <= p[axis] && p[axis] <= self.max[axis])
    }

    fn spans(self) -> Point {
        [self.max[0] - self.min[0], self.max[1] - self.min[1]]
    }
}

struct Term {
    center: Point,
    inner: f64,
    outer: f64,
    noise: Noise,
    noise_amp: f64,
    shade_center: Point,
    shade_radius: f64,
    angle: f64,
    compartment: Option<usize>,
    bounds: Bounds,
}

#[derive(Clone, Copy)]
struct Obstacle {
    center: Point,
    radius: f64,
    across_wall: f64,
    compartment: Option<usize>,
    reduced_by_wall: bool,
}

impl Obstacle {
    fn radius(self, compartment: Option<usize>) -> f64 {
        if self.reduced_by_wall && self.compartment != compartment {
            self.across_wall
        } else {
            self.radius
        }
    }
}

type Buckets<T> = HashMap<[i32; 2], Vec<T>>;

fn bucket(point: Point) -> [i32; 2] {
    point.map(|value| (value / 2.0).floor() as i32)
}

fn insert<T: Copy>(buckets: &mut Buckets<T>, item: T, bounds: Bounds) {
    let [min_x, min_y] = bucket(bounds.min);
    let [max_x, max_y] = bucket(bounds.max);
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            buckets.entry([x, y]).or_default().push(item);
        }
    }
}

struct Group<'a> {
    spec: &'a Spec,
    species: &'a Species,
    members: Vec<&'a Plant<'a>>,
    trees: Vec<&'a str>,
    terms: Vec<Term>,
    bounds: Bounds,
    blend: f64,
    term_buckets: Buckets<usize>,
    obstacles: Buckets<Obstacle>,
}

struct Grid {
    origin: Point,
    width: usize,
    height: usize,
    own: Vec<f32>,
    affinity: Vec<f32>,
}

impl Grid {
    fn point(&self, x: usize, y: usize, resolution: f64) -> Point {
        [
            self.origin[0] + x as f64 * resolution,
            self.origin[1] + y as f64 * resolution,
        ]
    }

    fn index(&self, [x, y]: Point, res: f64) -> Option<usize> {
        let x = ((x - self.origin[0]) / res + 0.5).floor();
        let y = ((y - self.origin[1]) / res + 0.5).floor();
        if x < 0.0 || y < 0.0 || x >= self.width as f64 || y >= self.height as f64 {
            return None;
        }
        Some(y as usize * self.width + x as usize)
    }
}

fn hash(value: &str) -> u32 {
    value.encode_utf16().fold(2166136261, |hash, unit| {
        (hash ^ u32::from(unit)).wrapping_mul(16777619)
    })
}

struct Random(u32);

impl Random {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let t = (self.0 ^ (self.0 >> 15)).wrapping_mul(self.0 | 1);
        let t = t ^ t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        f64::from(t ^ (t >> 14)) / 4294967296.0
    }
}

struct Noise {
    coarse: [f64; 10],
    fine: [f64; 22],
}

impl Noise {
    fn new(seed: u32) -> Self {
        let mut random = Random(seed);
        Self {
            coarse: std::array::from_fn(|_| random.next() * 2.0 - 1.0),
            fine: std::array::from_fn(|_| random.next() * 2.0 - 1.0),
        }
    }

    fn value(&self, theta: f64) -> f64 {
        let cycle = ((theta / TAU) % 1.0 + 1.0) % 1.0;
        let sample = |values: &[f64]| {
            let position = cycle * values.len() as f64;
            let index = position as usize;
            let weight = (1.0 - (PI * (position - index as f64)).cos()) / 2.0;
            values[index % values.len()] * (1.0 - weight)
                + values[(index + 1) % values.len()] * weight
        };
        (sample(&self.coarse) + 0.45 * sample(&self.fine)) / 1.45
    }
}

fn step(value: f64) -> f64 {
    let x = value.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn distance([x, y]: Point, [a, b]: Point) -> f64 {
    (x - a).hypot(y - b)
}

fn round_cm(value: f64) -> f64 {
    let value = value * 100.0;
    let floor = value.floor();
    (floor + if value >= floor + 0.5 { 1.0 } else { 0.0 }) / 100.0
}

fn shade_offset(star: &Species, climate: &str, scale: f64) -> f64 {
    let (lat, decl): (f64, f64) = match climate {
        "BOREAL" => (60.0, 18.0),
        "SUBTROPICAL" => (35.0, 18.0),
        "TROPICAL" => (15.0, 0.0),
        _ => (50.0, 18.0),
    };
    let height = star.height_m.unwrap_or(star.mature_radius_m * 1.6) * scale;
    let crown = (0.6 * height).max(height - star.mature_radius_m * scale);
    let sun_angle = (90.0 - (lat - decl).abs()) * PI / 180.0;
    (0.6 * crown * (1.0 / sun_angle.tan())).max(0.0)
}

fn direction(plant: &Species, south: bool) -> f64 {
    match plant.sector {
        Sector::NorthShade => {
            if south {
                180.0
            } else {
                0.0
            }
        }
        Sector::EastMorning => 90.0,
        Sector::SouthSun => {
            if south {
                0.0
            } else {
                180.0
            }
        }
        Sector::WestWind => 270.0,
        Sector::Any => f64::from(hash(&plant.id) % 360),
    }
}

fn groups<'a>(data: &'a Data, plants: &'a [Plant<'a>]) -> Vec<Group<'a>> {
    let mut by_species = BTreeMap::<&str, Group>::new();
    for plant in plants {
        let id = plant.species.id.as_str();
        let Some(spec) = data.species.get(id) else {
            continue;
        };
        let group = by_species.entry(id).or_insert_with(|| Group {
            spec,
            species: plant.species,
            members: Vec::new(),
            trees: Vec::new(),
            terms: Vec::new(),
            bounds: Bounds::empty(),
            blend: spec.blend(),
            term_buckets: HashMap::new(),
            obstacles: HashMap::new(),
        });
        group.members.push(plant);
        for &key in &plant.trees {
            if !group.trees.contains(&key) {
                group.trees.push(key);
            }
        }
    }
    by_species.into_values().collect()
}

fn ground_terms(
    groups: &mut [Group<'_>],
    trees: &[Tree<'_>],
    plants: &[Plant<'_>],
    options: &Options<'_>,
    eps: f64,
    compartment: &impl Fn(Point) -> Option<usize>,
    data: &Data,
) {
    let tree_by_key: HashMap<_, _> = trees.iter().map(|tree| (tree.key, tree)).collect();
    for index in 0..groups.len() {
        let spec = groups[index].spec;
        let alley = spec.mode == Mode::Alley;
        let mut terms = Vec::new();
        for &key in &groups[index].trees {
            let Some(tree) = tree_by_key.get(key).copied() else {
                continue;
            };
            let radius = tree.species.mature_radius_m;
            let collar = spec.collar(tree.species, options.young) + spec.r_inner_extra_m;
            let inner = if alley { radius + 0.5 } else { collar };
            let outer = if alley {
                inner + spec.strip_width_m.unwrap_or(1.0)
            } else {
                (inner + 0.6).max(radius * spec.r_outer_factor)
            };
            let scale = if options.young { 0.5 } else { 1.0 };
            let offset = shade_offset(tree.species, options.climate, scale);
            let peers = groups.iter().enumerate().filter(|(_, peer)| {
                peer.spec.season_layer == spec.season_layer
                    && (peer.spec.mode == Mode::Alley) == alley
                    && peer.trees.contains(&key)
            });
            let (mut before, mut count) = (0, 0);
            for (peer, _) in peers {
                before += usize::from(peer < index);
                count += 1;
            }
            let bearing = (direction(groups[index].species, options.south)
                + before as f64 * 360.0 / count.max(1) as f64)
                % 360.0;
            let amp = if alley { 0.06 } else { 0.12 };
            let bounds = Bounds::around(tree.point, outer * (1.0 + amp) + eps);
            terms.push(Term {
                center: tree.point,
                inner,
                outer,
                noise: Noise::new(hash(&format!("{key}|{}", groups[index].species.id))),
                noise_amp: amp,
                shade_center: [
                    tree.point[0],
                    tree.point[1] + if options.south { offset } else { -offset },
                ],
                shade_radius: radius * 0.85 * scale,
                angle: (bearing - 90.0) * PI / 180.0,
                compartment: compartment(tree.point),
                bounds,
            });
        }
        let group = &mut groups[index];
        for (index, term) in terms.iter().enumerate() {
            insert(&mut group.term_buckets, index, term.bounds);
            group.bounds.include(term.bounds);
        }
        group.terms = terms;
        if group.terms.is_empty() {
            continue;
        }
        for star in trees {
            let collar = spec.collar(star.species, options.young);
            let radius = if alley {
                collar.max(star.species.mature_radius_m + 0.5)
            } else {
                collar
            };
            let obstacle = Obstacle {
                center: star.point,
                radius,
                across_wall: radius,
                compartment: None,
                reduced_by_wall: false,
            };
            insert(
                &mut group.obstacles,
                obstacle,
                Bounds::around(star.point, radius + eps),
            );
        }
        for clump in plants
            .iter()
            .filter(|p| !data.species.contains_key(&p.species.id))
        {
            let hole = spec.clump_hole(clump.species, options.young);
            let allium_legume = clump.species.allium() && group.species.legume();
            let chemical: f64 = if clump.species.fennel() {
                1.5
            } else if clump.species.wormwood() {
                1.2
            } else if allium_legume {
                1.8
            } else {
                0.0
            };
            let radius = hole.max(chemical);
            let obstacle = Obstacle {
                center: clump.point,
                radius,
                across_wall: if allium_legume {
                    hole
                } else {
                    hole.max(chemical / 2.0)
                },
                compartment: compartment(clump.point),
                reduced_by_wall: chemical > hole,
            };
            insert(
                &mut group.obstacles,
                obstacle,
                Bounds::around(clump.point, radius + eps),
            );
        }
    }
}

fn raw_field(
    group: &Group<'_>,
    [x, y]: Point,
    eps: f64,
    compartment: Option<usize>,
    values: &mut Vec<f64>,
) -> [f64; 2] {
    let key = bucket([x, y]);
    let Some(terms) = group.term_buckets.get(&key) else {
        return [0.0; 2];
    };
    values.clear();
    let mut affinity: f64 = 0.0;
    let mut maximum: f64 = 0.0;
    for &index in terms {
        let term = &group.terms[index];
        if term.compartment != compartment || !term.bounds.contains([x, y]) {
            continue;
        }
        let [dx, dy] = [x - term.center[0], y - term.center[1]];
        let d = dx.hypot(dy);
        if d > term.outer * (1.0 + term.noise_amp) + eps || d < term.inner {
            continue;
        }
        let theta = dy.atan2(dx);
        let outer = term.outer * (1.0 + term.noise_amp * term.noise.value(theta));
        let mut value = step(0.5 + (d - term.inner) / eps) * step(0.5 + (outer - d) / eps);
        if value <= 0.0 {
            continue;
        }
        if group.spec.season_layer != SeasonLayer::SpringEphemeral && group.spec.mode != Mode::Alley
        {
            let ds = distance([x, y], term.shade_center);
            let shade = 1.0 - step(0.5 + (ds - term.shade_radius) / 0.5);
            value *= match group.spec.light {
                Light::Shade => 0.25 + 0.75 * shade,
                Light::Sun => 1.0 - 0.4 * shade,
                _ => 1.0,
            };
        }
        values.push(value);
        maximum = maximum.max(value);
        affinity = affinity.max(0.5 + 0.5 * (theta - term.angle).cos());
    }
    let mut density = match values.len() {
        0 => return [0.0; 2],
        1 => maximum,
        _ => {
            let sum: f64 = values.iter().map(|v| (8.0 * (v - maximum)).exp()).sum();
            (maximum + sum.ln() / 8.0).clamp(0.0, 1.0)
        }
    };
    for obstacle in group.obstacles.get(&key).into_iter().flatten() {
        let d = distance([x, y], obstacle.center);
        if d < obstacle.radius + eps {
            density *= step(0.5 + (d - obstacle.radius(compartment)) / eps);
            if density <= 0.0 {
                return [0.0; 2];
            }
        }
    }
    [density, density * (0.55 + 0.45 * affinity)]
}

fn sample_group(
    group: &Group<'_>,
    res: f64,
    eps: f64,
    infrastructure: Option<&Infrastructure>,
    compartment: &impl Fn(Point) -> Option<usize>,
) -> Option<Grid> {
    if group.terms.is_empty() {
        return None;
    }
    let origin = group
        .bounds
        .min
        .map(|value| ((value / res).floor() - 1.0) * res);
    let [width, height] = std::array::from_fn(|axis| {
        ((group.bounds.max[axis] - origin[axis]) / res).ceil() as usize + 2
    });
    let mut grid = Grid {
        origin,
        width,
        height,
        own: vec![0.0; width * height],
        affinity: vec![0.0; width * height],
    };
    let outline = infrastructure.and_then(|infra| infra.outline.as_ref());
    let mut values = Vec::new();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let point = grid.point(x, y, res);
            if !group.term_buckets.contains_key(&bucket(point))
                || outline.is_some_and(|shape| !shape.contains(point))
            {
                continue;
            }
            let [density, affinity] = raw_field(group, point, eps, compartment(point), &mut values);
            grid.own[y * width + x] = density as f32;
            grid.affinity[y * width + x] = affinity as f32;
        }
    }
    Some(grid)
}

fn shared_field(
    data: &Data,
    groups: &[Group<'_>],
    grids: &[Option<Grid>],
    index: usize,
    grid: &Grid,
    res: f64,
) -> Vec<f32> {
    let group = &groups[index];
    let peers: Vec<_> = groups
        .iter()
        .zip(grids)
        .enumerate()
        .filter_map(|(other, (peer, grid))| {
            let grid = grid.as_ref()?;
            if index == other {
                return None;
            }
            let verdict = data.verdict(&group.species.id, &peer.species.id);
            if verdict == Some(Verdict::Coexist) {
                return None;
            }
            let mix = if group.spec.hard() || peer.spec.hard() || verdict == Some(Verdict::Exclude)
            {
                0.0
            } else if group.spec.guerrilla() && peer.spec.guerrilla() {
                (group.blend * 0.7).min(0.25)
            } else {
                (group.blend * 0.4).min(0.12)
            };
            Some((grid, (peer.blend * 0.4).max(0.03), mix))
        })
        .collect();
    if peers.is_empty() {
        return grid
            .own
            .iter()
            .map(|&own| if own >= 0.5 { 1.0 } else { 0.0 })
            .collect();
    }
    let mut values = Vec::new();
    grid.own
        .iter()
        .enumerate()
        .map(|(index, &own)| {
            if own < 0.5 {
                return 0.0;
            }
            let point = grid.point(index % grid.width, index / grid.width, res);
            values.clear();
            let mut tau = (group.blend * 0.4).max(0.03);
            let mut overlap = f64::INFINITY;
            for &(peer, temperature, mix) in &peers {
                let Some(sample) = peer.index(point, res) else {
                    continue;
                };
                if peer.own[sample] < 0.5 || peer.affinity[sample] <= 0.0 {
                    continue;
                }
                values.push(f64::from(peer.affinity[sample]));
                tau = tau.min(temperature);
                overlap = overlap.min(mix);
            }
            if values.is_empty() {
                return 1.0;
            }
            let own = (f64::from(grid.affinity[index]) / tau).exp();
            let sum = values
                .iter()
                .fold(own, |sum, value| sum + (value / tau).exp());
            (0.5 + 2.0 * (own / sum - 0.5) + overlap).clamp(0.0, 1.0) as f32
        })
        .collect()
}

fn enforce(
    group: &Group<'_>,
    point: Point,
    res: f64,
    compartment: Option<usize>,
    circles: &mut Vec<(Point, f64)>,
) -> Point {
    circles.clear();
    circles.extend(group.terms.iter().map(|t| (t.center, t.inner)));
    circles.extend(
        group
            .obstacles
            .get(&bucket(point))
            .into_iter()
            .flatten()
            .map(|o| (o.center, o.radius(compartment))),
    );
    let mut candidate = point;
    let max_fix = (2.5 * res).max(0.12);
    for (index, &(center, radius)) in circles.iter().enumerate() {
        let d = distance(candidate, center);
        if d >= radius || d <= 1e-6 || radius - d > max_fix {
            continue;
        }
        let proposed = std::array::from_fn(|axis| {
            center[axis] + ((candidate[axis] - center[axis]) / d) * (radius + 0.005)
        });
        if circles
            .iter()
            .enumerate()
            .all(|(other, &(p, r))| index == other || distance(proposed, p) >= r - 0.005)
        {
            candidate = proposed;
        }
    }
    candidate
}

// A negative value rounded to zero keeps its sign in JavaScript's toFixed.
fn js_fixed_1(value: f64) -> String {
    if value == 0.0 {
        return "0.0".into();
    }
    let scaled = value.abs() * 10.0;
    let floor = scaled.floor();
    let fraction = scaled - floor;
    let up = fraction > 0.5 || (fraction == 0.5 && value.abs().mul_add(10.0, -scaled) >= 0.0);
    let magnitude = (floor + if up { 1.0 } else { 0.0 }) / 10.0;
    format!("{:.1}", magnitude.copysign(value))
}

fn drift_dots(group: &Group<'_>, drift: &Drift, field_at: &impl Fn(Point) -> f32) -> Vec<Point> {
    let mut key = format!("drift|{}|", group.species.id);
    for (index, term) in group.terms.iter().enumerate() {
        if index != 0 {
            key.push(';');
        }
        let _ = write!(
            key,
            "{},{}",
            js_fixed_1(term.center[0]),
            js_fixed_1(term.center[1])
        );
    }
    let mut random = Random(hash(&key));
    let [width, height] = group.bounds.spans();
    let count = (width * height * drift.parents_per_m2).round().min(120.0) as usize;
    let mut dots = Vec::new();
    for _ in 0..count {
        let [x, y] = [
            group.bounds.min[0] + random.next() * width,
            group.bounds.min[1] + random.next() * height,
        ];
        if field_at([x, y]) < 0.5 {
            continue;
        }
        let children = (drift.children_mean * (0.5 + random.next()))
            .round()
            .max(1.0) as usize;
        for _ in 0..children {
            let u1 = random.next().max(1e-9);
            let u2 = random.next();
            let radius = drift.sigma_m * (-2.0 * u1.ln()).sqrt();
            let candidate = [x + radius * (TAU * u2).cos(), y + radius * (TAU * u2).sin()];
            if field_at(candidate) >= 0.5 {
                dots.push(candidate.map(round_cm));
                if dots.len() == 400 {
                    return dots;
                }
            }
        }
    }
    dots
}

fn compute(
    data: &Data,
    trees: &[Tree<'_>],
    plants: &[Plant<'_>],
    mut options: Options<'_>,
) -> Vec<Cover> {
    let mut groups = groups(data, plants);
    if groups.is_empty() {
        return Vec::new();
    }
    let mut bed_shapes: Vec<_> = options
        .infrastructure
        .into_iter()
        .flat_map(|infra| &infra.raised_beds)
        .enumerate()
        .map(|(index, bed)| (index, bed.shape.area(), &bed.shape))
        .collect();
    bed_shapes.sort_by(|a, b| a.1.total_cmp(&b.1));
    let compartment = |point| {
        bed_shapes
            .iter()
            .find(|(_, _, shape)| shape.contains(point))
            .map(|entry| entry.0)
    };
    let mut eps = (2.0 * options.resolution).max(0.08);
    ground_terms(
        &mut groups,
        trees,
        plants,
        &options,
        eps,
        &compartment,
        data,
    );
    // An unusually large imported garden can span hundreds of metres. Bound
    // the combined field size while leaving ordinary plans on the legacy lattice.
    let units: f64 = groups
        .iter()
        .filter(|g| !g.terms.is_empty())
        .map(|g| {
            let [w, h] = g.bounds.spans();
            w * h
        })
        .sum();
    let minimum = (units / 2_000_000.0).sqrt();
    if minimum > options.resolution {
        options.resolution = (minimum * 100.0).ceil() / 100.0;
        eps = (2.0 * options.resolution).max(0.08);
        groups.iter_mut().for_each(|group| {
            group.bounds = Bounds::empty();
            group.term_buckets.clear();
            group.obstacles.clear();
        });
        ground_terms(
            &mut groups,
            trees,
            plants,
            &options,
            eps,
            &compartment,
            data,
        );
    }
    let res = options.resolution;
    let grids: Vec<_> = groups
        .iter()
        .map(|g| sample_group(g, res, eps, options.infrastructure, &compartment))
        .collect();
    let mut circles = Vec::new();
    groups
        .iter()
        .enumerate()
        .filter_map(|(index, group)| {
            let grid = grids[index].as_ref()?;
            let field = shared_field(data, &groups, &grids, index, grid, res);
            let field = gaussian_blur(
                field,
                grid.width,
                grid.height,
                0.06_f64.max(res * 1.2) / res,
            );
            let rings: Vec<_> = marching_squares(&field, grid.width, grid.height, 0.5)
                .into_iter()
                .filter_map(|ring| {
                    let metric = ring
                        .into_iter()
                        .map(|[x, y]| {
                            [
                                round_cm(grid.origin[0] + x * res),
                                round_cm(grid.origin[1] + y * res),
                            ]
                        })
                        .collect();
                    let smooth = simplify(
                        chaikin(simplify(metric, (res * 0.3).max(0.02)), 2),
                        (res * 0.15).max(0.01),
                    );
                    let corrected: Ring = smooth
                        .into_iter()
                        .map(|point| {
                            enforce(group, point, res, compartment(point), &mut circles)
                                .map(round_cm)
                        })
                        .collect();
                    (corrected.len() >= 3 && signed_area(&corrected).abs() >= 0.1)
                        .then_some(corrected)
                })
                .collect();
            if rings.is_empty() {
                return None;
            }
            let field_at = |point| grid.index(point, res).map_or(0.0, |index| field[index]);
            let dots = group
                .spec
                .drift
                .as_ref()
                .map_or_else(Vec::new, |drift| drift_dots(group, drift, &field_at));
            let detached_anchors = group
                .members
                .iter()
                .filter(|plant| field_at(plant.point) < 0.5)
                .map(|plant| (plant.id.to_owned(), plant.point))
                .collect();
            Some(Cover {
                plant_id: group.species.id.clone(),
                instance_ids: group
                    .members
                    .iter()
                    .map(|plant| plant.id.to_owned())
                    .collect(),
                name: group.species.name.clone(),
                color: group.species.color.clone(),
                hard_edge: group.spec.hard(),
                drift: group.spec.drift.is_some(),
                opacity: if group.spec.drift.is_some() {
                    0.14
                } else {
                    0.16 + 0.24 * group.spec.cover_fraction
                },
                season_layer: group.spec.season_layer,
                area_m2: even_odd_area(&rings),
                rings,
                dots,
                detached_anchors,
            })
        })
        .collect()
}

pub fn radial(data: &Data, star: &Species, placed: &[Placement<'_>], guild: &Guild) -> Vec<Cover> {
    let trees = [Tree {
        key: "star",
        point: [0.0; 2],
        species: star,
    }];
    let plants: Vec<_> = placed
        .iter()
        .map(|p| {
            let radians = (f64::from(p.angle) - 90.0) * PI / 180.0;
            Plant {
                id: &p.species.id,
                point: [p.distance * radians.cos(), p.distance * radians.sin()],
                species: p.species,
                trees: vec!["star"],
            }
        })
        .collect();
    compute(
        data,
        &trees,
        &plants,
        Options {
            south: guild.hemisphere == "SOUTHERN",
            climate: &guild.climate_zone,
            young: guild.tree_age != "ESTABLISHED",
            resolution: 0.06,
            infrastructure: None,
        },
    )
}

pub fn garden(
    data: &Data,
    catalog: &Catalog,
    stars: &[Star],
    companions: &[Companion],
    infrastructure: &Infrastructure,
    settings: &Settings,
) -> Vec<Cover> {
    garden_at_resolution(
        data,
        catalog,
        stars,
        companions,
        infrastructure,
        settings,
        None,
    )
}

pub fn garden_at_resolution(
    data: &Data,
    catalog: &Catalog,
    stars: &[Star],
    companions: &[Companion],
    infrastructure: &Infrastructure,
    settings: &Settings,
    resolution: Option<f64>,
) -> Vec<Cover> {
    if !companions
        .iter()
        .any(|plant| data.species.contains_key(&plant.plant_id))
    {
        return Vec::new();
    }
    let tree_lookup: HashMap<_, _> = catalog
        .trees
        .iter()
        .map(|tree| (tree.id.as_str(), tree))
        .collect();
    let plant_lookup: HashMap<_, _> = catalog
        .plants
        .iter()
        .map(|plant| (plant.id.as_str(), plant))
        .collect();
    let trees: Vec<_> = stars
        .iter()
        .filter_map(|tree| {
            let species = *tree_lookup.get(tree.tree_id.as_str())?;
            Some(Tree {
                key: &tree.instance_id,
                point: [tree.x_m, tree.y_m],
                species,
            })
        })
        .collect();
    let plants: Vec<_> = companions
        .iter()
        .filter_map(|plant| {
            let species = *plant_lookup.get(plant.plant_id.as_str())?;
            Some(Plant {
                id: &plant.instance_id,
                point: [plant.x_m, plant.y_m],
                species,
                trees: plant
                    .servicing_tree_ids
                    .iter()
                    .map(String::as_str)
                    .collect(),
            })
        })
        .collect();
    let mut bounds = Bounds::empty();
    for tree in &trees {
        bounds.include(Bounds::around(tree.point, 0.0));
    }
    let [w, h] = bounds.spans().map(|span| (span + 12.0).max(4.0));
    let res = resolution.unwrap_or_else(|| round_cm((w * h / 60000.0).sqrt().clamp(0.1, 0.25)));
    compute(
        data,
        &trees,
        &plants,
        Options {
            south: settings.southern,
            climate: settings.climate.as_deref().unwrap_or("TEMPERATE"),
            young: !settings.established,
            resolution: res,
            infrastructure: Some(infrastructure),
        },
    )
}
