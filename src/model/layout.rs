//! Pure layout geometry shared by Rail's Niri-inspired column/window manager.

pub(crate) const COLUMN_WIDTH_PRESETS: [f64; 2] = [0.5, 1.0];
pub(crate) const WINDOW_HEIGHT_PRESETS: [f64; 3] = [0.5, 2.0 / 3.0, 1.0];
pub(crate) const MIN_COLUMN_WIDTH: f64 = 180.0;
pub(crate) const MIN_WINDOW_HEIGHT: f64 = 80.0;

/// Return the first preset larger than the current share, wrapping to the first preset.
pub(crate) fn next_layout_preset(presets: &[f64], current: f64) -> Option<f64> {
    if presets.is_empty() {
        return None;
    }
    if !current.is_finite() {
        return Some(presets[0]);
    }
    presets
        .iter()
        .copied()
        .find(|preset| *preset > current + 0.025)
        .or_else(|| Some(presets[0]))
}

/// Center a world-space span inside the viewport without allowing camera overscroll.
pub(crate) fn centered_offset(viewport: f64, start: f64, end: f64, max_offset: f64) -> f64 {
    if !viewport.is_finite() || !start.is_finite() || !end.is_finite() || !max_offset.is_finite() {
        return 0.0;
    }
    let (left, right) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    ((left + right) * 0.5 - viewport.max(0.0) * 0.5).clamp(0.0, max_offset.max(0.0))
}

/// Clamp a desired column width to the viewport while preserving a usable minimum.
pub(crate) fn clamp_column_width(width: f64, viewport: f64) -> f64 {
    let viewport = viewport.max(1.0);
    let min = MIN_COLUMN_WIDTH.min(viewport);
    if !width.is_finite() {
        return min;
    }
    width.clamp(min, viewport)
}

/// Convert a vector of positive height weights into exact pixel allocations.
/// Each window receives at least `min_height`; the result sums to `total_height`.
pub(crate) fn distribute_height_weights(
    weights: &[f64],
    total_height: f64,
    min_height: f64,
) -> Vec<f64> {
    let count = weights.len();
    if count == 0 {
        return Vec::new();
    }
    let total = if total_height.is_finite() {
        total_height.max(1.0)
    } else {
        1.0
    };
    let requested_minimum = if min_height.is_finite() {
        min_height.max(1.0)
    } else {
        1.0
    };
    let minimum = requested_minimum.min(total / count as f64);
    if count == 1 {
        return vec![total];
    }

    let mut result = vec![0.0; count];
    let mut active: Vec<usize> = (0..count).collect();
    let mut remaining = total;

    // Water-filling keeps the requested proportions except where a window would
    // become smaller than the minimum. Clamped windows are removed and the rest
    // of the available height is redistributed proportionally.
    loop {
        let weight_sum: f64 = active
            .iter()
            .map(|&index| {
                let weight = weights[index];
                if weight.is_finite() && weight > 0.0 {
                    weight
                } else {
                    1.0
                }
            })
            .sum();
        if active.is_empty() || weight_sum <= 0.0 {
            break;
        }

        let below_min: Vec<usize> = active
            .iter()
            .copied()
            .filter(|&index| {
                let weight = weights[index];
                let safe_weight = if weight.is_finite() && weight > 0.0 {
                    weight
                } else {
                    1.0
                };
                remaining * safe_weight / weight_sum < minimum
            })
            .collect();

        if below_min.is_empty() {
            for &index in &active {
                let weight = weights[index];
                let safe_weight = if weight.is_finite() && weight > 0.0 {
                    weight
                } else {
                    1.0
                };
                result[index] = remaining * safe_weight / weight_sum;
            }
            break;
        }

        for index in below_min {
            result[index] = minimum;
            remaining -= minimum;
            active.retain(|candidate| *candidate != index);
        }
    }

    // Absorb floating-point residue in the largest allocation so that adjacent
    // column handles cannot accumulate a one-pixel gap after repeated resizing.
    let sum: f64 = result.iter().sum();
    if let Some((largest, _)) = result
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
    {
        result[largest] += total - sum;
    }
    result
}

