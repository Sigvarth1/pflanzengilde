use crate::{
    catalog::Catalog,
    drawing::{number, safe_color, Drawing, Geometry, Instance, Point},
};
use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Write},
};

const TAGS: [&str; 5] = ["CODE", "NAME", "BOTANICAL", "HEIGHT_M", "WIDTH_M"];

struct Writer {
    data: String,
    next: u32,
}

impl Writer {
    fn pair(&mut self, code: u16, value: impl Display) {
        let _ = write!(self.data, "{code}\r\n{value}\r\n");
    }
    fn handle(&mut self) -> u32 {
        let value = self.next;
        self.next += 1;
        value
    }
    fn reference(&mut self, code: u16, handle: u32) {
        self.pair(code, format_args!("{handle:X}"));
    }
    fn section(&mut self, name: &str) {
        self.pair(0, "SECTION");
        self.pair(2, name);
    }
    fn end_section(&mut self) {
        self.pair(0, "ENDSEC");
    }
    fn table(&mut self, name: &str, count: usize) -> u32 {
        let h = self.handle();
        self.pair(0, "TABLE");
        self.pair(2, name);
        self.reference(5, h);
        self.pair(330, "0");
        self.pair(100, "AcDbSymbolTable");
        self.pair(70, count);
        h
    }
    fn begin_entity(&mut self, name: &str, owner: u32, layer: &str, subclass: &str) -> u32 {
        let h = self.handle();
        self.pair(0, name);
        self.reference(5, h);
        self.reference(330, owner);
        self.pair(100, "AcDbEntity");
        self.pair(8, layer);
        if !subclass.is_empty() {
            self.pair(100, subclass);
        }
        h
    }
    fn point(&mut self, group: u16, [x, y]: Point) {
        self.pair(group, x);
        self.pair(group + 10, y);
        self.pair(group + 20, 0);
    }
    fn circle(
        &mut self,
        owner: u32,
        layer: &str,
        [x, y]: Point,
        radius: f64,
        height: f64,
        color: Option<&str>,
    ) {
        self.begin_entity("CIRCLE", owner, layer, "");
        if let Some(color) = color {
            self.pair(420, rgb(color));
        }
        self.pair(100, "AcDbCircle");
        self.point(10, [x, y]);
        self.pair(40, radius);
        if height != 0.0 {
            self.pair(39, height);
        }
    }
    fn line(&mut self, owner: u32, layer: &str, a: Point, b: Point) {
        self.begin_entity("LINE", owner, layer, "AcDbLine");
        self.point(10, a);
        self.point(11, b);
    }
    fn shape(&mut self, owner: u32, layer: &str, shape: &Geometry, height: f64) {
        match shape {
            Geometry::Circle { center, radius } => {
                self.circle(owner, layer, *center, *radius, height, None)
            }
            Geometry::Ring(points) => self.polyline(owner, layer, points, height, None),
        }
    }
    fn polyline(
        &mut self,
        owner: u32,
        layer: &str,
        points: &[Point],
        height: f64,
        color: Option<&str>,
    ) {
        self.begin_entity("LWPOLYLINE", owner, layer, "");
        if let Some(color) = color {
            self.pair(420, rgb(color));
        }
        self.pair(100, "AcDbPolyline");
        self.pair(90, points.len());
        self.pair(70, 1);
        if height != 0.0 {
            self.pair(39, height);
        }
        for [x, y] in points {
            self.pair(10, x);
            self.pair(20, y);
        }
    }
    fn label(&mut self, owner: u32, layer: &str, value: &str, at: Point, size: f64) {
        self.begin_entity("TEXT", owner, layer, "AcDbText");
        self.point(10, at);
        self.pair(40, size);
        self.pair(1, safe_text(value));
        self.pair(100, "AcDbText");
        self.pair(73, 0);
    }
    fn attribute(
        &mut self,
        owner: u32,
        layer: &str,
        at: Point,
        size: f64,
        tag: &str,
        value: &str,
        visible: bool,
        definition: bool,
    ) {
        self.begin_entity(
            if definition { "ATTDEF" } else { "ATTRIB" },
            owner,
            layer,
            "AcDbText",
        );
        self.point(10, at);
        self.pair(40, size);
        self.pair(1, safe_text(value));
        self.pair(
            100,
            if definition {
                "AcDbAttributeDefinition"
            } else {
                "AcDbAttribute"
            },
        );
        if definition {
            self.pair(3, tag);
        }
        self.pair(2, tag);
        self.pair(70, i32::from(!visible));
    }
    fn block(&mut self, owner: u32, name: &str) {
        self.begin_entity("BLOCK", owner, "0", "AcDbBlockBegin");
        self.pair(2, name);
        self.pair(70, 0);
        self.point(10, [0.0, 0.0]);
        self.pair(3, name);
        self.pair(1, "");
    }
}

