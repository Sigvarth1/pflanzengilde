use crate::round_cm;

/// Scale first to match the browser's two-coordinate distance at shape boundaries.
fn distance(x: f64, y: f64) -> f64 {
    let scale = x.abs().max(y.abs());
    if scale == 0.0 || !scale.is_finite() {
        return scale;
    }
    let (u, v) = (x / scale, y / scale);
    scale * (u * u + v * v).sqrt()
}

pub enum Outline<'a> {
    Circle {
        center: [f64; 2],
        radius: f64,
    },
    Polygon {
        center: [f64; 2],
        vertices: &'a [f64],
        rectangle: bool,
    },
}

impl<'a> Outline<'a> {
    pub fn from_buffer(data: &'a [f64]) -> Option<Self> {
        match data {
            [1.0, x, y, radius] => Some(Self::Circle {
                center: [*x, *y],
                radius: *radius,
            }),
            [kind @ (2.0 | 3.0), x, y, vertices @ ..]
                if vertices.len() >= 6 && vertices.len() % 2 == 0 =>
            {
                Some(Self::Polygon {
                    center: [*x, *y],
                    vertices,
                    rectangle: *kind == 2.0,
                })
            }
            _ => None,
        }
    }

    /// Signed distance and the nearest point on the boundary.
    pub fn border(&self, x: f64, y: f64) -> (f64, f64, f64) {
        match self {
            Self::Circle {
                center: [cx, cy],
                radius,
            } => {
                let (dx, dy) = (x - cx, y - cy);
                let dist = distance(dx, dy);
                let (ux, uy) = if dist > 1e-9 {
                    (dx / dist, dy / dist)
                } else {
                    (1.0, 0.0)
                };
                (dist - radius, cx + ux * radius, cy + uy * radius)
            }
            Self::Polygon {
                vertices,
                rectangle,
                ..
            } => {
                let mut nearest = (f64::INFINITY, x, y);
                let mut inside = false;
                for i in (0..vertices.len()).step_by(2) {
                    let j = (i + 2) % vertices.len();
                    let (ax, ay) = (vertices[i], vertices[i + 1]);
                    let (bx, by) = (vertices[j], vertices[j + 1]);
                    let (dx, dy) = (bx - ax, by - ay);
                    let len_sq = dx * dx + dy * dy;
                    let t = if len_sq > 0.0 {
                        (((x - ax) * dx + (y - ay) * dy) / len_sq).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let (px, py) = (ax + t * dx, ay + t * dy);
                    let dist = distance(x - px, y - py);
                    if dist < nearest.0 {
                        nearest = (dist, px, py);
                    }
                    if (ay > y) != (by > y) && x < dx * (y - ay) / dy + ax {
                        inside = !inside;
                    }
                }
                if *rectangle {
                    inside = x >= vertices[0]
                        && x <= vertices[2]
                        && y >= vertices[1]
                        && y <= vertices[5];
                }
                if inside {
                    nearest.0 = -nearest.0;
                }
                nearest
            }
        }
    }

    pub fn center(&self) -> [f64; 2] {
        match self {
            Self::Circle { center, .. } | Self::Polygon { center, .. } => *center,
        }
    }

    pub fn nearest_inside(&self, x: f64, y: f64, inset: f64) -> [f64; 2] {
        let (distance, px, py) = self.border(x, y);
        if distance <= -inset {
            return [x, y];
        }
        let [cx, cy] = self.center();
        let mut t = 0.0;
        while t <= 1.0001 {
            let (x, y) = (px + (cx - px) * t, py + (cy - py) * t);
            if self.border(x, y).0 <= -inset {
                return [x, y];
            }
            t += 0.05;
        }
        [cx, cy]
    }

    pub fn nearest_outside(&self, x: f64, y: f64, gap: f64) -> [f64; 2] {
        let (signed, px, py) = self.border(x, y);
        if signed >= gap {
            return [x, y];
        }
        let [cx, cy] = self.center();
        let (dx, dy) = (px - cx, py - cy);
        let magnitude = distance(dx, dy);
        let length = if magnitude > 0.0 { magnitude } else { 1.0 };
        let (ux, uy) = (dx / length, dy / length);
        for step in 0..200 {
            let offset = gap + f64::from(step) * 0.05;
            let (x, y) = (px + ux * offset, py + uy * offset);
            if self.border(x, y).0 >= gap {
                return [x, y];
            }
        }
        [px + ux * gap, py + uy * gap]
    }

    pub fn constrain(&self, plant: &mut [f64]) {
        let inset = (plant[2] / 2.0).clamp(0.15, 0.5);
        if self.border(plant[0], plant[1]).0 <= -inset + 0.01 {
            return;
        }
        let [x, y] = self.nearest_inside(plant[0], plant[1], inset + 0.01);
        plant[0] = round_cm(x);
        plant[1] = round_cm(y);
    }
}
