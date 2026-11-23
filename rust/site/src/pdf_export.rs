use crate::{
    calendar::Season,
    catalog::{Catalog, Species},
    drawing::{Drawing, Geometry, Point},
    garden::Settings,
    garden_conflicts::{Conflict, Severity, ShadePocket, SHADE_PLANTS},
    ground_cover,
    guild::{Guild, CLIMATES, SOILS},
    placement::{place, Metrics, Zone},
    seasonal,
};
use std::{collections::HashMap, fmt::Write};

// A4 landscape, in PDF points.
const WIDTH: f64 = 841.89;
const HEIGHT: f64 = 595.28;
const MARGIN: f64 = 42.0;
const INK: Color = [0.12, 0.20, 0.16];
const MUTED: Color = [0.29, 0.36, 0.32];
const LIGHT: Color = [0.88, 0.92, 0.89];
const PANEL: Color = [0.97, 0.98, 0.97];
const GREEN: Color = [0.21, 0.39, 0.29];
const BED: Color = [0.88, 0.81, 0.69];
type Color = [f64; 3];

#[derive(Default)]
struct Page(String);

impl Page {
    fn text(&mut self, [x, y]: Point, size: f64, bold: bool, color: Color, text: &str) {
        let [r, g, b] = color;
        let font = if bold { "Bold" } else { "Regular" };
        let _ = write!(
            self.0,
            "BT {r:.3} {g:.3} {b:.3} rg /{font} {size} Tf {x:.2} {y:.2} Td <"
        );
        for ch in text.chars() {
            let byte = match ch {
                ' '..='~' | '\u{a0}'..='ÿ' => ch as u8,
                '€' => 0x80,
                'Š' => 0x8a,
                'Œ' => 0x8c,
                'Ž' => 0x8e,
                '‘' => 0x91,
                '’' => 0x92,
                '“' => 0x93,
                '”' => 0x94,
                '•' => 0x95,
                '–' => 0x96,
                '—' => 0x97,
                'š' => 0x9a,
                'œ' => 0x9c,
                'ž' => 0x9e,
                'Ÿ' => 0x9f,
                '…' => 0x85,
                ch if ch.is_whitespace() => b' ',
                _ => b'?',
            };
            let _ = write!(self.0, "{byte:02X}");
        }
        self.0.push_str("> Tj ET\n");
    }

    fn stroke(&mut self, width: f64, [r, g, b]: Color) {
        let _ = writeln!(self.0, "{width:.2} w {r:.3} {g:.3} {b:.3} RG");
    }

    fn fill(&mut self, [r, g, b]: Color) {
        let _ = writeln!(self.0, "{r:.3} {g:.3} {b:.3} rg");
    }

    fn rect(&mut self, [x, y]: Point, width: f64, height: f64, color: Color) {
        self.fill(color);
        let _ = writeln!(self.0, "{x:.2} {y:.2} {width:.2} {height:.2} re f");
    }

    fn line(&mut self, [x, y]: Point, [ex, ey]: Point, width: f64, color: Color) {
        self.stroke(width, color);
        let _ = writeln!(self.0, "{x:.2} {y:.2} m {ex:.2} {ey:.2} l S");
    }

    fn outline(&mut self, center: Point, radius: f64, color: Color) {
        self.stroke(0.7, color);
        self.circle_path(center, radius);
        self.0.push_str("S\n");
    }

    fn circle(&mut self, center: Point, radius: f64, color: Color) {
        self.fill(color);
        self.circle_path(center, radius);
        self.0.push_str("f\n");
    }

    fn circle_path(&mut self, [x, y]: Point, r: f64) {
        let t = r * 0.5522847498;
        let _ = writeln!(
            self.0,
            "{:.2} {y:.2} m {:.2} {:.2} {:.2} {:.2} {x:.2} {:.2} c {:.2} {:.2} {:.2} {:.2} {:.2} {y:.2} c {:.2} {:.2} {:.2} {:.2} {x:.2} {:.2} c {:.2} {:.2} {:.2} {:.2} {:.2} {y:.2} c h",
            x + r, x + r, y + t, x + t, y + r, y + r,
            x - t, y + r, x - r, y + t, x - r,
            x - r, y - t, x - t, y - r, y - r,
            x + t, y - r, x + r, y - t, x + r
        );
    }

