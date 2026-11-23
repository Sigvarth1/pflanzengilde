use crate::catalog::Species;
use serde::Deserialize;
use std::f64::consts::PI;

#[derive(Clone, Copy, Default, Deserialize, serde::Serialize, PartialEq, Eq)]
pub enum Zone {
    #[default]
    #[serde(rename = "ZONE_0_COLLAR")]
    Collar,
    #[serde(rename = "ZONE_1_BULB")]
    Bulb,
    #[serde(rename = "ZONE_2_MID")]
    Mid,
    #[serde(rename = "ZONE_3_DRIP")]
    Drip,
    #[serde(rename = "ZONE_4_OUTER")]
    Outer,
}

#[derive(Clone, Copy, Default, Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sector {
    NorthShade,
    EastMorning,
    SouthSun,
    WestWind,
    #[default]
    Any,
}

pub struct Metrics {
    pub canopy: f64,
    pub rings: [f64; 5],
}

pub struct Placement<'a> {
    pub species: &'a Species,
    pub angle: u16,
    pub distance: f64,
    pub zone: Zone,
}

pub(crate) fn cm(distance: f64) -> f64 {
    // Equivalent to rounding positive distances with JavaScript toFixed(2).
    let scaled = distance * 100.0;
    let floor = scaled.floor();
    let fraction = scaled - floor;
    let round_up = fraction > 0.5 || (fraction == 0.5 && distance.mul_add(100.0, -scaled) >= 0.0);
    (floor + if round_up { 1.0 } else { 0.0 }) / 100.0
}

impl Metrics {
    pub fn new(canopy: f64) -> Self {
        let rings = if canopy >= 2.2 {
            [
                0.3,
                1.0,
                (canopy - 0.7).max(1.2),
                canopy + 1.0,
                canopy + 2.5,
            ]
        } else {
            let collar = cm((canopy * 0.2).min(0.2));
            let bulb = cm((canopy * 0.6).max(0.7));
            let mid = cm((bulb + 0.65).max(canopy + 0.35));
            let drip = cm((mid + 0.75).max(canopy + 1.1));
            [collar, bulb, mid, drip, cm((drip + 0.9).max(canopy + 1.9))]
        };
        Self { canopy, rings }
    }
}

impl Sector {
    fn contains(self, angle: u16, southern: bool) -> bool {
        match (self, southern) {
            (Self::Any, _) => true,
            (Self::EastMorning, _) => (45..=135).contains(&angle),
            (Self::WestWind, _) => (225..=315).contains(&angle),
            (Self::NorthShade, false) | (Self::SouthSun, true) => !(46..315).contains(&angle),
            (Self::NorthShade, true) | (Self::SouthSun, false) => (135..=225).contains(&angle),
        }
    }
}

impl Species {
    pub(crate) fn allium(&self) -> bool {
        ["chives", "garlic", "onion"]
            .iter()
            .any(|word| self.id.contains(word))
            || self
                .botanical_name
                .to_ascii_lowercase()
                .starts_with("allium")
    }

    pub(crate) fn legume(&self) -> bool {
        self.nitrogen_fixer
            || [
                "clover",
                "goumi",
                "elaeagnus",
                "seabuckthorn",
                "lupine",
                "alfalfa",
            ]
            .iter()
            .any(|word| self.id.contains(word))
    }

    pub(crate) fn fennel(&self) -> bool {
        self.id.contains("fennel")
            || self
                .botanical_name
                .to_ascii_lowercase()
                .starts_with("foeniculum")
    }

    pub(crate) fn wormwood(&self) -> bool {
        self.id.contains("wormwood")
            || self
                .botanical_name
                .to_ascii_lowercase()
                .starts_with("artemisia absinthium")
    }

    fn allelopath(&self) -> bool {
        self.fennel() || self.wormwood()
    }

    fn priority(&self) -> u8 {
        if self.allium() {
            0
        } else if self.wormwood() {
            4
        } else if self.fennel() {
            3
        } else if self.legume() {
            2
        } else {
            1
        }
    }

    pub(crate) fn competes_with(&self, other: &Self) -> bool {
        self.layer == other.layer
            || matches!(
                (self.layer.as_str(), other.layer.as_str()),
                ("SHRUB", "HERBACEOUS")
                    | ("HERBACEOUS", "SHRUB")
                    | ("CANOPY", "SUB_CANOPY")
                    | ("SUB_CANOPY", "CANOPY")
            )
    }

    fn clearance(&self, other: &Self) -> f64 {
        (self.spread_m + other.spread_m) / 2.0
            * if self.competes_with(other) {
                0.73
            } else {
                0.37
            }
    }
}

pub(crate) fn polar_distance(peer: &Placement, radius: f64, angle: u16) -> f64 {
    let radians = (f64::from(peer.angle) - f64::from(angle)) * PI / 180.0;
    let r = peer.distance;
    (r * r + radius * radius - 2.0 * r * radius * radians.cos())
        .max(0.0)
        .sqrt()
}

