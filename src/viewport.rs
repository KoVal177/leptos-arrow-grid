//! Viewport tracking and math for the virtualized grid.
//!
//! All types and functions are pure — no signals, no I/O, no side effects.

/// Conservative upper bound for browser scrollable height in CSS pixels.
///
/// Real browser limits vary, but once a grid spacer exceeds this range,
/// some engines clamp `scrollTop` and large datasets become unreachable.
const MAX_SCROLLABLE_HEIGHT_PX: f64 = 16_000_000.0;

/// Tracks horizontal scroll state for column virtualization.
#[derive(Clone, Debug, Default)]
pub struct HorizontalViewport {
    /// Current horizontal scroll position in pixels.
    pub scroll_left: f64,
    /// Visible width of the container in pixels.
    pub container_width: f64,
}

/// Current and last-communicated scroll window.
///
/// `last_emitted` tracks the `(start_row, visible_rows)` pair that was most
/// recently delivered to the host via `on_viewport_change`.  When it equals
/// the current pair, the callback is suppressed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewportState {
    /// Index of the first visible row (derived from `scrollTop / row_height`).
    pub start_row: u64,
    /// Number of rows that fit in the container at the current height.
    pub visible_rows: usize,
    /// The `(start_row, visible_rows)` pair last delivered to `on_viewport_change`.
    /// `None` means the callback has never fired.
    pub last_emitted: Option<(u64, usize)>,
}

impl ViewportState {
    /// Returns `true` when the host should be notified of a viewport change.
    ///
    /// The callback is suppressed when the current window is identical to the
    /// last-emitted window, preventing redundant host-side writes.
    #[must_use]
    pub fn should_emit(&self) -> bool {
        self.last_emitted != Some((self.start_row, self.visible_rows))
    }

    /// Returns a copy of `self` with `last_emitted` stamped to the current window.
    ///
    /// Call this *after* deciding to emit, so the next call to `should_emit`
    /// returns `false` for the same viewport.
    #[must_use]
    pub fn with_emitted(self) -> Self {
        Self {
            last_emitted: Some((self.start_row, self.visible_rows)),
            ..self
        }
    }
}

/// The range of rows that should be visible given the current scroll state.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportRange {
    /// Absolute index of the first visible row (inclusive).
    pub first_row: u64,
    /// Number of rows to render.
    pub row_count: usize,
}

/// Compute the visible row range for a given scroll state.
///
/// When `overscan > 0` the range is expanded by that many rows above and
/// below the visible window, reducing flicker during fast scrolling.
///
/// # Guarantees (enforced by proptest)
///
/// - `first_row + row_count <= total_rows` always.
/// - `first_row` is always a valid row index when `total_rows > 0`.
/// - `row_count >= 1` when `total_rows > 0`.
pub fn compute_viewport(
    scroll_top_px: f64,
    container_height_px: f64,
    row_height_px: f64,
    total_rows: u64,
) -> ViewportRange {
    compute_viewport_with_overscan(
        scroll_top_px,
        container_height_px,
        row_height_px,
        total_rows,
        0,
    )
}

/// Like [`compute_viewport`] but with a configurable overscan buffer.
pub fn compute_viewport_with_overscan(
    scroll_top_px: f64,
    container_height_px: f64,
    row_height_px: f64,
    total_rows: u64,
    overscan: usize,
) -> ViewportRange {
    if total_rows == 0 || row_height_px <= 0.0 || container_height_px <= 0.0 {
        return ViewportRange {
            first_row: 0,
            row_count: 0,
        };
    }

    let virtual_scroll_top = scroll_top_to_virtual_offset_px(
        scroll_top_px,
        container_height_px,
        row_height_px,
        total_rows,
    );

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let first_visible = (virtual_scroll_top / row_height_px).floor() as u64;
    let first_row = first_visible.saturating_sub(overscan as u64);
    let first_row = first_row.min(total_rows.saturating_sub(1));

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let visible = (container_height_px / row_height_px).ceil() as usize;
    // Add one extra row to handle partial rows at viewport boundary,
    // plus overscan above and below.
    let visible = visible + 1 + 2 * overscan;

    #[allow(clippy::cast_possible_truncation)]
    let remaining = (total_rows - first_row) as usize;
    let row_count = visible.min(remaining);

    ViewportRange {
        first_row,
        row_count,
    }
}

/// Total scrollable height for a dataset.
pub fn total_height_px(total_rows: u64, row_height_px: f64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let result = total_rows as f64 * row_height_px;
    result
}

/// Browser-safe scroll spacer height for the dataset.
pub fn scrollable_height_px(total_rows: u64, row_height_px: f64) -> f64 {
    total_height_px(total_rows, row_height_px).min(MAX_SCROLLABLE_HEIGHT_PX)
}

