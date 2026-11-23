//! Runs the iterative garden spacing pass in one call from the browser.
//! gardenSpacingWasm.ts documents the input buffers.

pub mod contour;
mod outline;
pub use outline::Outline;

use std::cell::RefCell;

pub const RECORD_LEN: usize = 6;
const EPS: f64 = 1e-9;
const ALLIUM: u32 = 1;
const LEGUME: u32 = 2;
const FENNEL: u32 = 4;
const WORMWOOD: u32 = 8;

thread_local! {
    static PLANTS: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
    static SITE: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}

#[no_mangle]
pub extern "C" fn reserve_plants(count: usize) -> *mut f64 {
    PLANTS.with_borrow_mut(|plants| {
        plants.resize(count * RECORD_LEN, 0.0);
        plants.as_mut_ptr()
    })
}

pub fn round_cm(value: f64) -> f64 {
    (value * 100.0 + 1e-6 + 0.5).floor() / 100.0
}

fn separate(a: &mut [f64], b: &mut [f64]) {
    let (a_flags, b_flags) = (a[3] as u32, b[3] as u32);
    let combined = a_flags | b_flags;
    let is_allium_legume = (a_flags & ALLIUM != 0 && b_flags & LEGUME != 0)
        || (a_flags & LEGUME != 0 && b_flags & ALLIUM != 0);
    let mut min_dist = 0.6_f64.max((a[2] + b[2]) * 0.4);
    if is_allium_legume {
        min_dist = min_dist.max(1.95);
    } else if combined & FENNEL != 0 {
        min_dist = min_dist.max(1.65);
    } else if combined & WORMWOOD != 0 {
        min_dist = min_dist.max(1.35);
    }

    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    // Reject distant pairs before computing a hypotenuse.
    let threshold = min_dist - EPS;
    if dx.abs() >= threshold || dy.abs() >= threshold {
        return;
    }
    let dist = dx.hypot(dy);
    if dist.is_nan() || dist >= threshold {
        return;
    }

    let angle = if dist > 0.05 + EPS {
        dy.atan2(dx)
    } else {
        // Reconstruct the original UTF-16 hash of "a|b", with JS i32 wrapping.
        let hash = (a[4] as i32)
            .wrapping_mul(31)
            .wrapping_add(i32::from(b'|'))
            .wrapping_mul(b[5] as i32)
            .wrapping_add(b[4] as i32);
        (hash.unsigned_abs() % 360) as f64 * std::f64::consts::PI / 180.0
    };
    let push = (min_dist - dist + 0.06) / 2.0;
    let (x, y) = (push * angle.cos(), push * angle.sin());
    a[0] = round_cm(a[0] - x);
    a[1] = round_cm(a[1] - y);
    b[0] = round_cm(b[0] + x);
    b[1] = round_cm(b[1] + y);
}

#[no_mangle]
pub extern "C" fn reserve_site(values: usize) -> *mut f64 {
    SITE.with_borrow_mut(|site| {
        site.resize(values, 0.0);
        site.as_mut_ptr()
    })
}

fn clear_trunks(plant: &mut [f64], trees: &[f64]) {
    let allium = plant[3] as u32 & ALLIUM != 0;
    for tree in trees.chunks_exact(3) {
        let (dx, dy) = (plant[0] - tree[0], plant[1] - tree[1]);
        let clearance = if allium && tree[2] != 0.0 { 1.95 } else { 0.4 };
        let threshold = clearance - EPS;
        if dx.abs() >= threshold || dy.abs() >= threshold {
            continue;
        }
        let distance = dx.hypot(dy);
        if distance < threshold {
            let angle = if distance > 0.01 + EPS {
                dy.atan2(dx)
            } else {
                0.5
            };
            plant[0] = round_cm(tree[0] + clearance * angle.cos());
            plant[1] = round_cm(tree[1] + clearance * angle.sin());
        }
    }
}

/// Apply the 16 existing relaxation passes. Plant records contain the x and y
/// coordinates, spread, species flags and two values for the deterministic pair hash.
pub fn relax(plants: &mut [f64], trees: &[f64], outline: Option<&Outline<'_>>) {
    for _ in 0..16 {
        for i in (0..plants.len()).step_by(RECORD_LEN) {
            let (left, right) = plants.split_at_mut(i + RECORD_LEN);
            let a = &mut left[i..];
            for b in right.chunks_exact_mut(RECORD_LEN) {
                separate(a, b);
            }
        }
        for plant in plants.chunks_exact_mut(RECORD_LEN) {
            clear_trunks(plant, trees);
        }
        if let Some(outline) = outline {
            for plant in plants.chunks_exact_mut(RECORD_LEN) {
                outline.constrain(plant);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn relax_spacing(tree_count: usize) {
    SITE.with_borrow(|site| {
        let (trees, outline_data) = site.split_at(tree_count * 3);
        let outline = Outline::from_buffer(outline_data);
        PLANTS.with_borrow_mut(|plants| relax(plants, trees, outline.as_ref()));
    });
}