fn rgb(color: &str) -> i32 {
    let hex = &safe_color(color)[1..];
    let parsed = i32::from_str_radix(hex, 16).unwrap_or(0x486745);
    if hex.len() == 3 {
        ((parsed & 0xf00) >> 8) * 0x110000 + ((parsed & 0xf0) >> 4) * 0x1100 + (parsed & 0xf) * 0x11
    } else {
        parsed
    }
}

fn safe_text(value: &str) -> String {
    value
        .chars()
        .filter(|&ch| !ch.is_control() || ch == '\r' || ch == '\n')
        .map(|ch| if matches!(ch, '\r' | '\n') { ' ' } else { ch })
        .collect()
}

fn insert(w: &mut Writer, model: u32, layer: &str, p: &Instance, size: f64, is_tree: bool) {
    let owner = w.begin_entity("INSERT", model, layer, "AcDbBlockReference");
    w.pair(66, 1);
    w.pair(2, format_args!("PG_{}", p.code));
    w.point(10, p.point);
    let values = [
        p.code.clone(),
        p.species.name.clone(),
        p.species.botanical_name.clone(),
        p.species
            .height_m
            .map(|height| format!("{height:.2}"))
            .unwrap_or_default(),
        format!("{:.2}", p.width),
    ];
    for (i, (tag, value)) in TAGS.into_iter().zip(values).enumerate() {
        w.attribute(
            owner,
            layer,
            [p.point[0] + 0.2, p.point[1] + 0.2 - i as f64 * size],
            size * 0.85,
            tag,
            &value,
            !is_tree && i == 0,
            false,
        );
    }
    w.begin_entity("SEQEND", owner, layer, "");
}

