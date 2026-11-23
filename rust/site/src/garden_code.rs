use crate::{
    catalog::Catalog,
    garden::{Bed, Garden, Infrastructure, Settings, Shape, Star},
    guild::{
        climate_index, decode_plant_mask, encode_plant_mask, push_varint, read_varint, soil_index,
        CLIMATES, SOILS,
    },
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

const VERSION: u8 = 2;
const SHADE: u8 = 1 << 3;
const NAME: u8 = 1 << 4;
const EXTENSION: u8 = 1 << 5;
const ESTABLISHED: u8 = 1 << 6;

struct Cursor<'a>(&'a [u8]);

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (value, rest) = self.0.split_at_checked(count)?;
        self.0 = rest;
        Some(value)
    }

    fn byte(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn varint(&mut self) -> Option<usize> {
        let (value, rest) = read_varint(self.0)?;
        self.0 = rest;
        Some(value)
    }

    fn field(&mut self, legacy: bool) -> Option<usize> {
        if legacy {
            self.byte().map(usize::from)
        } else {
            self.varint()
        }
    }

    fn integer(&mut self) -> Option<i16> {
        Some(i16::from_be_bytes(self.take(2)?.try_into().ok()?))
    }

    fn centimeters(&mut self) -> Option<f64> {
        Some(f64::from(self.integer()?) / 100.0)
    }

    fn shape(&mut self) -> Option<Shape> {
        Some(match self.byte()? {
            0 => Shape::Rect {
                x_m: self.centimeters()?,
                y_m: self.centimeters()?,
                w_m: self.centimeters()?,
                h_m: self.centimeters()?,
            },
            1 => Shape::Circle {
                cx_m: self.centimeters()?,
                cy_m: self.centimeters()?,
                r_m: self.centimeters()?,
            },
            2 => {
                let count = self.varint()?;
                if !(3..=256).contains(&count) {
                    return None;
                }
                let (mut x, mut y) = (i64::from(self.integer()?), i64::from(self.integer()?));
                let mut points = vec![[x as f64 / 100.0, y as f64 / 100.0]];
                for _ in 1..count {
                    x = x.checked_add(unzigzag(self.varint()?))?;
                    y = y.checked_add(unzigzag(self.varint()?))?;
                    points.push([x as f64 / 100.0, y as f64 / 100.0]);
                }
                Shape::Polygon { points }
            }
            _ => return None,
        })
    }

    fn infrastructure(&mut self) -> Option<Infrastructure> {
        let outline = if self.varint()? & 1 != 0 {
            Some(self.shape()?)
        } else {
            None
        };
        let mut raised_beds = Vec::new();
        for i in 1..=self.varint().unwrap_or(0).min(64) {
            let shape = self.shape()?;
            let Some(height) = self.byte() else { break };
            raised_beds.push(Bed {
                id: format!("bed-{i}"),
                shape,
                height_m: f64::from(height) / 20.0,
                name: None,
            });
        }
        Some(Infrastructure {
            outline,
            raised_beds,
        })
    }
}

fn unzigzag(value: usize) -> i64 {
    let magnitude = (value / 2) as i64;
    if value % 2 == 0 {
        magnitude
    } else {
        -magnitude - 1
    }
}

fn zigzag(value: i32) -> usize {
    if value >= 0 {
        (value * 2) as usize
    } else {
        (-2 * value - 1) as usize
    }
}

fn js_round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

fn integer(value: f64, scale: f64) -> i16 {
    js_round(value * scale) as i16
}

fn shape_bytes(shape: &Shape, out: &mut Vec<u8>) {
    match shape {
        Shape::Rect { x_m, y_m, w_m, h_m } => {
            out.push(0);
            for value in [x_m, y_m, w_m, h_m] {
                out.extend(integer(*value, 100.0).to_be_bytes());
            }
        }
        Shape::Circle { cx_m, cy_m, r_m } => {
            out.push(1);
            for value in [cx_m, cy_m, r_m] {
                out.extend(integer(*value, 100.0).to_be_bytes());
            }
        }
        Shape::Polygon { points } => {
            out.push(2);
            let count = points.len().min(256);
            push_varint(out, count);
            let Some((&first, rest)) = points[..count].split_first() else {
                return;
            };
            let mut previous = first.map(|coordinate| integer(coordinate, 100.0));
            for value in previous {
                out.extend(value.to_be_bytes());
            }
            for point in rest {
                let next = point.map(|coordinate| integer(coordinate, 100.0));
                push_varint(out, zigzag(i32::from(next[0]) - i32::from(previous[0])));
                push_varint(out, zigzag(i32::from(next[1]) - i32::from(previous[1])));
                previous = next;
            }
        }
    }
}

