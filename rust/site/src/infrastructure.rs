use crate::garden::Shape;

pub type Point = [f64; 2];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Rectangle,
    Circle,
    Free,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Target {
    Outline,
    NewBed,
    Bed(String),
}

#[derive(Clone, PartialEq, Eq)]
pub struct Tool {
    pub target: Target,
    pub form: Form,
}

#[derive(Clone)]
pub struct Gesture {
    pub tool: Tool,
    pub points: Vec<Point>,
    pub shift: bool,
}

impl Gesture {
    pub fn sample(&mut self, point: Point, shift: bool) {
        self.shift = shift;
        if self.tool.form != Form::Free {
            if self.points.len() == 1 {
                self.points.push(point);
            } else {
                self.points[1] = point;
            }
            return;
        }
        let previous = self.points[self.points.len() - 1];
        if (point[0] - previous[0]).hypot(point[1] - previous[1]) >= 0.03
            && self.points.len() < 4096
        {
            self.points.push(point);
        }
    }

    pub fn shape(&self, finalise: bool) -> Option<Shape> {
        let [x, y] = self.points[0];
        let [end_x, end_y] = *self.points.last()?;
        match self.tool.form {
            Form::Rectangle => {
                let (x, y, mut end_x, mut end_y) = (snap(x), snap(y), snap(end_x), snap(end_y));
                if self.shift {
                    let (dx, dy) = (end_x - x, end_y - y);
                    let max_x = if dx.is_sign_negative() {
                        x + 300.0
                    } else {
                        300.0 - x
                    };
                    let max_y = if dy.is_sign_negative() {
                        y + 300.0
                    } else {
                        300.0 - y
                    };
                    let length = dx.abs().max(dy.abs()).min(max_x).min(max_y);
                    end_x = x + length.copysign(dx);
                    end_y = y + length.copysign(dy);
                }
                let (w_m, h_m) = ((end_x - x).abs(), (end_y - y).abs());
                if w_m < 0.25 || h_m < 0.25 || w_m > 300.0 || h_m > 300.0 {
                    return None;
                }
                Some(Shape::Rect {
                    x_m: x.min(end_x),
                    y_m: y.min(end_y),
                    w_m,
                    h_m,
                })
            }
            Form::Circle => {
                let (x, y) = (snap(x), snap(y));
                let radius = snap((end_x - x).hypot(end_y - y));
                (radius >= 0.25 && radius <= 300.0).then_some(Shape::Circle {
                    cx_m: x,
                    cy_m: y,
                    r_m: radius,
                })
            }
            Form::Free => {
                let points = if finalise {
                    simplify(&self.points)
                } else {
                    self.points.clone()
                };
                if points.len() < 3 || (finalise && !valid_polygon(&points)) {
                    return None;
                }
                Some(Shape::Polygon { points })
            }
        }
    }
}

fn snap(value: f64) -> f64 {
    (value * 4.0 + 0.5).floor() / 4.0
}

fn segment_distance([x, y]: Point, [ax, ay]: Point, [bx, by]: Point) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (((x - ax) * dx + (y - ay) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (x - (ax + t * dx)).hypot(y - (ay + t * dy))
}

fn retain_segment(points: &[Point], keep: &mut [bool], start: usize, end: usize, tolerance: f64) {
    let mut pending = vec![(start, end)];
    while let Some((start, end)) = pending.pop() {
        let mut farthest = (tolerance, start);
        for i in start + 1..end {
            let distance = segment_distance(points[i], points[start], points[end]);
            if distance > farthest.0 {
                farthest = (distance, i);
            }
        }
        if farthest.1 != start {
            keep[farthest.1] = true;
            pending.extend([(start, farthest.1), (farthest.1, end)]);
        }
    }
}

fn simplify(raw: &[Point]) -> Vec<Point> {
    let count = raw.len();
    if count < 3 {
        return raw.to_vec();
    }
    let radius = (count / 6).min(3);
    let mut points: Vec<_> = (0..count)
        .map(|i| {
            let smoothed = (0..=2 * radius).fold([0.0, 0.0], |[x, y], offset| {
                let [px, py] = raw[(i + count + offset - radius) % count];
                [x + px, y + py]
            });
            smoothed.map(|coordinate| coordinate / (2 * radius + 1) as f64)
        })
        .collect();
    let mut split = 1;
    let mut farthest = 0.0;
    for i in 1..count {
        let distance = (points[i][0] - points[0][0]).hypot(points[i][1] - points[0][1]);
        if distance > farthest {
            split = i;
            farthest = distance;
        }
    }
    points.push(points[0]);
    let mut tolerance = 0.05;
    loop {
        let mut keep = vec![false; points.len()];
        keep[0] = true;
        keep[split] = true;
        retain_segment(&points, &mut keep, 0, split, tolerance);
        retain_segment(&points, &mut keep, split, count, tolerance);
        let mut ring: Vec<_> = points[..count]
            .iter()
            .zip(keep)
            .filter_map(|(point, keep)| {
                keep.then(|| point.map(|value| (value * 100.0 + 0.5).floor() / 100.0))
            })
            .collect();
        ring.dedup();
        if ring.first() == ring.last() && ring.len() > 1 {
            ring.pop();
        }
        if ring.len() <= 256 {
            return ring;
        }
        tolerance *= 1.5;
    }
}

fn valid_polygon(points: &[Point]) -> bool {
    let n = points.len();
    let cross = |[ox, oy]: Point, [ax, ay]: Point, [bx, by]: Point| {
        (ax - ox) * (by - oy) - (ay - oy) * (bx - ox)
    };
    let on_segment = |[x, y]: Point, [ax, ay]: Point, [bx, by]: Point| {
        x >= ax.min(bx) && x <= ax.max(bx) && y >= ay.min(by) && y <= ay.max(by)
    };
    let mut twice_area = 0.0;
    for (i, &a) in points.iter().enumerate() {
        let b = points[(i + 1) % n];
        twice_area += a[0] * b[1] - b[0] * a[1];
        for (j, &c) in points.iter().enumerate().skip(i + 2) {
            if i == 0 && j == n - 1 {
                continue;
            }
            let d = points[(j + 1) % n];
            let [abc, abd, cda, cdb] = [
                cross(a, b, c),
                cross(a, b, d),
                cross(c, d, a),
                cross(c, d, b),
            ];
            let overlaps = (abc.abs() < 1e-9 && on_segment(c, a, b))
                || (abd.abs() < 1e-9 && on_segment(d, a, b))
                || (cda.abs() < 1e-9 && on_segment(a, c, d))
                || (cdb.abs() < 1e-9 && on_segment(b, c, d));
            if (abc * abd < 0.0 && cda * cdb < 0.0) || overlaps {
                return false;
            }
        }
    }
    twice_area.abs() >= 0.125
}