pub fn create(catalog: &Catalog, drawing: &Drawing, lang: &str) -> String {
    let mut out = Writer {
        data: String::new(),
        next: 16,
    };
    let layers = match lang {
        "nb" => [
            "PG-Rutenett",
            "PG-Tomt",
            "PG-Hoybed",
            "PG-Traer-stamme",
            "PG-Traer-krone",
            "PG-Planter",
            "PG-Etiketter",
            "PG-Avstander",
            "PG-Tegnforklaring",
            "PG-Bunndekke",
        ],
        "de" => [
            "PG-Raster",
            "PG-Grundstueck",
            "PG-Hochbeete",
            "PG-Baeume-Stamm",
            "PG-Baeume-Krone",
            "PG-Pflanzen",
            "PG-Beschriftung",
            "PG-Abstaende",
            "PG-Legende",
            "PG-Bodendecker",
        ],
        _ => [
            "PG-Grid",
            "PG-Outline",
            "PG-Raised-beds",
            "PG-Trees-trunk",
            "PG-Trees-canopy",
            "PG-Plants",
            "PG-Labels",
            "PG-Distances",
            "PG-Legend",
            "PG-Ground-covers",
        ],
    };
    let mut seen = HashSet::new();
    let species: Vec<_> = drawing
        .trees
        .iter()
        .map(|p| (p, true))
        .chain(drawing.plants.iter().map(|p| (p, false)))
        .filter(|(p, _)| seen.insert(&p.code))
        .collect();
    let mut blocks = vec![
        ("*Model_Space".to_owned(), out.handle()),
        ("*Paper_Space".to_owned(), out.handle()),
    ];
    blocks.extend(
        species
            .iter()
            .map(|(p, _)| (format!("PG_{}", p.code), out.handle())),
    );
    let model = blocks[0].1;
    out.section("HEADER");
    for (key, value) in [("$ACADVER", "AC1021"), ("$DWGCODEPAGE", "UTF-8")] {
        out.pair(9, key);
        out.pair(if key == "$ACADVER" { 1 } else { 3 }, value);
    }
    out.pair(9, "$INSUNITS");
    out.pair(70, 6);
    out.pair(9, "$MEASUREMENT");
    out.pair(70, 1);
    out.end_section();

    out.section("TABLES");
    let owner = out.table("LTYPE", 1);
    let h = out.handle();
    out.pair(0, "LTYPE");
    out.reference(5, h);
    out.reference(330, owner);
    out.pair(100, "AcDbSymbolTableRecord");
    out.pair(100, "AcDbLinetypeTableRecord");
    out.pair(2, "CONTINUOUS");
    out.pair(70, 0);
    out.pair(3, "Solid line");
    out.pair(72, 65);
    out.pair(73, 0);
    out.pair(40, 0);
    out.pair(0, "ENDTAB");
    let owner = out.table("LAYER", layers.len() + 1);
    for (layer, color) in
        std::iter::once(("0", 7)).chain(layers.into_iter().zip([9, 7, 1, 8, 3, 4, 7, 5, 7]))
    {
        let h = out.handle();
        out.pair(0, "LAYER");
        out.reference(5, h);
        out.reference(330, owner);
        out.pair(100, "AcDbSymbolTableRecord");
        out.pair(100, "AcDbLayerTableRecord");
        out.pair(2, layer);
        out.pair(70, 0);
        out.pair(62, color);
        out.pair(6, "CONTINUOUS");
    }
    out.pair(0, "ENDTAB");
    let owner = out.table("BLOCK_RECORD", blocks.len());
    for (name, h) in &blocks {
        out.pair(0, "BLOCK_RECORD");
        out.reference(5, *h);
        out.reference(330, owner);
        out.pair(100, "AcDbSymbolTableRecord");
        out.pair(100, "AcDbBlockTableRecord");
        out.pair(2, name);
    }
    out.pair(0, "ENDTAB");
    out.end_section();

    let (w, h) = (drawing.width, drawing.height);
    let size = (w.max(h) / 120.0).clamp(0.15, 0.6);
    out.section("BLOCKS");
    for (i, (name, owner)) in blocks.iter().enumerate() {
        out.block(*owner, name);
        if let Some((p, is_tree)) = i.checked_sub(2).and_then(|i| species.get(i)).copied() {
            out.circle(
                *owner,
                "0",
                [0.0, 0.0],
                if is_tree {
                    0.15
                } else {
                    (p.width / 2.0).max(0.05)
                },
                0.0,
                (!is_tree).then_some(&p.species.color),
            );
            for (i, tag) in TAGS.into_iter().enumerate() {
                out.attribute(
                    *owner,
                    "0",
                    [0.0, -(i as f64) * size],
                    size * 0.85,
                    tag,
                    "",
                    !is_tree && i == 0,
                    true,
                );
            }
        }
        out.begin_entity("ENDBLK", *owner, "0", "AcDbBlockEnd");
    }
    out.end_section();

    out.section("ENTITIES");
    let [grid, outline, beds, trunks, canopies, plants, labels, dimensions, legend, ground] =
        layers;
    let step = if w.max(h) > 60.0 { 5 } else { 1 };
    for x in (0..=w as usize).step_by(step) {
        let x = x as f64;
        out.line(model, grid, [x, 0.0], [x, h]);
    }
    for y in (0..=h as usize).step_by(step) {
        let y = y as f64;
        out.line(model, grid, [0.0, y], [w, y]);
    }
    if let Some((shape, _)) = &drawing.outline {
        out.shape(model, outline, shape, 0.0);
    }
    for bed in &drawing.beds {
        out.shape(model, beds, &bed.shape, bed.height);
        out.label(model, labels, &bed.name, bed.shape.center(), size);
    }
    for cover in &drawing.ground {
        let color = Some(cover.species.color.as_str());
        for ring in &cover.rings {
            out.polyline(model, ground, ring, 0.0, color);
        }
        for &dot in &cover.dots {
            out.circle(model, ground, dot, 0.05, 0.0, color);
        }
    }
    let mut counts = HashMap::new();
    for (p, is_tree) in drawing
        .trees
        .iter()
        .map(|p| (p, true))
        .chain(drawing.plants.iter().map(|p| (p, false)))
    {
        *counts.entry(&p.code).or_insert(0) += 1;
        if is_tree {
            out.circle(
                model,
                canopies,
                p.point,
                p.species.mature_radius_m,
                0.0,
                Some(&p.species.color),
            );
            out.label(
                model,
                labels,
                &format!("{} {}", p.reference, p.label),
                [p.point[0], p.point[1] - 0.45],
                size,
            );
        }
        insert(
            &mut out,
            model,
            if is_tree { trunks } else { plants },
            p,
            size,
            is_tree,
        );
    }
    for &(a, b, distance) in &drawing.dimensions {
        out.line(model, dimensions, a, b);
        out.label(
            model,
            dimensions,
            &format!("{} m", number(distance, 2, lang)),
            [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0 + size / 3.0],
            size,
        );
    }
    let x = w + 2.0;
    out.shape(
        model,
        legend,
        &Geometry::Ring(vec![
            [x, h],
            [x - 0.35, h - 1.0],
            [x, h - 0.8],
            [x + 0.35, h - 1.0],
        ]),
        0.0,
    );
    out.label(model, legend, "N", [x, h + 0.3], size * 1.4);
    let x = w + 3.5;
    let mut y = h - 0.2;
    let mut legend_line = |value: &str| {
        out.label(model, legend, value, [x, y], size);
        y -= size * 1.65;
    };
    legend_line(drawing.name);
    for key in ["cadLegendUnits", "cadLegendOrigin", "cadLegendSchedule"] {
        legend_line(&catalog.text(key));
    }
    for (p, _) in species {
        legend_line(&format!(
            "{}  {} ({})  {}×",
            p.code, p.species.name, p.species.botanical_name, counts[&p.code]
        ));
    }
    for cover in &drawing.ground {
        legend_line(&format!(
            "{}  {}  {} m²",
            cover.code,
            cover.species.name,
            number(cover.area, 2, lang)
        ));
    }
    out.end_section();
    out.pair(0, "EOF");
    out.data
}
