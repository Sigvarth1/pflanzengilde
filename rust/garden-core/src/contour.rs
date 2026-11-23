//! Metric contours traced from the ground-cover density field.

pub type Point = [f64; 2];
pub type Ring = Vec<Point>;

pub fn signed_area(points: &[Point]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f64>()
        / 2.0
}

pub fn contains([x, y]: Point, ring: &[Point]) -> bool {
    let previous = ring.iter().rev().take(1).chain(ring);
    ring.iter()
        .zip(previous)
        .fold(false, |inside, (&[ax, ay], &[bx, by])| {
            inside ^ ((ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax)
        })
}

pub fn even_odd_area(rings: &[Ring]) -> f64 {
    rings
        .iter()
        .enumerate()
        .filter(|(_, ring)| !ring.is_empty())
        .map(|(index, ring)| {
            let area = signed_area(ring).abs();
            let hole = rings
                .iter()
                .enumerate()
                .fold(false, |inside, (other, points)| {
                    inside ^ (index != other && contains(ring[0], points))
                });
            if hole {
                -area
            } else {
                area
            }
        })
        .sum::<f64>()
        .max(0.0)
}

/// Trace a row-major field with at least one below-threshold sample around its
/// border. The returned rings omit their repeated closing vertices.
pub fn marching_squares(field: &[f32], width: usize, height: usize, iso: f64) -> Vec<Ring> {
    assert_eq!(field.len(), width * height);
    if width < 2 || height < 2 {
        return Vec::new();
    }
    let at = |index: usize| f64::from(field[index]);
    let point = |edge: usize| {
        let base = edge / 2;
        let x = (base % width) as f64;
        let y = (base / width) as f64;
        let vertical = edge % 2 != 0;
        let a = at(base);
        let b = at(base + if vertical { width } else { 1 });
        let t = if a == b {
            0.5
        } else {
            ((iso - a) / (b - a)).clamp(0.0, 1.0)
        };
        if vertical {
            [x, y + t]
        } else {
            [x + t, y]
        }
    };
    // Horizontal and vertical edges start at 2 * index and 2 * index + 1.
    let mut next = vec![usize::MAX; field.len() * 2];
    let mut starts = Vec::new();
    let mut add = |from, to| {
        if next[from] == usize::MAX {
            starts.push(from);
        }
        next[from] = to;
    };
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let index = y * width + x;
            let samples = [
                at(index),
                at(index + 1),
                at(index + width + 1),
                at(index + width),
            ];
            let case = samples
                .into_iter()
                .fold(0, |case, sample| (case << 1) | u8::from(sample >= iso));
            let top = 2 * index;
            let bottom = 2 * (index + width);
            let left = top + 1;
            let right = top + 3;
            match case {
                1 => add(bottom, left),
                2 => add(right, bottom),
                3 => add(right, left),
                4 => add(top, right),
                5 => {
                    if samples.iter().sum::<f64>() / 4.0 >= iso {
                        add(top, left);
                        add(bottom, right);
                    } else {
                        add(top, right);
                        add(bottom, left);
                    }
                }
                6 => add(top, bottom),
                7 => add(top, left),
                8 => add(left, top),
                9 => add(bottom, top),
                10 => {
                    if samples.iter().sum::<f64>() / 4.0 >= iso {
                        add(left, bottom);
                        add(right, top);
                    } else {
                        add(left, top);
                        add(right, bottom);
                    }
                }
                11 => add(right, top),
                12 => add(left, right),
                13 => add(bottom, right),
                14 => add(left, bottom),
                _ => {}
            }
        }
    }

    let mut rings = Vec::new();
    for start in starts {
        let mut ring = Vec::new();
        let mut edge = start;
        while next[edge] != usize::MAX {
            ring.push(point(edge));
            edge = std::mem::replace(&mut next[edge], usize::MAX);
        }
        if ring.len() >= 3 && edge == start {
            rings.push(ring);
        }
    }
    rings
}

