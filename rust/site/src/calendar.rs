use crate::{
    catalog::{Catalog, Species},
    garden::{Garden, Settings},
    garden_layout::Companion,
    guild::Guild,
};
use serde::Deserialize;
use std::collections::HashMap;

const ORIGIN: &str = "https://pflanzengilde.de";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Season {
    EarlySpring,
    LateSpring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub const ALL: [Self; 5] = [
        Self::EarlySpring,
        Self::LateSpring,
        Self::Summer,
        Self::Autumn,
        Self::Winter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::EarlySpring => "seasonEarlySpring",
            Self::LateSpring => "seasonLateSpring",
            Self::Summer => "seasonSummer",
            Self::Autumn => "seasonAutumn",
            Self::Winter => "seasonWinter",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::EarlySpring => "early_spring",
            Self::LateSpring => "late_spring",
            Self::Summer => "summer",
            Self::Autumn => "autumn",
            Self::Winter => "winter",
        }
    }
}

#[derive(Default, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Activity {
    pub flowering: Vec<Season>,
    pub active: Vec<Season>,
    pub foliage: Vec<Season>,
    pub pest_deterrence: Vec<Season>,
    pub planting: Vec<Season>,
    pub chop: Vec<Season>,
    pub harvest: Vec<Season>,
    pub planting_time: String,
    pub harvest_time: String,
    pub soil_notes: String,
    pub perennial: bool,
    pub autumn_planting: bool,
    pub tree_category: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chop {
    how_to_cut: String,
    how_much: String,
    where_to_spread: String,
    nutrient_benefit: String,
}

#[derive(Deserialize)]
struct MonthDay {
    month: u8,
    day: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    labels: HashMap<String, String>,
    chop: HashMap<String, Chop>,
    anchors: HashMap<String, String>,
    overrides: HashMap<String, HashMap<Season, MonthDay>>,
}

impl Data {
    fn tr(&self, key: &str) -> &str {
        self.labels.get(key).map(String::as_str).unwrap_or_default()
    }

    fn message(&self, key: &str, values: &[(&str, &str)]) -> String {
        values
            .iter()
            .fold(self.tr(key).to_owned(), |text, (token, value)| {
                text.replace(&format!("{{{token}}}"), value)
            })
    }

    fn chop_guide(&self, id: &str, fallback: &str) -> String {
        let anchor = self.anchors.get(id).map(String::as_str).unwrap_or(fallback);
        format!(
            "{ORIGIN}/guides{anchor}",
            anchor = if anchor.is_empty() {
                String::new()
            } else {
                format!("#{anchor}")
            }
        )
    }

