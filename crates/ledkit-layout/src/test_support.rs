//! Test helpers shared by the layout tests.

/// Asserts that `index` maps every `(x, y)` of a `width` by `height` grid to a
/// different number, and that together they are exactly `0..width * height`.
/// Also asserts that positions just outside the grid give `None`.
pub(crate) fn assert_bijection(
    width: u16,
    height: u16,
    index: impl Fn(u16, u16) -> Option<usize>,
    what: &str,
) {
    let mut seen = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let i = index(x, y).unwrap_or_else(|| panic!("{what}: ({x}, {y}) is None"));
            seen.push(i);
        }
    }
    seen.sort_unstable();
    let expected: Vec<usize> = (0..usize::from(width) * usize::from(height)).collect();
    assert_eq!(seen, expected, "{what}: not one-to-one onto 0..len");
    assert_eq!(index(width, 0), None, "{what}: x == width");
    assert_eq!(index(0, height), None, "{what}: y == height");
}