/// Map a container `scrollTop` to the virtual row offset in unscaled pixels.
pub fn scroll_top_to_virtual_offset_px(
    scroll_top_px: f64,
    container_height_px: f64,
    row_height_px: f64,
    total_rows: u64,
) -> f64 {
    if total_rows == 0 || row_height_px <= 0.0 || container_height_px <= 0.0 {
        return 0.0;
    }

    let actual_total_height = total_height_px(total_rows, row_height_px);
    let scrollable_height = scrollable_height_px(total_rows, row_height_px);
    let max_virtual_scroll_top = (actual_total_height - container_height_px).max(0.0);
    let max_scroll_top = (scrollable_height - container_height_px).max(0.0);

    if max_virtual_scroll_top <= 0.0 || max_scroll_top <= 0.0 {
        return 0.0;
    }

    let clamped_scroll_top = scroll_top_px.clamp(0.0, max_scroll_top);
    (clamped_scroll_top / max_scroll_top) * max_virtual_scroll_top
}

/// Map a virtual row offset in unscaled pixels to container `scrollTop`.
pub fn virtual_offset_to_scroll_top_px(
    virtual_offset_px: f64,
    container_height_px: f64,
    row_height_px: f64,
    total_rows: u64,
) -> f64 {
    if total_rows == 0 || row_height_px <= 0.0 || container_height_px <= 0.0 {
        return 0.0;
    }

    let actual_total_height = total_height_px(total_rows, row_height_px);
    let scrollable_height = scrollable_height_px(total_rows, row_height_px);
    let max_virtual_scroll_top = (actual_total_height - container_height_px).max(0.0);
    let max_scroll_top = (scrollable_height - container_height_px).max(0.0);

    if max_virtual_scroll_top <= 0.0 || max_scroll_top <= 0.0 {
        return 0.0;
    }

    let clamped_virtual_offset = virtual_offset_px.clamp(0.0, max_virtual_scroll_top);
    (clamped_virtual_offset / max_virtual_scroll_top) * max_scroll_top
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    // ── HorizontalViewport tests ───────────────────────────────────────────

    #[test]
    fn horizontal_viewport_default() {
        let hv = HorizontalViewport::default();
        assert!((hv.scroll_left - 0.0).abs() < f64::EPSILON);
        assert!((hv.container_width - 0.0).abs() < f64::EPSILON);
    }

    // ── ViewportState dedupe tests ─────────────────────────────────────────

    #[test]
    fn default_always_emits() {
        // On first render last_emitted is None, so should_emit must be true
        // regardless of start_row / visible_rows.
        let vp = ViewportState::default();
        assert!(vp.should_emit());
    }

    #[test]
    fn same_values_after_emit_suppresses() {
        let vp = ViewportState {
            start_row: 0,
            visible_rows: 20,
            last_emitted: None,
        };
        let vp2 = vp.with_emitted();
        assert!(!vp2.should_emit());
    }

    #[test]
    fn changed_start_row_emits() {
        let vp = ViewportState {
            start_row: 0,
            visible_rows: 20,
            last_emitted: None,
        }
        .with_emitted();
        let vp2 = ViewportState {
            start_row: 5,
            visible_rows: 20,
            last_emitted: vp.last_emitted,
        };
        assert!(vp2.should_emit());
    }

    #[test]
    fn changed_visible_rows_emits() {
        let vp = ViewportState {
            start_row: 0,
            visible_rows: 20,
            last_emitted: None,
        }
        .with_emitted();
        let vp2 = ViewportState {
            start_row: 0,
            visible_rows: 25,
            last_emitted: vp.last_emitted,
        };
        assert!(vp2.should_emit());
    }

    #[test]
    fn with_emitted_is_pure() {
        let vp = ViewportState {
            start_row: 7,
            visible_rows: 15,
            last_emitted: None,
        };
        let vp2 = vp.with_emitted();
        // Original unchanged
        assert_eq!(vp.last_emitted, None);
        // Copy stamped
        assert_eq!(vp2.last_emitted, Some((7, 15)));
    }

    // ── compute_viewport tests ─────────────────────────────────────────────

    #[test]
    fn zero_rows_returns_zero_range() {
        let vp = compute_viewport(0.0, 600.0, 28.0, 0);
        assert_eq!(vp.first_row, 0);
        assert_eq!(vp.row_count, 0);
    }

    #[test]
    fn basic_viewport() {
        let vp = compute_viewport(0.0, 280.0, 28.0, 1000);
        assert_eq!(vp.first_row, 0);
        // ceil(280/28) + 1 = 11
        assert_eq!(vp.row_count, 11);
    }

    #[test]
    fn scrolled_halfway() {
        let vp = compute_viewport(2800.0, 280.0, 28.0, 1000);
        // floor(2800/28) = 100
        assert_eq!(vp.first_row, 100);
        assert_eq!(vp.row_count, 11);
    }

    #[test]
    fn scroll_past_end_clamps() {
        let vp = compute_viewport(1_000_000.0, 280.0, 28.0, 100);
        assert_eq!(vp.first_row, 90);
        assert_eq!(vp.row_count, 10);
    }

    #[test]
    fn total_height() {
        assert!((total_height_px(1000, 28.0) - 28000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn scrollable_height_caps_large_datasets() {
        assert_eq!(
            scrollable_height_px(10_000_000, 24.0),
            MAX_SCROLLABLE_HEIGHT_PX
        );
    }

    #[test]
    fn large_dataset_scroll_roundtrip_stays_within_one_row() {
        let virtual_offset = 120_000_000.0;
        let scroll_top = virtual_offset_to_scroll_top_px(virtual_offset, 240.0, 24.0, 10_000_000);
        let roundtrip = scroll_top_to_virtual_offset_px(scroll_top, 240.0, 24.0, 10_000_000);
        assert!((roundtrip - virtual_offset).abs() < 24.0);
    }

    #[test]
    fn compute_viewport_handles_scaled_scroll_ranges() {
        let scroll_top = scrollable_height_px(10_000_000, 24.0) / 2.0;
        let vp = compute_viewport(scroll_top, 240.0, 24.0, 10_000_000);
        assert!(vp.first_row > 4_500_000);
        assert!(vp.first_row < 5_500_000);
        assert_eq!(vp.row_count, 11);
    }
    // ── compute_viewport_with_overscan tests ──────────────────────────────

    #[test]
    fn overscan_expands_range() {
        let vp = compute_viewport_with_overscan(280.0, 280.0, 28.0, 1000, 5);
        // first_visible = 10, overscan = 5 → first_row = 5
        assert_eq!(vp.first_row, 5);
        // visible = ceil(280/28)+1 = 11, + 2*5 = 21
        assert_eq!(vp.row_count, 21);
    }

    #[test]
    fn overscan_clamps_at_start() {
        let vp = compute_viewport_with_overscan(0.0, 280.0, 28.0, 1000, 5);
        // first_visible = 0, saturating_sub(5) = 0
        assert_eq!(vp.first_row, 0);
    }

    #[test]
    fn overscan_clamps_at_end() {
        // Near end: first_visible = 995, overscan=5 → first_row=990
        // visible = ceil(280/28)+1=11 + 2*5=21, but only 10 rows remaining from 990
        let vp = compute_viewport_with_overscan(27860.0, 280.0, 28.0, 1000, 5);
        assert!(vp.first_row + vp.row_count as u64 <= 1000);
    }

    #[test]
    fn zero_overscan_is_equivalent() {
        let a = compute_viewport(500.0, 600.0, 28.0, 1000);
        let b = compute_viewport_with_overscan(500.0, 600.0, 28.0, 1000, 0);
        assert_eq!(a, b);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn viewport_is_always_in_bounds(
            scroll_top in 0.0f64..1_000_000.0f64,
            container_height in 100.0f64..2000.0f64,
            row_height in 1.0f64..200.0f64,
            total_rows in 0u64..10_000_000u64,
        ) {
            let vp = compute_viewport(scroll_top, container_height, row_height, total_rows);
            prop_assert!(vp.first_row + vp.row_count as u64 <= total_rows);
        }

        #[test]
        fn viewport_row_count_is_nonzero_when_data_exists(
            scroll_top in 0.0f64..1_000_000.0f64,
            container_height in 100.0f64..2000.0f64,
            row_height in 1.0f64..200.0f64,
            total_rows in 1u64..10_000_000u64,
        ) {
            let vp = compute_viewport(scroll_top, container_height, row_height, total_rows);
            prop_assert!(vp.row_count >= 1);
        }

        #[test]
        fn viewport_first_row_never_exceeds_total(
            scroll_top in 0.0f64..1_000_000_000.0f64,
            container_height in 100.0f64..2000.0f64,
            row_height in 1.0f64..200.0f64,
            total_rows in 1u64..10_000_000u64,
        ) {
            let vp = compute_viewport(scroll_top, container_height, row_height, total_rows);
            prop_assert!(vp.first_row < total_rows);
        }

        #[test]
        fn overscan_viewport_always_in_bounds(
            scroll_top in 0.0f64..1_000_000.0f64,
            container_height in 100.0f64..2000.0f64,
            row_height in 1.0f64..200.0f64,
            total_rows in 0u64..10_000_000u64,
            overscan in 0usize..20usize,
        ) {
            let vp = compute_viewport_with_overscan(
                scroll_top, container_height, row_height, total_rows, overscan,
            );
            prop_assert!(vp.first_row + vp.row_count as u64 <= total_rows);
        }
    }
}