pub fn encode(catalog: &Catalog, garden: &Garden, settings: &Settings) -> String {
    let soil = settings.soil.as_deref().map_or(7, soil_index) as u8;
    let climate = settings.climate.as_deref().map_or(7, climate_index) as u8;
    let header = soil
        | (climate << 3)
        | (u8::from(settings.southern) << 6)
        | (u8::from(settings.english) << 7);
    let name = garden.garden_name.trim();
    let named = !matches!(
        name,
        "" | "Mein Permakultur-Garten" | "My Permaculture Garden"
    );
    let infra = &garden.infrastructure;
    let extension = infra.outline.is_some() || !infra.raised_beds.is_empty();
    let flags = VERSION
        | if garden.auto_shade_enabled { SHADE } else { 0 }
        | if named { NAME } else { 0 }
        | if extension { EXTENSION } else { 0 }
        | if settings.established { ESTABLISHED } else { 0 };
    let mut out = vec![header, flags];
    let stars: Vec<_> = garden
        .star_plants
        .iter()
        .filter_map(|star| {
            catalog
                .trees
                .iter()
                .position(|tree| tree.id == star.tree_id)
                .map(|index| (star, index))
        })
        .collect();
    push_varint(&mut out, stars.len());
    for (star, index) in stars {
        push_varint(&mut out, index);
        out.extend(integer(star.x_m, 10.0).to_be_bytes());
        out.extend(integer(star.y_m, 10.0).to_be_bytes());
        let mask = encode_plant_mask(catalog, &star.selected_plant_ids);
        push_varint(&mut out, mask.len());
        out.extend(mask);
    }
    if named {
        let name: String = name.chars().take(64).collect();
        push_varint(&mut out, name.len());
        out.extend(name.as_bytes());
    }
    if extension {
        push_varint(&mut out, usize::from(infra.outline.is_some()));
        if let Some(outline) = &infra.outline {
            shape_bytes(outline, &mut out);
        }
        let count = infra.raised_beds.len().min(64);
        push_varint(&mut out, count);
        for bed in &infra.raised_beds[..count] {
            shape_bytes(&bed.shape, &mut out);
            out.push(js_round(bed.height_m * 100.0 / 5.0).clamp(1.0, 255.0) as u8);
        }
    }
    URL_SAFE_NO_PAD.encode(out)
}

fn next_star<'a>(fields: &mut Cursor<'a>, legacy: bool) -> Option<(usize, f64, f64, &'a [u8])> {
    let index = fields.field(legacy)?;
    let x = f64::from(fields.integer()?) / 10.0;
    let y = f64::from(fields.integer()?) / 10.0;
    let len = fields.field(legacy)?;
    Some((index, x, y, fields.take(len)?))
}

/// Opens version 2 codes and the fixed-width format used by older links.
pub fn decode(catalog: &Catalog, code: &str) -> Option<(Garden, Settings)> {
    let bytes = URL_SAFE_NO_PAD
        .decode(code.trim().trim_end_matches('='))
        .ok()?;
    let mut fields = Cursor(&bytes);
    let header = fields.byte()?;
    let flags = fields.byte()?;
    let legacy = flags & 7 != VERSION;
    let count = fields.field(legacy)?;
    let mut garden = Garden {
        auto_shade_enabled: flags & SHADE != 0,
        ..Garden::default()
    };
    for i in 0..count {
        let Some((index, x, y, mask)) = next_star(&mut fields, legacy) else {
            break;
        };
        let species = catalog
            .trees
            .get(index)
            .or_else(|| legacy.then(|| catalog.trees.first()).flatten());
        if let Some(tree) = species {
            let id = format!("garden-tree-{i}-{}", tree.id);
            garden
                .star_plants
                .push(Star::new(tree, id, x, y, decode_plant_mask(catalog, mask)));
        }
    }
    if flags & NAME != 0 && !fields.0.is_empty() {
        let mut name = Cursor(fields.0);
        if let Some(raw) = name.field(legacy).and_then(|length| name.take(length)) {
            garden.garden_name = String::from_utf8_lossy(raw).into_owned();
            fields = name;
        }
    }
    if !legacy && flags & EXTENSION != 0 {
        garden.infrastructure = fields.infrastructure().unwrap_or_default();
    }
    let settings = Settings {
        soil: SOILS
            .get(usize::from(header & 7))
            .map(|(value, _)| (*value).into()),
        climate: CLIMATES
            .get(usize::from((header >> 3) & 7))
            .map(|(value, _)| (*value).into()),
        southern: header & 64 != 0,
        english: header & 128 != 0,
        established: !legacy && flags & ESTABLISHED != 0,
    };
    Some((garden, settings))
}