    fn chop_description(
        &self,
        plant: &Species,
        guide: &str,
        require_dedicated: bool,
    ) -> Option<String> {
        let info = self.chop.get(&plant.id);
        if require_dedicated && info.is_none() {
            return None;
        }
        Some(self.message(
            "calendarGardenChopDesc",
            &[
                (
                    "howToCut",
                    info.map_or(self.tr("calendarChopDefaultHowToCut"), |c| &c.how_to_cut),
                ),
                (
                    "howMuch",
                    info.map_or(self.tr("calendarChopDefaultHowMuch"), |c| &c.how_much),
                ),
                (
                    "where",
                    info.map_or(self.tr("calendarChopDefaultWhereToSpread"), |c| {
                        &c.where_to_spread
                    }),
                ),
                (
                    "benefit",
                    info.map_or(self.tr("calendarChopDefaultNutrientBenefit"), |c| {
                        &c.nutrient_benefit
                    }),
                ),
                ("guide", guide),
            ],
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: u32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    fn ics(self) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day)
    }

    fn tomorrow(self) -> Self {
        let days = match self.month {
            2 if self.year % 4 == 0 && (self.year % 100 != 0 || self.year % 400 == 0) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        if self.day < days {
            Self {
                day: self.day + 1,
                ..self
            }
        } else if self.month < 12 {
            Self {
                month: self.month + 1,
                day: 1,
                ..self
            }
        } else {
            Self {
                year: self.year + 1,
                month: 1,
                day: 1,
            }
        }
    }
}

enum Chore {
    Planting,
    Chop,
    Harvest,
    Prune,
    Collar,
}

fn shift(month: u8, southern: bool) -> u8 {
    if southern {
        (month + 5) % 12 + 1
    } else {
        month
    }
}

fn next_date(
    min: Date,
    season: Season,
    task: Chore,
    south: bool,
    named: Option<&MonthDay>,
) -> Date {
    use Chore::*;
    use Season::*;
    let (month, day) = named
        .map(|date| (date.month, date.day))
        .unwrap_or(match (season, task) {
            (EarlySpring, Prune) => (3, 1),
            (EarlySpring, Planting) => (4, 5),
            (EarlySpring, Harvest) => (4, 15),
            (EarlySpring, _) => (4, 20),
            (LateSpring, Planting) => (5, 15),
            (LateSpring, Chop) => (5, 25),
            (LateSpring, Harvest) => (5, 20),
            (LateSpring, _) => (5, 18),
            (Summer, Planting) => (6, 15),
            (Summer, Chop) => (7, 15),
            (Summer, Harvest) => (7, 25),
            (Summer, _) => (8, 1),
            (Autumn, Planting) => (10, 15),
            (Autumn, Chop) => (10, 10),
            (Autumn, Harvest) => (9, 25),
            (Autumn, _) => (10, 5),
            (Winter, Prune) => (2, 15),
            (Winter, Collar) => (11, 20),
            (Winter, Chop) => (2, 20),
            (Winter, _) => (12, 1),
        });
    let mut date = Date {
        year: min.year,
        month: shift(month, south),
        day,
    };
    if date < min {
        date.year += 1;
    }
    date
}

struct Event {
    uid: String,
    date: Date,
    summary: String,
    description: String,
    html: Option<String>,
    categories: Vec<String>,
    recurring: bool,
    link: Option<String>,
    image: Option<String>,
}

fn text_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace("\r\n", "\\n")
        .replace(['\r', '\n'], "\\n")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn fold(ics: &mut Vec<String>, line: String) {
    let mut row = String::new();
    for ch in line.chars() {
        if row.len() + ch.len_utf8() > 75 {
            ics.push(std::mem::take(&mut row));
            row.push(' ');
        }
        row.push(ch);
    }
    ics.push(row);
}

fn serialize(mut events: Vec<Event>, name: &str, description: Option<&str>, stamp: &str) -> String {
    let garden = description.is_none();
    let product = if garden {
        "-//Pflanzengilde//Permakultur-Garten-Planer//DE"
    } else {
        "-//Pflanzengilde//Permakultur Kalender 1.0//DE"
    };
    let mut lines = ["BEGIN:VCALENDAR", "VERSION:2.0"]
        .map(str::to_owned)
        .to_vec();
    lines.extend([
        format!("PRODID:{product}"),
        "CALSCALE:GREGORIAN".into(),
        "METHOD:PUBLISH".into(),
    ]);
    fold(&mut lines, format!("X-WR-CALNAME:{}", text_escape(name)));
    if let Some(description) = description {
        fold(
            &mut lines,
            format!("X-WR-CALDESC:{}", text_escape(description)),
        );
    }
    lines.push(format!(
        "X-WR-TIMEZONE:{}",
        if garden { "Europe/Berlin" } else { "UTC" }
    ));
    events.sort_by_key(|event| event.date);
    for event in events {
        lines.push("BEGIN:VEVENT".into());
        fold(&mut lines, format!("UID:{}", event.uid));
        lines.push(format!("DTSTAMP:{stamp}"));
        lines.push(format!("DTSTART;VALUE=DATE:{}", event.date.ics()));
        lines.push(format!("DTEND;VALUE=DATE:{}", event.date.tomorrow().ics()));
        fold(
            &mut lines,
            format!("SUMMARY:{}", text_escape(&event.summary)),
        );
        fold(
            &mut lines,
            format!("DESCRIPTION:{}", text_escape(&event.description)),
        );
        if let Some(html) = event.html {
            fold(
                &mut lines,
                format!("X-ALT-DESC;FMTTYPE=text/html:{}", text_escape(&html)),
            );
        }
        if let Some(image) = event.image {
            let path = image.split('?').next().unwrap_or(&image);
            let mime = if path.ends_with(".svg") {
                "image/svg+xml"
            } else if path.ends_with(".webp") {
                "image/webp"
            } else if path.ends_with(".png") {
                "image/png"
            } else {
                "image/jpeg"
            };
            fold(&mut lines, format!("ATTACH;FMTTYPE={mime}:{image}"));
            fold(
                &mut lines,
                format!("ATTACH;FMTTYPE=image/svg+xml:{ORIGIN}/favicon.svg"),
            );
        }
        if let Some(link) = event.link {
            fold(&mut lines, format!("URL:{link}"));
        }
        let categories = event
            .categories
            .iter()
            .map(|c| text_escape(c))
            .collect::<Vec<_>>()
            .join(",");
        if !garden && !categories.is_empty() {
            fold(&mut lines, format!("CATEGORIES:{categories}"));
        }
        if event.recurring {
            lines.push("RRULE:FREQ=YEARLY".into());
        }
        if garden {
            lines.push(format!("CATEGORIES:{categories}"));
        } else {
            lines.extend(["STATUS:CONFIRMED".into(), "TRANSP:TRANSPARENT".into()]);
        }
        lines.push("END:VEVENT".into());
    }
    lines.push("END:VCALENDAR".into());
    lines.join("\r\n")
}

fn unique_counts<'a>(ids: impl Iterator<Item = &'a str>) -> Vec<(&'a str, usize)> {
    let mut counts = Vec::<(&str, usize)>::new();
    let mut indexed = HashMap::new();
    for id in ids {
        let i = *indexed.entry(id).or_insert_with(|| {
            counts.push((id, 0));
            counts.len() - 1
        });
        counts[i].1 += 1;
    }
    counts
}

pub fn stable_hash(input: &str) -> String {
    format!(
        "{:08x}",
        input.encode_utf16().fold(0x811c9dc5u32, |hash, unit| {
            (hash ^ u32::from(unit)).wrapping_mul(0x01000193)
        })
    )
}

pub fn garden_ics(
    catalog: &Catalog,
    data: &Data,
    garden: &Garden,
    settings: &Settings,
    placed: &[Companion],
    year: u32,
) -> String {
    let mut explicit = String::new();
    for ch in garden.garden_id.to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            explicit.push(ch);
        } else if !explicit.ends_with('-') {
            explicit.push('-');
        }
    }
    let key = if explicit.trim_matches('-').is_empty() {
        let mut ids: Vec<_> = garden
            .star_plants
            .iter()
            .map(|star| star.instance_id.as_str())
            .collect();
        ids.sort_unstable();
        stable_hash(&format!("{}|{}", garden.garden_name, ids.join(",")))
    } else {
        explicit.trim_matches('-').to_owned()
    };
    let mut events = Vec::new();
    for (id, count) in unique_counts(garden.star_plants.iter().map(|tree| tree.tree_id.as_str())) {
        let Some(tree) = catalog.trees.iter().find(|tree| tree.id == id) else {
            continue;
        };
        let autumn = matches!(
            tree.activity.tree_category.as_str(),
            "FRUIT_TREE" | "NUT_TREE" | "NITROGEN_FIXING_TREE" | "BERRY_SHRUB"
        );
        let month = shift(if autumn { 10 } else { 4 }, settings.southern);
        let title = data.message(
            "calendarGardenTreePlantingTitle",
            &[("tree", &tree.name), ("count", &count.to_string())],
        );
        let exposure = data.tr(if tree.sun_preference == "FULL_SUN" {
            "calendarGardenExposureFullSun"
        } else {
            "calendarGardenExposureOther"
        });
        let description = data.message(
            "calendarGardenTreePlantingDesc",
            &[
                ("tree", &tree.name),
                ("botanical", &tree.botanical_name),
                ("exposure", exposure),
                ("radius", &tree.mature_radius_m.to_string()),
                ("origin", ORIGIN),
            ],
        );
        let html = format!("<!DOCTYPE html><html><body><h2>{}</h2><p>{}</p><p><a href=\"{ORIGIN}/garten\">Pflanzengilde.de Garten-Planer</a></p></body></html>",
            html_escape(&title), html_escape(&tree.description));
        events.push(Event {
            uid: format!("garden-{key}-tree-{id}-planting@pflanzengilde.de"),
            date: Date {
                year,
                month,
                day: 15,
            },
            summary: title,
            description,
            html: Some(html),
            categories: ["PFLANZEN", "LEITPFLANZE", "PERMAKULTUR"]
                .map(str::to_owned)
                .into(),
            recurring: false,
            image: None,
            link: None,
        });
    }
    for (id, count) in unique_counts(placed.iter().map(|plant| plant.plant_id.as_str())) {
        let Some(plant) = catalog.plants.iter().find(|plant| plant.id == id) else {
            continue;
        };
        let perennial = plant.activity.perennial;
        let action = data.tr(if perennial {
            "calendarPlanting"
        } else {
            "calendarGardenSowing"
        });
        events.push(Event {
            uid: format!("garden-{key}-comp-{id}-sow@pflanzengilde.de"),
            date: Date {
                year,
                month: shift(4, settings.southern),
                day: 20,
            },
            summary: format!("🌿 {action}: {} ({count}x)", plant.name),
            description: data.message(
                "calendarGardenSowDesc",
                &[
                    ("plant", &plant.name),
                    ("botanical", &plant.botanical_name),
                    ("roles", &plant.role_ids.join(", ")),
                    ("origin", ORIGIN),
                ],
            ),
            html: None,
            categories: [
                "BEGLEITPFLANZE",
                if perennial {
                    "MEHRJAEHRIG"
                } else {
                    "EINJAEHRIG"
                },
            ]
            .map(str::to_owned)
            .into(),
            recurring: !perennial,
            image: None,
            link: None,
        });
        let guide = data.chop_guide(id, "");
        let Some(chop) = data.chop_description(plant, &guide, true) else {
            continue;
        };
        let mut months = Vec::new();
        for season in &plant.activity.chop {
            let month = data
                .overrides
                .get(id)
                .and_then(|dates| dates.get(season))
                .map_or_else(
                    || match season {
                        Season::EarlySpring => 3,
                        Season::LateSpring => 5,
                        Season::Summer => 7,
                        Season::Autumn => 9,
                        Season::Winter => 2,
                    },
                    |date| date.month,
                );
            let month = shift(month, settings.southern);
            if !months.contains(&month) {
                months.push(month);
            }
        }
        for (idx, &month) in months.iter().enumerate() {
            events.push(Event {
                uid: format!("garden-{key}-chop-{id}-m{month:02}@pflanzengilde.de"),
                date: Date {
                    year,
                    month,
                    day: 15,
                },
                summary: data.message(
                    "calendarGardenChopTitle",
                    &[
                        ("plant", &plant.name),
                        ("index", &(idx + 1).to_string()),
                        ("total", &months.len().to_string()),
                    ],
                ),
                description: chop.clone(),
                html: None,
                categories: ["CHOP_DROP", "MULCH", "PFLEGE"].map(str::to_owned).into(),
                recurring: true,
                image: None,
                link: None,
            });
        }
    }
    let name = if garden.garden_name.is_empty() {
        data.tr("calendarGardenDefaultName")
    } else {
        &garden.garden_name
    };
    serialize(
        events,
        &format!("{name} • Pflanzengilde.de"),
        None,
        &format!("{year}0101T000000Z"),
    )
}