    fn shape(
        &mut self,
        shape: &Geometry,
        project: &impl Fn(Point) -> Point,
        scale: f64,
        fill: Option<Color>,
    ) {
        self.stroke(1.1, GREEN);
        if let Some(color) = fill {
            self.fill(color);
        }
        match shape {
            Geometry::Circle { center, radius } => {
                self.circle_path(project(*center), radius * scale)
            }
            Geometry::Ring(points) => {
                let Some((first, rest)) = points.split_first() else {
                    return;
                };
                let [x, y] = project(*first);
                let _ = writeln!(self.0, "{x:.2} {y:.2} m");
                for point in rest {
                    let [x, y] = project(*point);
                    let _ = writeln!(self.0, "{x:.2} {y:.2} l");
                }
                self.0.push_str("h\n");
            }
        }
        self.0.push_str(if fill.is_some() { "B\n" } else { "S\n" });
    }

    fn ground(
        &mut self,
        rings: &[Vec<Point>],
        dots: &[Point],
        color: Color,
        project: &impl Fn(Point) -> Point,
    ) {
        self.0.push_str("q /GroundFill gs\n");
        self.stroke(0.6, color);
        self.fill(color);
        for ring in rings {
            let Some((&first, rest)) = ring.split_first() else {
                continue;
            };
            let [x, y] = project(first);
            let _ = writeln!(self.0, "{x:.2} {y:.2} m");
            for &point in rest {
                let [x, y] = project(point);
                let _ = writeln!(self.0, "{x:.2} {y:.2} l");
            }
            self.0.push_str("h\n");
        }
        self.0.push_str("B* Q\n");
        for &point in dots {
            self.circle(project(point), 1.0, color);
        }
    }

    fn heading(&mut self, section: &str, title: &str) {
        self.rect([0.0, HEIGHT - 8.0], WIDTH, 8.0, GREEN);
        self.text([MARGIN, 550.0], 11.0, true, GREEN, section);
        let title = fit(title, 58);
        let size = (740.0 / (title.chars().count().max(1) as f64 * 0.93)).clamp(14.0, 26.0);
        self.text([MARGIN, 515.0], size, true, INK, &title);
    }

    fn map_frame(&mut self) {
        self.rect([MARGIN, 73.0], 506.0, 404.0, PANEL);
        self.line([57.0, 429.0], [57.0, 451.0], 0.9, GREEN);
        self.line([52.0, 444.0], [57.0, 451.0], 0.9, GREEN);
        self.line([57.0, 451.0], [62.0, 444.0], 0.9, GREEN);
        self.text([53.5, 458.0], 9.0, true, GREEN, "N");
    }

    fn scale_bar(&mut self, scale: f64) {
        let meters = grid_step(80.0 / scale);
        let width = meters * scale;
        self.line([56.0, 92.0], [56.0 + width, 92.0], 1.2, INK);
        for x in [56.0, 56.0 + width] {
            self.line([x, 89.0], [x, 95.0], 1.2, INK);
        }
        self.text(
            [56.0 + width + 7.0, 89.0],
            9.0,
            false,
            INK,
            &format!("{meters} m"),
        );
    }

    fn key(&mut self, y: f64, label: &str, color: Color) {
        self.circle([576.0, y + 3.0], 3.0, color);
        self.text([587.0, y], 10.0, false, INK, &fit(label, 37));
    }
}

fn grid_step(target: f64) -> f64 {
    let decade = 10.0_f64.powf(target.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|factor| factor * decade)
        .find(|step| *step >= target)
        .unwrap_or(10.0 * decade)
}

fn fit(text: &str, limit: usize) -> String {
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= limit {
        return text;
    }
    text.chars().take(limit - 1).chain(['…']).collect()
}