pub fn simplify(ring: Ring, tolerance: f64) -> Ring {
    if ring.len() <= 4 {
        return ring;
    }
    let [x, y] = ring[0];
    let mut far = 1;
    let mut best = -1.0;
    for (index, &[px, py]) in ring.iter().enumerate().skip(1) {
        let distance = (px - x).powi(2) + (py - y).powi(2);
        if distance > best {
            far = index;
            best = distance;
        }
    }
    let mut tail = ring[far..].to_vec();
    tail.push(ring[0]);
    let mut result = Vec::new();
    simplify_segment(&ring[..=far], tolerance, &mut result);
    simplify_segment(&tail, tolerance, &mut result);
    result
}

// Append the first point of each retained segment, excluding the common end.
fn simplify_segment(points: &[Point], tolerance: f64, result: &mut Ring) {
    let [x1, y1] = points[0];
    let [x2, y2] = points[points.len() - 1];
    let [dx, dy] = [x2 - x1, y2 - y1];
    let length = dx.hypot(dy);
    let length = if length == 0.0 { 1e-9 } else { length };
    let mut far = None;
    let mut max = tolerance;
    for (index, &[x, y]) in points.iter().enumerate().skip(1).take(points.len() - 2) {
        let distance = (dy * x - dx * y + x2 * y1 - y2 * x1).abs() / length;
        if distance > max {
            far = Some(index);
            max = distance;
        }
    }
    if let Some(index) = far {
        simplify_segment(&points[..=index], tolerance, result);
        simplify_segment(&points[index..], tolerance, result);
    } else {
        result.push(points[0]);
    }
}

pub fn chaikin(mut ring: Ring, iterations: usize) -> Ring {
    if ring.len() < 3 {
        return ring;
    }
    for _ in 0..iterations {
        ring = ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .flat_map(|(&[x, y], &[ex, ey])| {
                [
                    [0.75 * x + 0.25 * ex, 0.75 * y + 0.25 * ey],
                    [0.25 * x + 0.75 * ex, 0.25 * y + 0.75 * ey],
                ]
            })
            .collect();
    }
    ring
}

/// Separable Gaussian smoothing. Keep the outermost cells at zero to leave a
/// padded border for marching squares.
pub fn gaussian_blur(field: Vec<f32>, width: usize, height: usize, sigma: f64) -> Vec<f32> {
    assert_eq!(field.len(), width * height);
    if sigma < 0.3 || field.is_empty() {
        return field;
    }
    let radius = (sigma * 2.0).ceil().max(1.0) as isize;
    let mut kernel: Vec<_> = (-radius..=radius)
        .map(|offset| (-(offset as f64).powi(2) / (2.0 * sigma * sigma)).exp() as f32)
        .collect();
    let weight: f64 = kernel.iter().copied().map(f64::from).sum();
    for value in &mut kernel {
        *value = (f64::from(*value) / weight) as f32;
    }
    let blend = |samples: &[f32], center: usize, length: usize, base: usize, stride: usize| {
        kernel
            .iter()
            .enumerate()
            .fold(0.0, |acc, (index, &weight)| {
                let cell = center as isize + index as isize - radius;
                if cell >= 0 && (cell as usize) < length {
                    acc + f64::from(samples[base + cell as usize * stride]) * f64::from(weight)
                } else {
                    acc
                }
            }) as f32
    };
    let mut horizontal = vec![0.0; field.len()];
    for (y, row) in horizontal.chunks_exact_mut(width).enumerate() {
        for (x, sample) in row.iter_mut().enumerate() {
            *sample = blend(&field, x, width, y * width, 1);
        }
    }
    let mut output = vec![0.0; field.len()];
    for y in 1..height.saturating_sub(1) {
        for x in 1..width.saturating_sub(1) {
            output[y * width + x] = blend(&horizontal, y, height, x, width);
        }
    }
    output
}