fn detail(data: &Data, fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| format!("{}: {value}", data.tr(key)))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn stock_details(data: &Data, fields: &[(&str, &str)]) -> String {
    detail(
        data,
        &fields
            .iter()
            .map(|(label, value)| (*label, data.tr(value)))
            .collect::<Vec<_>>(),
    )
}

struct GuildEvent<'a> {
    species: &'a Species,
    uid: String,
    date: Date,
    summary: String,
    instructions: String,
    category: &'a str,
    recurring: bool,
    link: &'a str,
}

impl GuildEvent<'_> {
    fn event(self) -> Event {
        let image = match self.species.image_url.as_str() {
            "" => format!("{ORIGIN}/favicon.svg"),
            url if url.starts_with("http") => url.to_owned(),
            url => format!("{ORIGIN}{url}"),
        };
        let description = format!(
            "{} ({})\n\n{}\n\n{}",
            self.species.name, self.species.botanical_name, self.instructions, self.link
        );
        let html = format!(
            "<!DOCTYPE html><html><body><h2>{}</h2><p><em>{}</em></p><p>{}</p><p><a href=\"{}\">Pflanzengilde.de</a></p></body></html>",
            html_escape(&self.summary), html_escape(&self.species.botanical_name),
            html_escape(&self.instructions).replace('\n', "<br>"), html_escape(self.link),
        );
        Event {
            uid: format!("{}@pflanzengilde.de", self.uid),
            date: self.date,
            summary: self.summary,
            description,
            html: Some(html),
            categories: vec!["Pflanzengilde".into(), self.category.into()],
            recurring: self.recurring,
            link: Some(self.link.into()),
            image: Some(image),
        }
    }
}