/// Desired active-window height as a percentage of the complete column.
/// Remaining windows share the rest equally; no window is pushed below the minimum.
pub(crate) fn window_height_preset_distribution(
    total_height: f64,
    window_count: usize,
    active_index: usize,
    proportion: f64,
    min_height: f64,
) -> Vec<f64> {
    if window_count == 0 {
        return Vec::new();
    }
    let total = if total_height.is_finite() {
        total_height.max(1.0)
    } else {
        1.0
    };
    let requested_minimum = if min_height.is_finite() {
        min_height.max(1.0)
    } else {
        1.0
    };
    let minimum = requested_minimum.min(total / window_count as f64);
    let active = active_index.min(window_count - 1);
    let proportion = if proportion.is_finite() {
        proportion.clamp(0.0, 1.0)
    } else {
        1.0 / window_count as f64
    };
    if window_count == 1 {
        return vec![total];
    }

    let max_active = (total - minimum * (window_count - 1) as f64).max(minimum);
    let active_height = (total * proportion).clamp(minimum, max_active);
    let sibling_height = (total - active_height) / (window_count - 1) as f64;
    let mut heights = vec![sibling_height; window_count];
    heights[active] = active_height;
    heights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_presets_cycle_and_manual_width_chooses_nearest() {
        assert_eq!(next_layout_preset(&COLUMN_WIDTH_PRESETS, 0.5), Some(1.0));
        assert_eq!(next_layout_preset(&COLUMN_WIDTH_PRESETS, 1.0), Some(0.5));
        assert_eq!(next_layout_preset(&COLUMN_WIDTH_PRESETS, 0.72), Some(1.0));
        assert_eq!(
            next_layout_preset(&WINDOW_HEIGHT_PRESETS, 1.0 / 3.0),
            Some(0.5)
        );
        assert_eq!(next_layout_preset(&[], 0.5), None);
    }

    #[test]
    fn centered_offset_centers_spans_and_never_overscrolls() {
        assert_eq!(centered_offset(1000.0, 1200.0, 1600.0, 3000.0), 900.0);
        assert_eq!(centered_offset(1000.0, 0.0, 300.0, 3000.0), 0.0);
        assert_eq!(centered_offset(1000.0, 3600.0, 4000.0, 3000.0), 3000.0);
        assert_eq!(centered_offset(1000.0, f64::NAN, 1000.0, 3000.0), 0.0);
    }

    #[test]
    fn column_width_clamps_to_viewport_and_minimum() {
        assert_eq!(clamp_column_width(600.0, 1200.0), 600.0);
        assert_eq!(clamp_column_width(1400.0, 1200.0), 1200.0);
        assert_eq!(clamp_column_width(20.0, 1200.0), MIN_COLUMN_WIDTH);
        assert_eq!(clamp_column_width(20.0, 180.0), 180.0);
    }

    #[test]
    fn height_distribution_preserves_requested_weights_and_exact_total() {
        let heights = distribute_height_weights(&[0.5, 0.3, 0.2], 900.0, 80.0);
        assert!((heights[0] - 450.0).abs() < 1e-8);
        assert!((heights[1] - 270.0).abs() < 1e-8);
        assert!((heights[2] - 180.0).abs() < 1e-8);
        assert!((heights.iter().sum::<f64>() - 900.0).abs() < 1e-8);
    }

    #[test]
    fn height_distribution_respects_minimum_without_overflow() {
        let heights = distribute_height_weights(&[0.99, 0.005, 0.005], 900.0, 80.0);
        assert!(heights.iter().all(|height| *height >= 80.0));
        assert!((heights.iter().sum::<f64>() - 900.0).abs() < 1e-8);
        assert!((heights[1] - 80.0).abs() < 1e-8);
        assert!((heights[2] - 80.0).abs() < 1e-8);
    }

    #[test]
    fn height_distribution_never_exceeds_a_short_viewport() {
        let heights = distribute_height_weights(&[0.5, 0.5], 100.0, 80.0);
        assert!((heights.iter().sum::<f64>() - 100.0).abs() < 1e-8);
        assert!(heights.iter().all(|height| *height >= 50.0));
    }

    #[test]
    fn single_window_always_uses_the_whole_column_height() {
        assert_eq!(
            window_height_preset_distribution(700.0, 1, 0, f64::NAN, 80.0),
            vec![700.0]
        );
    }

    #[test]
    fn non_finite_viewport_inputs_are_safely_normalized() {
        let heights = distribute_height_weights(&[1.0, 1.0], f64::INFINITY, f64::NAN);
        assert_eq!(heights, vec![0.5, 0.5]);
    }

    #[test]
    fn invalid_height_proportion_falls_back_to_equal_allocation() {
        let heights = window_height_preset_distribution(600.0, 3, 1, f64::NAN, 80.0);
        assert_eq!(heights, vec![200.0, 200.0, 200.0]);
    }

    #[test]
    fn window_height_presets_match_niri_proportions_and_keep_siblings_visible() {
        let half = window_height_preset_distribution(900.0, 2, 0, 0.5, 80.0);
        assert!((half[0] - 450.0).abs() < 1e-8);
        assert!((half[1] - 450.0).abs() < 1e-8);

        let two_thirds = window_height_preset_distribution(900.0, 3, 1, 2.0 / 3.0, 80.0);
        assert!((two_thirds[1] - 600.0).abs() < 1e-8);
        assert!((two_thirds[0] - 150.0).abs() < 1e-8);
        assert!((two_thirds[2] - 150.0).abs() < 1e-8);

        let full = window_height_preset_distribution(900.0, 3, 0, 1.0, 80.0);
        assert!((full[0] - 740.0).abs() < 1e-8);
        assert!((full[1] - 80.0).abs() < 1e-8);
        assert!((full[2] - 80.0).abs() < 1e-8);
    }
}