fn ladder(angle: u16) -> bool {
    (215..=245).contains(&angle)
}

struct Search<'a, 'p> {
    plant: &'a Species,
    placed: &'p [Placement<'a>],
    southern: bool,
    zone: Zone,
}

#[derive(Clone, Copy)]
struct Candidate {
    angle: u16,
    radius: f64,
    margin: f64,
    score: f64,
}

impl<'a, 'p> Search<'a, 'p> {
    fn angles(&self, strictly_in_sector: bool) -> impl Iterator<Item = u16> + '_ {
        (0..360).step_by(5).filter(move |angle| {
            (self.zone == Zone::Bulb || !ladder(*angle))
                && (!strictly_in_sector || self.plant.sector.contains(*angle, self.southern))
        })
    }

    fn allium_legume(&self, mut radius: f64, drip_inner: f64, trunk_clearance: f64) -> Candidate {
        let legume = self.plant.legume();
        let opponents: Vec<_> = self
            .placed
            .iter()
            .filter(|p| {
                if legume {
                    p.species.allium()
                } else {
                    p.species.legume()
                }
            })
            .collect();
        let same_group: Vec<_> = self
            .placed
            .iter()
            .filter(|p| {
                if legume {
                    p.species.legume()
                } else {
                    p.species.allium()
                }
            })
            .collect();
        if !opponents.is_empty() {
            let farthest = opponents.iter().fold(0.5_f64, |max, p| max.max(p.distance));
            if radius + farthest < 1.95 {
                radius = cm(radius.max(1.95 - farthest + 0.1).max(drip_inner));
            }
        }
        radius = radius.max(trunk_clearance);
        let pick = |radius, strict| {
            let mut best = Candidate {
                angle: 0,
                radius,
                margin: 0.0,
                score: -999999.0,
            };
            for angle in self.angles(strict) {
                let margin = opponents
                    .iter()
                    .map(|p| polar_distance(p, radius, angle))
                    .fold(999.0, f64::min);
                let same_angle = same_group
                    .iter()
                    .map(|p| {
                        f64::from(((i32::from(angle) - i32::from(p.angle) + 540) % 360 - 180).abs())
                    })
                    .fold(999.0, f64::min);
                let penalty: f64 = self
                    .placed
                    .iter()
                    .map(|p| {
                        -(self.plant.clearance(p.species) - polar_distance(p, radius, angle))
                            .max(0.0)
                            * 25.0
                    })
                    .sum();
                let preferred = self.plant.sector != Sector::Any
                    && self.plant.sector.contains(angle, self.southern);
                let score = if margin >= 1.85 {
                    50.0
                } else {
                    (margin - 1.85) * 40.0
                } + margin.min(3.5) * 3.0
                    + same_angle.min(90.0) / 30.0
                    + if preferred { 2.0 } else { 0.0 }
                    + if same_angle < 25.0 { -4.0 } else { 0.0 }
                    + penalty;
                if score > best.score {
                    best = Candidate {
                        angle,
                        radius,
                        margin,
                        score,
                    };
                }
            }
            best
        };
        let mut best = pick(radius, true);
        if best.margin < 1.85 && self.plant.sector != Sector::Any {
            best = pick(radius, false);
        }
        while best.margin < 1.85 && radius < 15.0 {
            radius = cm(radius + 0.35);
            best = pick(radius, false);
        }
        best
    }

    fn allelopath(&self, mut radius: f64, outer: f64) -> Candidate {
        radius = cm(radius
            .max(outer - 0.3)
            .max(if self.plant.fennel() { 1.85 } else { 1.65 }));
        let pick = |radius| {
            let mut best = Candidate {
                angle: 0,
                radius,
                margin: -999.0,
                score: -999999.0,
            };
            for angle in self.angles(false).filter(|angle| !ladder(*angle)) {
                let (mut margin, mut dist) = if self.placed.is_empty() {
                    (1.0_f64, radius)
                } else {
                    (999.0, 999.0)
                };
                for peer in self.placed {
                    let d = polar_distance(peer, radius, angle);
                    let minimum = if self.plant.fennel() || peer.species.fennel() {
                        1.55
                    } else if self.plant.wormwood() || peer.species.wormwood() {
                        1.25
                    } else {
                        0.4
                    };
                    margin = margin.min(d - self.plant.clearance(peer.species).max(minimum));
                    dist = dist.min(d);
                }
                let preferred = self.plant.sector != Sector::Any
                    && self.plant.sector.contains(angle, self.southern);
                let score = if margin >= 0.0 {
                    100.0 + dist.min(4.0) * 2.0
                } else {
                    margin * 50.0
                } + if preferred { 1.5 } else { 0.0 };
                if score > best.score {
                    best = Candidate {
                        angle,
                        radius,
                        margin,
                        score,
                    };
                }
            }
            best
        };
        let mut best = pick(radius);
        while best.margin < 0.0 && radius < 16.0 {
            radius = cm(radius + 0.35);
            best = pick(radius);
        }
        best
    }

    fn standard(
        &self,
        distance: f64,
        min_r: f64,
        max_r: f64,
        span: f64,
        clearance: f64,
    ) -> Candidate {
        let radii = [
            distance,
            min_r + span * 0.2,
            min_r + span * 0.5,
            min_r + span * 0.8,
            max_r,
        ]
        .map(|r| r.max(clearance));
        let pick = |candidates: &[f64], strict| {
            let mut best = Candidate {
                angle: 0,
                radius: candidates[0],
                margin: -999.0,
                score: -999999.0,
            };
            for &radius in candidates {
                for angle in self.angles(strict) {
                    let (mut margin, mut dist) = if self.placed.is_empty() {
                        (1.0_f64, 2.0_f64)
                    } else {
                        (999.0, 999.0)
                    };
                    for peer in self.placed {
                        let d = polar_distance(peer, radius, angle);
                        let minimum: f64 = if peer.species.fennel() {
                            1.55
                        } else if peer.species.wormwood() {
                            1.25
                        } else {
                            0.0
                        };
                        margin = margin.min(d - self.plant.clearance(peer.species).max(minimum));
                        dist = dist.min(d);
                    }
                    let score = if margin >= 0.0 {
                        50.0 + dist.min(2.5) * 2.0
                    } else {
                        margin * 40.0
                    } + if self.plant.sector.contains(angle, self.southern) {
                        3.0
                    } else {
                        0.0
                    } - (radius - distance).abs() * 0.5;
                    if score > best.score {
                        best = Candidate {
                            angle,
                            radius,
                            margin,
                            score,
                        };
                    }
                }
            }
            best
        };
        let mut best = pick(&radii, true);
        if best.margin < 0.0 && self.plant.sector != Sector::Any {
            best = pick(&radii, false);
        }
        for step in 1..=6 {
            if best.margin >= 0.0 {
                break;
            }
            best = pick(&[cm(max_r + f64::from(step) * 0.35)], false);
        }
        best
    }
}