pub fn guild_ics(
    catalog: &Catalog,
    data: &Data,
    guild: &Guild,
    language: &str,
    today: Date,
    stamp: &str,
) -> Option<String> {
    use Season::*;
    let tree = catalog.trees.iter().find(|tree| tree.id == guild.tree_id)?;
    let share = format!(
        "{ORIGIN}/share/{}/?g={}",
        tree.id,
        guild.encode(catalog, language)
    );
    let south = guild.hemisphere == "SOUTHERN";
    let planting = next_date(
        today,
        if tree.activity.autumn_planting {
            Autumn
        } else {
            EarlySpring
        },
        Chore::Planting,
        south,
        None,
    );
    let tree_event = |suffix: &str, key: &str, date, instructions, category, recurring| {
        GuildEvent {
            species: tree,
            uid: format!("{}-{suffix}", tree.id),
            date,
            instructions,
            category,
            recurring,
            link: &share,
            summary: data.message(
                key,
                &[("tree", &tree.name), ("botanical", &tree.botanical_name)],
            ),
        }
        .event()
    };
    let mut plant_text = stock_details(
        data,
        &[
            ("calendarTreePlantHoleLabel", "calendarTreePlantHoleText"),
            ("calendarTreePlantDepthLabel", "calendarTreePlantDepthText"),
            (
                "calendarTreeTrunkCollarLabel",
                "calendarTreeTrunkCollarText",
            ),
        ],
    );
    plant_text.push_str(&format!(
        "\n\n{}",
        detail(data, &[("calendarSoilGuidance", &tree.activity.soil_notes)])
    ));
    let mut events = vec![
        tree_event(
            "planting",
            "calendarTreePlantTitle",
            planting,
            plant_text,
            data.tr("calendarPlanting"),
            false,
        ),
        tree_event(
            "pruning-winter",
            "calendarTreePruneTitle",
            next_date(planting, Winter, Chore::Prune, south, None),
            stock_details(
                data,
                &[
                    ("calendarPruneLightLabel", "calendarPruneLightText"),
                    ("calendarPruneDeadwoodLabel", "calendarPruneDeadwoodText"),
                    ("calendarPruneSproutsLabel", "calendarPruneSproutsText"),
                    ("calendarPruneWoodLabel", "calendarPruneWoodText"),
                ],
            ),
            data.tr("calendarCatPruning"),
            true,
        ),
        tree_event(
            "harvest",
            "calendarTreeHarvestTitle",
            next_date(
                planting,
                tree.activity.harvest.first().copied().unwrap_or(Autumn),
                Chore::Harvest,
                south,
                None,
            ),
            format!(
                "{}\n\n{}",
                tree.activity.harvest_time,
                stock_details(
                    data,
                    &[
                        (
                            "calendarTreeHarvestTiltLabel",
                            "calendarTreeHarvestTiltText"
                        ),
                        (
                            "calendarTreeHarvestWindfallLabel",
                            "calendarTreeHarvestWindfallText"
                        ),
                        (
                            "calendarTreeHarvestStorageLabel",
                            "calendarTreeHarvestStorageText"
                        ),
                    ]
                )
            ),
            data.tr("calendarCatHarvest"),
            true,
        ),
        tree_event(
            "collar-care",
            "calendarTreeCollarTitle",
            next_date(planting, Winter, Chore::Collar, south, None),
            stock_details(
                data,
                &[
                    ("calendarCollarClearLabel", "calendarCollarClearText"),
                    ("calendarCollarBarkLabel", "calendarCollarBarkText"),
                    ("calendarCollarFrostLabel", "calendarCollarFrostText"),
                ],
            ),
            data.tr("calendarCatTreeCare"),
            true,
        ),
    ];
    for id in &guild.plant_ids {
        let Some(plant) = catalog.plants.iter().find(|plant| &plant.id == id) else {
            continue;
        };
        let first = next_date(
            planting,
            plant
                .activity
                .planting
                .first()
                .copied()
                .unwrap_or(LateSpring),
            Chore::Planting,
            south,
            None,
        );
        let recurring = !plant.activity.perennial;
        let action = data.tr(if recurring {
            "calendarActionSowingPlanting"
        } else {
            "calendarActionPlanting"
        });
        let range = format!("{}–{} m", plant.min_distance_m, plant.max_distance_m);
        let roles = plant.roles.join(", ");
        let season = if plant.activity.planting_time.is_empty() {
            data.tr("calendarSpringOrAutumn")
        } else {
            &plant.activity.planting_time
        };
        let uid = format!("{}.{id}", tree.id);
        events.push(
            GuildEvent {
                species: plant,
                uid: format!("{uid}-planting"),
                date: first,
                summary: format!("{action} {} ({})", plant.name, plant.botanical_name),
                instructions: detail(
                    data,
                    &[
                        ("calendarPlantTimingLabel", season),
                        ("calendarDistanceToTrunk", &range),
                        ("calendarSoilNote", &plant.activity.soil_notes),
                        ("calendarGuildRoles", &roles),
                        ("calendarPermacultureNotes", &plant.description),
                    ],
                ),
                category: data.tr("calendarPlanting"),
                recurring,
                link: &share,
            }
            .event(),
        );
        for &season in &plant.activity.chop {
            let guide = data.chop_guide(id, "chop_and_drop");
            let title = data.message(
                "calendarChopTitle",
                &[
                    ("plant", &plant.name),
                    (
                        "season",
                        data.tr(match season {
                            LateSpring => "calendarChopSeasonLateSpring",
                            Summer => "calendarChopSeasonSummer",
                            Autumn => "calendarChopSeasonAutumn",
                            _ => season.label(),
                        }),
                    ),
                ],
            );
            events.push(
                GuildEvent {
                    species: plant,
                    uid: format!("{uid}-chop-{}", season.slug()),
                    date: next_date(
                        first,
                        season,
                        Chore::Chop,
                        south,
                        data.overrides.get(id).and_then(|dates| dates.get(&season)),
                    ),
                    summary: title,
                    instructions: data
                        .chop_description(plant, &guide, false)
                        .unwrap_or_default(),
                    category: "Chop & Drop",
                    recurring: true,
                    link: &guide,
                }
                .event(),
            );
        }
        if let Some(&season) = plant.activity.harvest.first().filter(|_| {
            plant
                .role_ids
                .iter()
                .any(|role| role == "EDIBLE_UNDERSTORY")
        }) {
            events.push(
                GuildEvent {
                    species: plant,
                    uid: format!("{uid}-harvest"),
                    date: next_date(first, season, Chore::Harvest, south, None),
                    summary: data.message(
                        "calendarPlantHarvestTitle",
                        &[("plant", &plant.name), ("botanical", &plant.botanical_name)],
                    ),
                    instructions: format!(
                        "{}\n\n{}",
                        plant.activity.harvest_time,
                        stock_details(
                            data,
                            &[
                                (
                                    "calendarPlantHarvestTimingLabel",
                                    "calendarPlantHarvestTimingText"
                                ),
                                (
                                    "calendarPlantHarvestQuantityLabel",
                                    "calendarPlantHarvestQuantityText"
                                ),
                                (
                                    "calendarPlantHarvestUsageLabel",
                                    "calendarPlantHarvestUsageText"
                                ),
                            ]
                        )
                    ),
                    category: data.tr("calendarCatHarvest"),
                    recurring: true,
                    link: &share,
                }
                .event(),
            );
        }
    }
    let name = data.message("calendarIcsName", &[("tree", &tree.name)]);
    let desc = data.message("calendarIcsDesc", &[("tree", &tree.name)]);
    Some(serialize(events, &name, Some(&desc), stamp))
}