fn color(species: &Species) -> Color {
    let Some(value) = species
        .color
        .strip_prefix('#')
        .filter(|value| value.len() == 6)
    else {
        return GREEN;
    };
    u32::from_str_radix(value, 16).map_or(GREEN, |value| {
        [16, 8, 0].map(|shift| f64::from((value >> shift) & 255_u32) / 255.0)
    })
}

fn marker(
    page: &mut Page,
    center: Point,
    radius: f64,
    species: &Species,
    code: &str,
    shared: bool,
) {
    page.circle(center, radius, color(species));
    page.outline(center, radius, INK);
    if shared {
        page.outline(center, radius + 2.0, INK);
    }
    let [x, y] = center;
    let location = [x + 4.0, y + 4.0];
    page.rect(
        [location[0] - 1.0, location[1] - 1.5],
        4.4 * code.len() as f64 + 1.5,
        9.0,
        PANEL,
    );
    page.text(location, 7.5, true, INK, code);
}

struct Row<'a> {
    code: String,
    name: &'a str,
    species: &'a Species,
    detail: String,
}

fn inventory(
    pages: &mut Vec<Page>,
    catalog: &Catalog,
    section: &str,
    title: &str,
    detail_heading: &str,
    rows: &[Row<'_>],
) {
    for chunk in rows.chunks(16) {
        let mut page = Page::default();
        page.heading(section, title);
        page.rect([MARGIN, 464.0], WIDTH - MARGIN * 2.0, 27.0, LIGHT);
        for (x, title) in [
            (51.0, catalog.text("gardenPdfColPos")),
            (102.0, catalog.text("gardenPdfColName")),
            (314.0, catalog.text("gardenPdfColBotanical")),
            (511.0, detail_heading.to_string()),
        ] {
            page.text([x, 473.0], 10.0, true, GREEN, &title);
        }
        for (index, row) in chunk.iter().enumerate() {
            let y = 447.0 - 24.0 * index as f64;
            if index % 2 == 0 {
                page.rect([MARGIN, y - 7.0], WIDTH - MARGIN * 2.0, 24.0, PANEL);
            }
            page.circle([54.0, y + 3.0], 3.0, color(row.species));
            page.text([64.0, y], 9.0, true, INK, &row.code);
            page.text([102.0, y], 10.0, true, INK, &fit(row.name, 31));
            page.text(
                [314.0, y],
                10.0,
                false,
                MUTED,
                &fit(&row.species.botanical_name, 30),
            );
            page.text([511.0, y], 10.0, false, INK, &fit(&row.detail, 49));
        }
        pages.push(page);
    }
}

pub fn guild(
    catalog: &Catalog,
    guild: &Guild,
    cover_data: Option<&ground_cover::Data>,
) -> Option<Vec<u8>> {
    let tree = catalog.trees.iter().find(|tree| tree.id == guild.tree_id)?;
    let plants: Vec<_> = guild
        .plant_ids
        .iter()
        .filter_map(|id| catalog.plants.iter().find(|plant| plant.id == *id))
        .collect();
    let metrics = Metrics::new(tree.mature_radius_m);
    let positions = place(tree, &plants, guild.hemisphere == "SOUTHERN");
    let covers = cover_data.map_or_else(Vec::new, |data| {
        ground_cover::radial(data, tree, &positions, guild)
    });
    let extent = positions
        .iter()
        .map(|p| p.distance + p.species.spread_m / 2.0)
        .chain(covers.iter().flat_map(|cover| {
            cover
                .rings
                .iter()
                .flatten()
                .map(|[x, y]| x.abs().max(y.abs()))
        }))
        .fold(metrics.rings[4], f64::max)
        + 0.5;
    let scale = 165.0 / extent;
    let center = [295.0, 275.0];
    let mut map = Page::default();
    map.heading(&catalog.text("pdfTitle"), &tree.name);
    map.map_frame();
    map.scale_bar(scale);
    map.circle(center, metrics.canopy * scale, LIGHT);
    for ring in metrics.rings {
        map.outline(center, ring * scale, GREEN);
    }
    let project = |[x, y]: Point| [center[0] + x * scale, center[1] - y * scale];
    marker(&mut map, center, 5.0, tree, "T1", false);
    map.text(
        [572.0, 459.0],
        13.0,
        true,
        INK,
        &catalog.text("pdfSiteParams"),
    );
    for (i, value) in [
        settings_label(catalog, &SOILS, &guild.soil_type),
        settings_label(catalog, &CLIMATES, &guild.climate_zone),
        catalog.text(if guild.hemisphere == "SOUTHERN" {
            "southHemisphere"
        } else {
            "northHemisphere"
        }),
    ]
    .into_iter()
    .enumerate()
    {
        map.text(
            [572.0, 436.0 - 18.0 * i as f64],
            10.0,
            false,
            MUTED,
            &fit(&value, 37),
        );
    }
    let mut rows = vec![Row {
        code: "T1".into(),
        name: &tree.name,
        species: tree,
        detail: "0 m".into(),
    }];
    for (index, position) in positions.iter().enumerate() {
        let angle = f64::from(position.angle).to_radians();
        let point = [
            center[0] + position.distance * angle.sin() * scale,
            center[1] + position.distance * angle.cos() * scale,
        ];
        let code = format!("P{}", index + 1);
        if let Some(cover) = covers
            .iter()
            .find(|cover| cover.plant_id == position.species.id)
        {
            map.ground(&cover.rings, &cover.dots, color(position.species), &project);
            rows.push(Row {
                code,
                name: &position.species.name,
                species: position.species,
                detail: format!("{} m²", crate::drawing::num(cover.area_m2, 2)),
            });
            continue;
        }
        marker(
            &mut map,
            point,
            (position.species.spread_m * scale / 2.0).max(3.0),
            position.species,
            &code,
            false,
        );
        let zone = catalog.text(match position.zone {
            Zone::Collar => "zoneCollar",
            Zone::Bulb => "zoneBulb",
            Zone::Mid => "zoneMid",
            Zone::Drip => "zoneDrip",
            Zone::Outer => "zoneOuter",
        });
        rows.push(Row {
            code,
            species: position.species,
            name: &position.species.name,
            detail: format!("{} m · {}° · {zone}", crate::drawing::num(position.distance, 2), position.angle),
        });
    }
    map_legend(&mut map, &rows, 343.0, 13);
    let mut pages = vec![map];
    inventory(
        &mut pages,
        catalog,
        &catalog.text("pdfTitle"),
        &catalog.text("pdfPlantInventory"),
        &catalog.text("pdfSpatialGuide"),
        &rows,
    );
    pages.push(seasons(catalog, guild));
    Some(finish(pages, catalog))
}

fn seasons(catalog: &Catalog, guild: &Guild) -> Page {
    let mut page = Page::default();
    page.heading(&catalog.text("pdfTitle"), &catalog.text("calendarTitle"));
    page.text(
        [MARGIN, 489.0],
        10.0,
        false,
        MUTED,
        &catalog.text("pdfEcologicalRoles"),
    );
    page.rect([MARGIN, 444.0], WIDTH - MARGIN * 2.0, 27.0, LIGHT);
    page.text(
        [54.0, 454.0],
        10.0,
        true,
        INK,
        &catalog.text("functionalRole"),
    );
    for (index, season) in Season::ALL.into_iter().enumerate() {
        page.text(
            [242.0 + 108.0 * index as f64, 454.0],
            10.0,
            true,
            INK,
            &catalog.text(season.label()),
        );
    }
    for (index, row) in seasonal::coverage(catalog, guild).iter().enumerate() {
        let y = 422.0 - 33.0 * index as f64;
        if index % 2 == 0 {
            page.rect([MARGIN, y - 13.0], WIDTH - MARGIN * 2.0, 33.0, PANEL);
        }
        page.text(
            [54.0, y],
            10.0,
            true,
            INK,
            &catalog.text(&format!("role:{}", row.role)),
        );
        for (column, plants) in row.across.iter().enumerate() {
            let count = plants.len().to_string();
            page.text(
                [242.0 + 108.0 * column as f64, y],
                10.0,
                true,
                GREEN,
                &count,
            );
        }
    }
    page
}

fn settings_label(catalog: &Catalog, options: &[(&str, &str)], value: &str) -> String {
    options
        .iter()
        .find(|(code, _)| *code == value)
        .map_or_else(|| value.into(), |(_, key)| catalog.text(key))
}

fn map_legend(page: &mut Page, rows: &[Row<'_>], top: f64, lines: usize) {
    for (index, row) in rows.iter().take(lines).enumerate() {
        page.key(
            top - 18.0 * index as f64,
            &format!("{}  {}", row.code, row.name),
            color(row.species),
        );
    }
    if rows.len() > lines {
        page.text(
            [572.0, top - 18.0 * lines as f64],
            10.0,
            false,
            MUTED,
            &format!("+ {}", rows.len() - lines),
        );
    }
}

pub fn garden(
    catalog: &Catalog,
    drawing: &Drawing<'_>,
    settings: &Settings,
    conflicts: &[Conflict],
    shades: &[ShadePocket],
    language: &str,
) -> Vec<u8> {
    let title = catalog.text("gardenPdfDefaultTitle");
    let mut map = Page::default();
    map.heading(
        &title,
        if drawing.name.trim().is_empty() {
            &title
        } else {
            drawing.name
        },
    );
    let values = [
        settings
            .soil
            .as_ref()
            .map(|soil| settings_label(catalog, &SOILS, soil)),
        settings
            .climate
            .as_ref()
            .map(|zone| settings_label(catalog, &CLIMATES, zone)),
        Some(catalog.text(if settings.southern {
            "southHemisphere"
        } else {
            "northHemisphere"
        })),
    ];
    let context = values
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("  ·  ");
    map.text([MARGIN, 490.0], 10.0, false, MUTED, &context);
    map.map_frame();
    let scale = (446.0 / drawing.width.max(1.0)).min(334.0 / drawing.height.max(1.0));
    let [left, bottom] = [
        295.0 - drawing.width * scale / 2.0,
        275.0 - drawing.height * scale / 2.0,
    ];
    let project = |[x, y]: Point| [left + x * scale, bottom + y * scale];
    let step = grid_step(22.0 / scale);
    for index in 0..=(drawing.width / step) as u32 {
        let x = f64::from(index) * step;
        map.line(project([x, 0.0]), project([x, drawing.height]), 0.45, LIGHT);
    }
    for index in 0..=(drawing.height / step) as u32 {
        let y = f64::from(index) * step;
        map.line(project([0.0, y]), project([drawing.width, y]), 0.45, LIGHT);
    }
    map.scale_bar(scale);
    if let Some((outline, _)) = &drawing.outline {
        map.shape(outline, &project, scale, None);
    }
    for (index, bed) in drawing.beds.iter().enumerate() {
        map.shape(&bed.shape, &project, scale, Some(BED));
        let [x, y] = project(bed.shape.center());
        map.text([x, y], 8.0, true, INK, &format!("B{}", index + 1));
    }
    for cover in &drawing.ground {
        map.ground(&cover.rings, &cover.dots, color(cover.species), &project);
    }
    for pocket in shades {
        map.outline(
            project(drawing.project(pocket.point)),
            pocket.radius * scale,
            [0.35, 0.32, 0.60],
        );
    }
    for &(from, to, metres) in &drawing.dimensions {
        let [x, y] = project(from);
        let [tx, ty] = project(to);
        map.line([x, y], [tx, ty], 0.75, MUTED);
        map.text(
            [(x + tx) / 2.0, (y + ty) / 2.0 + 4.0],
            8.0,
            true,
            INK,
            &format!("{} m", crate::drawing::num(metres, 1)),
        );
    }
    let mut rows: Vec<Row<'_>> = Vec::new();
    let mut counts = Vec::new();
    let mut by_code = HashMap::new();
    for instance in drawing.trees.iter().chain(&drawing.plants) {
        let center = project(instance.point);
        let radius = (instance.width * scale / 2.0).max(2.5);
        map.outline(center, radius, color(instance.species));
        marker(
            &mut map,
            center,
            2.5,
            instance.species,
            &instance.code,
            instance.shared,
        );
        let index = *by_code.entry(&instance.code).or_insert_with(|| {
            let index = rows.len();
            counts.push(0);
            rows.push(Row {
                code: instance.code.clone(),
                name: &instance.species.name,
                species: instance.species,
                detail: String::new(),
            });
            index
        });
        counts[index] += 1;
    }
    for (row, count) in rows.iter_mut().zip(counts) {
        row.detail = if row.species.roles.is_empty() {
            count.to_string()
        } else {
            format!("{count}  ·  {}", row.species.roles.join(", "))
        };
    }
    for conflict in conflicts {
        map.line(
            project(drawing.project(conflict.a.point)),
            project(drawing.project(conflict.b.point)),
            0.9,
            warning_color(conflict.severity),
        );
    }
    for cover in &drawing.ground {
        rows.push(Row {
            code: cover.code.clone(),
            name: &cover.species.name,
            species: cover.species,
            detail: format!(
                "{} m²  ·  {}",
                crate::drawing::num(cover.area, 2),
                cover.species.roles.join(", ")
            ),
        });
    }
    map.text(
        [572.0, 459.0],
        13.0,
        true,
        INK,
        &catalog.text("gardenPdfColColor"),
    );
    map_legend(
        &mut map,
        &rows,
        436.0,
        if drawing.beds.is_empty() { 18 } else { 12 },
    );
    if !drawing.beds.is_empty() {
        map.text(
            [572.0, 175.0],
            12.0,
            true,
            INK,
            &catalog.text("infraRaisedBeds"),
        );
        for (index, bed) in drawing.beds.iter().take(4).enumerate() {
            let label = format!(
                "B{}  {} · {} m² · {} m",
                index + 1,
                fit(&bed.name, 18),
                crate::drawing::num(bed.area, 2),
                crate::drawing::num(bed.height, 2)
            );
            map.key(153.0 - 18.0 * index as f64, &label, BED);
        }
        if drawing.beds.len() > 4 {
            map.text(
                [572.0, 81.0],
                10.0,
                false,
                MUTED,
                &format!("+ {}", drawing.beds.len() - 4),
            );
        }
    }
    let mut pages = vec![map];
    let detail_heading = format!(
        "{}  ·  {}",
        catalog.text("gardenPdfColQty"),
        catalog.text("gardenPdfColRoles")
    );
    inventory(
        &mut pages,
        catalog,
        &title,
        &catalog.text("gardenPdfBomTitle"),
        &detail_heading,
        &rows,
    );
    maintenance(&mut pages, catalog, conflicts, shades, language);
    finish(pages, catalog)
}

fn warning_color(severity: Severity) -> Color {
    match severity {
        Severity::Critical => [0.71, 0.16, 0.16],
        Severity::Warning => [0.62, 0.34, 0.12],
        Severity::Info => [0.22, 0.42, 0.55],
    }
}

fn report_lines(text: &str, limit: usize) -> Vec<String> {
    let text = text.replace('↔', " / ");
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut current = String::new();
        let mut width = 0;
        for word in paragraph.split_whitespace() {
            let word = fit(word, limit);
            let length = word.chars().count();
            if width + length >= limit && !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                width = 0;
            }
            if !current.is_empty() {
                current.push(' ');
                width += 1;
            }
            current.push_str(&word);
            width += length;
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    lines
}

fn maintenance(
    pages: &mut Vec<Page>,
    catalog: &Catalog,
    conflicts: &[Conflict],
    shade: &[ShadePocket],
    language: &str,
) {
    let title = catalog.text("gardenPdfHarmonyTitle");
    let section = catalog.text("gardenPdfDefaultTitle");
    let mut page = Page::default();
    page.heading(&section, &title);
    let mut y = 472.0;
    let mut write = |heading: &str, text: &str, color: Color| {
        let title_lines = report_lines(heading, 104);
        let body_lines = report_lines(text, 106);
        let height = 15.0 * (title_lines.len() + body_lines.len()) as f64;
        if y - height < 75.0 {
            pages.push(std::mem::take(&mut page));
            page.heading(&section, &title);
            y = 472.0;
        }
        for line in title_lines {
            page.text([MARGIN, y], 11.0, true, color, &line);
            y -= 15.0;
        }
        for line in body_lines {
            page.text([MARGIN, y], 10.0, false, MUTED, &line);
            y -= 15.0;
        }
        y -= 12.0;
    };
    if conflicts.is_empty() {
        write(
            &catalog.text("gardenWarningsAllHarmony"),
            &catalog.text("gardenWarningsNoConflictsDesc"),
            GREEN,
        );
    }
    for conflict in conflicts {
        let (title, description) = conflict.text(catalog, language);
        write(
            &format!("{} · {title}", conflict.severity.label(language)),
            &description,
            warning_color(conflict.severity),
        );
    }
    let light = if shade.is_empty() {
        catalog.text("gardenWarningsNoShade")
    } else {
        let count = catalog
            .text("gardenWarningsShadePockets")
            .replace("{count}", &shade.len().to_string());
        let suggestions = SHADE_PLANTS
            .iter()
            .filter_map(|id| catalog.plants.iter().find(|plant| plant.id == *id))
            .map(|plant| plant.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!("{count}. {suggestions}.")
    };
    write(&catalog.text("gardenPdfShadingTitle"), &light, GREEN);
    write(
        &catalog.text("gardenPdfChopTitle"),
        &catalog.text("gardenPdfChopText"),
        GREEN,
    );
    pages.push(page);
}

fn finish(mut pages: Vec<Page>, catalog: &Catalog) -> Vec<u8> {
    use std::io::Write as _;

    // Standard Type 1 fonts are supplied by PDF viewers. Text is hex-encoded
    // before it enters a stream so names can't affect the PDF syntax.
    let count = pages.len();
    let mut pdf = b"%PDF-1.4\n%\xC7\xEC\x8F\xA2\n".to_vec();
    let mut offsets = Vec::with_capacity(4 + count * 2);
    let mut object = |pdf: &mut Vec<u8>, contents: &str| {
        offsets.push(pdf.len());
        let _ = writeln!(pdf, "{} 0 obj\n{contents}\nendobj", offsets.len());
    };
    object(&mut pdf, "<< /Type /Catalog /Pages 2 0 R >>");
    let kids: String = (0..count).map(|i| format!("{} 0 R ", 5 + i * 2)).collect();
    object(
        &mut pdf,
        &format!("<< /Type /Pages /Count {count} /Kids [{kids}] >>"),
    );
    for font in ["Helvetica", "Helvetica-Bold"] {
        object(
            &mut pdf,
            &format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /{font} /Encoding /WinAnsiEncoding >>"
            ),
        );
    }
    for (index, page) in pages.iter_mut().enumerate() {
        let footer = catalog
            .text("gardenPdfPageOf")
            .replace("{page}", &(index + 1).to_string())
            .replace("{total}", &count.to_string());
        page.line([MARGIN, 56.0], [WIDTH - MARGIN, 56.0], 0.7, LIGHT);
        page.text([MARGIN, 40.0], 9.0, false, MUTED, &footer);
        let stream = 6 + index * 2;
        object(&mut pdf, &format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {WIDTH} {HEIGHT}] /Resources << /Font << /Regular 3 0 R /Bold 4 0 R >> /ExtGState << /GroundFill << /Type /ExtGState /ca 0.27 >> >> >> /Contents {stream} 0 R >>"));
        object(
            &mut pdf,
            &format!(
                "<< /Length {} >>\nstream\n{}endstream",
                page.0.len(),
                page.0
            ),
        );
    }
    let xref = pdf.len();
    let size = offsets.len() + 1;
    let _ = writeln!(pdf, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = writeln!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF"
    );
    pdf
}
