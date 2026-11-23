use crate::{
    catalog::Catalog,
    drawing::{number, rounded, safe_color, signed_area, Drawing, Geometry, Instance, Point},
    ground_cover::Cover,
};
use serde_json::{json, Value};
use std::fmt::Write;

pub struct Reference {
    pub point: Point,
    pub lat: f64,
    pub lon: f64,
}

impl Reference {
    pub fn valid(&self) -> bool {
        self.lat.abs() <= 85.0 && self.lon.abs() <= 180.0
    }

    fn project(&self, [x, y]: Point) -> Point {
        let phi = self.lat.to_radians();
        let radius = 6378137.0;
        let eccentricity = 6.69437999014e-3;
        let sin = phi.sin();
        let s = 1.0 - eccentricity * sin * sin;
        let n = radius / s.sqrt();
        let m = radius * (1.0 - eccentricity) / s.powf(1.5);
        [
            rounded(
                (self.lon + (x - self.point[0]) / (n * phi.cos()).to_radians() + 180.0)
                    .rem_euclid(360.0)
                    - 180.0,
                1e7,
            ),
            rounded(self.lat + (y - self.point[1]) / m.to_radians(), 1e7),
        ]
    }
}

pub fn geojson(drawing: &Drawing, reference: &Reference, now: &str) -> String {
    let mut features = Vec::new();
    let mut feature = |kind: &str, shape: Value, mut properties: Value| {
        properties["kind"] = json!(kind);
        properties["plan_name"] = json!(drawing.name);
        features.push(json!({"type":"Feature","geometry":shape,"properties":properties}));
    };
    let ring = |points: &[Point], outer: bool| {
        let ccw = signed_area(points) > 0.0;
        let mut points: Vec<_> = points
            .iter()
            .map(|&point| reference.project(point))
            .collect();
        if ccw != outer {
            points.reverse();
        }
        if let Some(first) = points.first().copied() {
            points.push(first);
        }
        points
    };
    let polygon = |shape: &Geometry, n: usize| json!({"type":"Polygon","coordinates":[ring(&shape.polygon(n), true)]});
    if let Some((shape, area)) = &drawing.outline {
        feature("outline", polygon(shape, 64), json!({"area_m2":area}));
    }
    for bed in &drawing.beds {
        feature(
            "raised_bed",
            polygon(&bed.shape, 64),
            json!({"name":bed.name,"height_m":bed.height,"area_m2":bed.area}),
        );
    }
    for cover in &drawing.ground {
        let polygons: Vec<_> = cover
            .polygons
            .iter()
            .map(|polygon| {
                polygon
                    .iter()
                    .enumerate()
                    .map(|(i, &index)| ring(&cover.rings[index], i == 0))
                    .collect::<Vec<_>>()
            })
            .collect();
        feature(
            "ground_cover",
            json!({"type":"MultiPolygon","coordinates":polygons}),
            json!({"code":cover.code,"name":cover.species.name,"botanical_name":cover.species.botanical_name,
                "plant_id":cover.species.id,"area_m2":cover.area,"height_m":cover.species.height_m,"color":cover.species.color}),
        );
    }
    for (i, tree) in drawing.trees.iter().enumerate() {
        let properties = json!({"code": tree.code,"ref":tree.reference,"no":i+1,"name":tree.label,
            "botanical_name":tree.species.botanical_name,"plant_id":tree.species.id,
            "height_m":tree.species.height_m,"canopy_diameter_m":rounded(tree.width,100.0),
            "plan_x_m":tree.point[0],"plan_y_m":tree.point[1],"color":tree.species.color});
        let canopy = Geometry::Circle {
            center: tree.point,
            radius: tree.species.mature_radius_m,
        };
        feature("tree_canopy", polygon(&canopy, 48), properties.clone());
        feature(
            "tree",
            json!({"type":"Point","coordinates":reference.project(tree.point)}),
            properties,
        );
    }
    for (i, p) in drawing.plants.iter().enumerate() {
        feature(
            "plant",
            json!({"type":"Point","coordinates":reference.project(p.point)}),
            json!({"code":p.code,"ref":p.reference,"no":i+1,"name":p.label,"botanical_name":p.species.botanical_name,
                "plant_id":p.species.id,"height_m":p.species.height_m,"spread_m":p.width,"layer":p.species.layer_label,
                "roles":p.species.roles.join(", "),"serves_trees":p.served.join(" "),"shared":p.shared,
                "light":p.light,"plan_x_m":p.point[0],"plan_y_m":p.point[1],"color":p.species.color}),
        );
    }
    serde_json::to_string_pretty(&json!({
        "type":"FeatureCollection","name":drawing.name,"features":features,
        "pflanzengilde":{"generator":"pflanzengilde.de","created":now,"plan_width_m":drawing.width,"plan_height_m":drawing.height,
            "reference":{"plan_x_m":reference.point[0],"plan_y_m":reference.point[1],"lat":reference.lat,"lon":reference.lon}}
    })).unwrap_or_default()
}