pub fn place<'a>(tree: &Species, plants: &[&'a Species], southern: bool) -> Vec<Placement<'a>> {
    let metrics = Metrics::new(tree.mature_radius_m);
    let mut placed = Vec::<Placement>::new();
    let has_alliums = plants.iter().any(|plant| plant.allium());
    let has_legumes = plants.iter().any(|plant| plant.legume());
    let effective_zone = |plant: &Species| {
        if tree.nitrogen_fixer && plant.allium() && matches!(plant.zone, Zone::Bulb | Zone::Mid) {
            Zone::Drip
        } else if plant.allelopath() {
            Zone::Outer
        } else {
            plant.zone
        }
    };
    for (ring, zone) in [Zone::Bulb, Zone::Mid, Zone::Drip, Zone::Outer]
        .into_iter()
        .enumerate()
    {
        let mut group: Vec<_> = plants
            .iter()
            .copied()
            .filter(|plant| effective_zone(plant) == zone)
            .collect();
        group.sort_by(|a, b| {
            a.priority()
                .cmp(&b.priority())
                .then_with(|| b.spread_m.total_cmp(&a.spread_m))
        });
        let (min_r, max_r) = (metrics.rings[ring], metrics.rings[ring + 1]);
        let span = (max_r - min_r).max(0.2);
        for (index, plant) in group.into_iter().enumerate() {
            let clearance: f64 = if tree.nitrogen_fixer && plant.allium() {
                1.9
            } else {
                0.0
            };
            let distance =
                (min_r + span * 0.35 + (index % 3) as f64 * (span * 0.28)).max(clearance);
            let search = Search {
                plant,
                placed: &placed,
                southern,
                zone,
            };
            let result = if (has_alliums && plant.legume())
                || (has_legumes
                    && plant.allium()
                    && placed.iter().any(|peer| peer.species.legume()))
            {
                search.allium_legume(distance, metrics.rings[2], clearance)
            } else if plant.allelopath() {
                search.allelopath(distance, metrics.rings[4])
            } else {
                search.standard(distance, min_r, max_r, span, clearance)
            };
            let actual_zone = if plant.allelopath() || result.radius > metrics.rings[3] + 0.05 {
                Zone::Outer
            } else if result.radius > metrics.rings[2] + 0.05 {
                Zone::Drip
            } else if result.radius > metrics.rings[1] + 0.05 {
                Zone::Mid
            } else {
                zone
            };
            placed.push(Placement {
                species: plant,
                angle: result.angle,
                distance: cm(result.radius),
                zone: actual_zone,
            });
        }
    }
    placed
}