pub fn csv(catalog: &Catalog, drawing: &Drawing, lang: &str) -> String {
    let sep = if lang == "en" { ',' } else { ';' };
    let local = |value, digits| number(value, digits, lang);
    let mut out = String::from("\u{feff}");
    let mut row = |cells: Vec<String>| {
        for (i, mut cell) in cells.into_iter().enumerate() {
            if i != 0 {
                out.push(sep);
            }
            if matches!(cell.trim_start().chars().next(), Some('=' | '+' | '@'))
                || (cell.starts_with('-')
                    && !cell[1..]
                        .chars()
                        .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ',')))
            {
                cell.insert(0, '\'');
            }
            if cell.contains(sep) || cell.contains(['"', '\r', '\n']) {
                out.push('"');
                out.push_str(&cell.replace('"', "\"\""));
                out.push('"');
            } else {
                out.push_str(&cell);
            }
        }
        out.push_str("\r\n");
    };
    row([
        "cadCsvNo",
        "cadCsvCode",
        "cadCsvType",
        "cadCsvName",
        "cadCsvBotanical",
        "cadCsvQuantity",
        "cadCsvUnit",
        "cadCsvX",
        "cadCsvY",
        "cadCsvHeight",
        "cadCsvWidth",
        "cadCsvLayer",
        "cadCsvRoles",
        "cadCsvServes",
        "cadCsvNotes",
    ]
    .map(|key| catalog.text(key))
    .into());
    let plant_row = |p: &Instance, tree: bool| {
        let notes = if tree {
            String::new()
        } else {
            let light = catalog.text(match p.light {
                "FULL_SUN" => "cadLightSun",
                "PARTIAL_SUN" => "cadLightPartial",
                _ => "cadLightShade",
            });
            let mut notes = vec![light];
            if p.shared {
                notes.push(catalog.text("cadNoteShared"));
            }
            if p.pest_defense {
                notes.push(catalog.text("cadNotePest"));
            }
            notes.join(", ")
        };
        let digits = if tree { 1 } else { 2 };
        vec![
            p.reference.clone(),
            p.code.clone(),
            catalog.text(if tree { "cadTypeTree" } else { "cadTypePlant" }),
            p.label.into(),
            p.species.botanical_name.clone(),
            "1".into(),
            catalog.text("cadUnitPiece"),
            local(p.point[0], 2),
            local(p.point[1], 2),
            p.species
                .height_m
                .map_or_else(String::new, |v| local(v, digits)),
            local(p.width, digits),
            p.species.layer_label.clone(),
            if tree {
                String::new()
            } else {
                p.species.roles.join(", ")
            },
            p.served.join(" "),
            notes,
        ]
    };
    for tree in &drawing.trees {
        row(plant_row(tree, true));
    }
    for plant in &drawing.plants {
        row(plant_row(plant, false));
    }
    for cover in &drawing.ground {
        row(vec![
            cover.code.clone(),
            cover.code.clone(),
            catalog.text("cadTypeGroundCover"),
            cover.species.name.clone(),
            cover.species.botanical_name.clone(),
            local(cover.area, 2),
            "m²".into(),
            String::new(),
            String::new(),
            cover
                .species
                .height_m
                .map_or_else(String::new, |v| local(v, 2)),
            local(cover.species.spread_m, 2),
            cover.species.layer_label.clone(),
            cover.species.roles.join(", "),
            String::new(),
            catalog.text("cadNoteGroundCover"),
        ]);
    }
    for (i, bed) in drawing.beds.iter().enumerate() {
        let [x, y] = bed.shape.center();
        let mut cells = vec![String::new(); 15];
        cells[0] = format!("HB-{}", i + 1);
        cells[2] = catalog.text("cadTypeBed");
        cells[3] = bed.name.clone();
        cells[5] = local(bed.area, 2);
        cells[6] = "m²".into();
        cells[7] = local(x, 2);
        cells[8] = local(y, 2);
        cells[9] = local(bed.height, 2);
        row(cells);
    }
    if let Some((_, area)) = &drawing.outline {
        let mut cells = vec![String::new(); 15];
        cells[2] = catalog.text("cadTypeOutline");
        cells[3] = drawing.name.into();
        cells[5] = local(*area, 2);
        cells[6] = "m²".into();
        cells[14] = catalog.text("cadLegendOrigin");
        row(cells);
    }
    out
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn svg(catalog: &Catalog, drawing: &Drawing, lang: &str) -> String {
    let w = drawing.width;
    let h = drawing.height;
    let scale = [50, 100, 200, 250, 500, 1000]
        .into_iter()
        .find(|&scale| w.max(h) * 1000.0 / f64::from(scale) <= 420.0)
        .unwrap_or(1000);
    let margin = (w.max(h) / 55.0).max(0.7);
    let legend = (w.max(h) * 0.3).max(6.0);
    let legend_count =
        drawing.trees.len() + drawing.plants.len() + drawing.beds.len() + drawing.ground.len();
    let text = (w.max(h) / 95.0).clamp(0.15, 0.5);
    let paper_w = w + legend + 3.0 * margin;
    let paper_h = (h + 2.0 * margin).max((legend_count + 5) as f64 * text * 1.5 + 2.0 * margin);
    let mut out = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="{}mm" height="{}mm" viewBox="{} {} {paper_w} {paper_h}">
<title>{}</title><desc>1:{scale}; 1 unit = 1 m; north up</desc>"#,
        paper_w * 1000.0 / f64::from(scale),
        paper_h * 1000.0 / f64::from(scale),
        -margin,
        -margin,
        xml(drawing.name)
    );
    let y = |north: f64| h - north;
    let add_shape = |out: &mut String, shape: &Geometry, class: &str, color: &str| match shape {
        Geometry::Circle {
            center: [x, north],
            radius,
        } => write!(
            out,
            r#"<circle class="{class}" cx="{x}" cy="{}" r="{radius}" fill="none" stroke="{color}"/>"#,
            y(*north)
        ),
        Geometry::Ring(points) => {
            let points = points
                .iter()
                .map(|[x, north]| format!("{x},{}", y(*north)))
                .collect::<Vec<_>>()
                .join(" ");
            write!(
                out,
                r#"<polygon class="{class}" points="{points}" fill="none" stroke="{color}"/>"#
            )
        }
    };
    let columns = if w.max(h) > 60.0 { 5 } else { 1 };
    let _ = write!(
        out,
        r##"<g id="grid" inkscape:groupmode="layer" inkscape:label="Grid" stroke="#aaa498" stroke-width="0.012">"##
    );
    for x in (0..=w as usize).step_by(columns) {
        let _ = write!(out, r#"<path d="M{x} 0v{h}"/>"#);
    }
    for row in (0..=h as usize).step_by(columns) {
        let _ = write!(out, r#"<path d="M0 {row}h{w}"/>"#);
    }
    out.push_str(r#"</g><g id="infrastructure" inkscape:groupmode="layer" inkscape:label="Infrastructure" stroke-width="0.045">"#);
    if let Some((shape, _)) = &drawing.outline {
        let _ = add_shape(&mut out, shape, "outline", "#445740");
    }
    for bed in &drawing.beds {
        let _ = add_shape(&mut out, &bed.shape, "bed", "#914c2a");
    }
    let _ = write!(
        out,
        r#"</g><g id="ground-covers" inkscape:groupmode="layer" inkscape:label="{}" stroke-width="0.033">"#,
        xml(&catalog.text("showGroundCovers"))
    );
    for cover in &drawing.ground {
        let rings: Vec<_> = cover
            .rings
            .iter()
            .map(|ring| ring.iter().map(|[x, north]| [*x, y(*north)]).collect())
            .collect();
        let color = safe_color(&cover.species.color);
        let path = Cover::path(&rings);
        let _ = write!(
            out,
            r#"<g id="{}"><title>{}</title><path d="{path}" fill="{color}" stroke="{color}" fill-rule="evenodd" fill-opacity="{}"/>"#,
            cover.code,
            xml(&cover.species.name),
            cover.opacity
        );
        for &[x, north] in &cover.dots {
            let _ = write!(
                out,
                r#"<circle cx="{x}" cy="{}" r="0.04" fill="{color}" fill-opacity="0.75"/>"#,
                y(north)
            );
        }
        out.push_str("</g>");
    }
    out.push_str(r#"</g><g id="trees" inkscape:groupmode="layer" inkscape:label="Trees" stroke-width="0.035">"#);
    for tree in &drawing.trees {
        let [x, north] = tree.point;
        let yc = y(north);
        let color = safe_color(&tree.species.color);
        let _ = write!(
            out,
            r##"<circle cx="{x}" cy="{yc}" r="{}" fill="{color}" fill-opacity="0.24" stroke="{color}"/><circle cx="{x}" cy="{yc}" r="0.15" fill="#423325"/>"##,
            tree.species.mature_radius_m
        );
    }
    out.push_str(r#"</g><g id="plants" inkscape:groupmode="layer" inkscape:label="Plants" stroke-width="0.022">"#);
    for plant in &drawing.plants {
        let [x, north] = plant.point;
        let _ = write!(
            out,
            r##"<circle cx="{x}" cy="{}" r="{}" fill="{}" stroke="#302d29"><title>{}</title></circle>"##,
            y(north),
            (plant.width / 2.0).max(0.05),
            safe_color(&plant.species.color),
            xml(plant.label)
        );
    }
    let _ = write!(
        out,
        r#"</g><g id="dimensions" inkscape:groupmode="layer" inkscape:label="Distances" font-size="{text}">"#
    );
    for &([x1, y1], [x2, y2], d) in &drawing.dimensions {
        let _ = write!(
            out,
            r##"<path d="M{x1} {}L{x2} {}" stroke="#48648a" stroke-width="0.026"/><text x="{}" y="{}">{}</text>"##,
            y(y1),
            y(y2),
            (x1 + x2) / 2.0,
            (y(y1) + y(y2)) / 2.0 - text * 0.3,
            xml(&format!("{} m", number(d, 2, lang)))
        );
    }
    let _ = write!(
        out,
        r#"</g><g id="labels" inkscape:groupmode="layer" inkscape:label="Labels" font-size="{text}">"#
    );
    for p in drawing.trees.iter().chain(drawing.plants.iter()) {
        let _ = write!(
            out,
            r#"<text x="{}" y="{}">{}</text>"#,
            p.point[0] + text / 2.0,
            y(p.point[1]) + text,
            xml(&p.reference)
        );
    }
    let lx = w + margin;
    let _ = write!(
        out,
        r#"</g><g id="legend" inkscape:groupmode="layer" inkscape:label="Legend" font-size="{text}"><text x="{lx}" y="0">N ↑</text>"#
    );
    let mut row = text * 1.6;
    let mut line = |value: &str| {
        let _ = write!(out, r#"<text x="{lx}" y="{row}">{}</text>"#, xml(value));
        row += text * 1.5;
    };
    line(drawing.name);
    line(&format!("{} 1:{scale}", catalog.text("cadScale")));
    line(&catalog.text("cadLegendOrigin"));
    let mut labels = std::collections::HashSet::new();
    for p in drawing.trees.iter().chain(drawing.plants.iter()) {
        if labels.insert(&p.code) {
            line(&format!("{} {}", p.code, p.species.name));
        }
    }
    for cover in &drawing.ground {
        line(&format!(
            "{} {} · {} m²",
            cover.code,
            cover.species.name,
            number(cover.area, 2, lang)
        ));
    }
    for bed in &drawing.beds {
        line(&format!("{} · {} m²", bed.name, number(bed.area, 2, lang)));
    }
    out.push_str("</g></svg>");
    out
}
