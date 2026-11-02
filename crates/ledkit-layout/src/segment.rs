//! Segments of a strip.

use crate::LayoutTooBig;

/// A run of LEDs on a strip, optionally counted from the far end.
///
/// Use it to treat part of a strip as a strip of its own, for example one
/// side of a light bar wired back-to-back with the other.
///
/// # Example
///
/// ```
/// use ledkit_color::Rgb;
/// use ledkit_layout::Segment;
///
/// let mut pixels = [Rgb::BLACK; 10];
/// let mut segment = Segment::new(2, 5).reversed().of(&mut pixels).unwrap();
/// segment.set(0, Rgb::RED); // the segment's first LED is its far end
/// assert_eq!(pixels[6], Rgb::RED);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    start: usize,
    len: usize,
    reversed: bool,
}

impl Segment {
    /// A segment of `len` LEDs starting at strip position `start`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::Segment;
    ///
    /// let first_eight = Segment::new(0, 8);
    /// ```
    pub const fn new(start: usize, len: usize) -> Self {
        Self {
            start,
            len,
            reversed: false,
        }
    }

    /// The same segment, counted from its far end.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::Segment;
    ///
    /// let backwards = Segment::new(8, 8).reversed();
    /// ```
    #[must_use]
    pub const fn reversed(self) -> Self {
        Self {
            reversed: !self.reversed,
            ..self
        }
    }

    /// The strip position of the segment's first LED.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::Segment;
    ///
    /// assert_eq!(Segment::new(8, 4).start(), 8);
    /// ```
    pub const fn start(&self) -> usize {
        self.start
    }

    /// How many LEDs the segment covers.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::Segment;
    ///
    /// assert_eq!(Segment::new(8, 4).len(), 4);
    /// ```
    pub const fn len(&self) -> usize {
        self.len
    }

    /// True if the segment covers no LEDs.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::Segment;
    ///
    /// assert!(Segment::new(3, 0).is_empty());
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// True if the segment counts from its far end (see [`reversed`](Segment::reversed)).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::Segment;
    ///
    /// assert!(Segment::new(0, 4).reversed().is_reversed());
    /// ```
    pub const fn is_reversed(&self) -> bool {
        self.reversed
    }

    /// Borrows this segment's LEDs out of `pixels`.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutTooBig`] if the segment reaches past the end of
    /// `pixels`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::Segment;
    ///
    /// let mut pixels = [Rgb::BLACK; 10];
    /// assert!(Segment::new(2, 5).of(&mut pixels).is_ok());
    /// assert!(Segment::new(8, 5).of(&mut pixels).is_err());
    /// ```
    pub fn of<'a, P>(&self, pixels: &'a mut [P]) -> Result<SegmentMut<'a, P>, LayoutTooBig> {
        let available = pixels.len();
        let end = self.start.saturating_add(self.len);
        if end > available {
            return Err(LayoutTooBig {
                needed: end,
                available,
            });
        }
        Ok(SegmentMut {
            pixels: &mut pixels[self.start..end],
            reversed: self.reversed,
        })
    }
}

/// The borrowed LEDs of a [`Segment`], numbered in segment order.
///
/// # Example
///
/// ```
/// use ledkit_color::Rgb;
/// use ledkit_layout::Segment;
///
/// let mut pixels = [Rgb::BLACK; 6];
/// let mut segment = Segment::new(1, 4).of(&mut pixels).unwrap();
/// segment.fill(Rgb::BLUE);
/// segment.set(0, Rgb::RED);
/// assert_eq!(segment.get(0), Some(Rgb::RED));
/// ```
#[derive(Debug)]
pub struct SegmentMut<'a, P> {
    pixels: &'a mut [P],
    reversed: bool,
}

impl<P> SegmentMut<'_, P> {
    /// How many LEDs the segment has.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::Segment;
    ///
    /// let mut pixels = [Rgb::BLACK; 6];
    /// assert_eq!(Segment::new(1, 4).of(&mut pixels).unwrap().len(), 4);
    /// ```
    pub fn len(&self) -> usize {
        self.pixels.len()
    }

    /// True if the segment has no LEDs.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::Segment;
    ///
    /// let mut pixels = [Rgb::BLACK; 6];
    /// assert!(Segment::new(1, 0).of(&mut pixels).unwrap().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }

    fn slot(&self, i: usize) -> Option<usize> {
        if i >= self.pixels.len() {
            None
        } else if self.reversed {
            Some(self.pixels.len() - 1 - i)
        } else {
            Some(i)
        }
    }

    /// Sets LED number `i` of the segment. Out-of-range `i` is ignored.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::Segment;
    ///
    /// let mut pixels = [Rgb::BLACK; 6];
    /// let mut segment = Segment::new(1, 4).of(&mut pixels).unwrap();
    /// segment.set(0, Rgb::RED);
    /// segment.set(99, Rgb::RED); // ignored
    /// assert_eq!(pixels[1], Rgb::RED);
    /// ```
    pub fn set(&mut self, i: usize, color: impl Into<P>) {
        if let Some(slot) = self.slot(i) {
            self.pixels[slot] = color.into();
        }
    }

    /// The color of LED number `i`, or `None` if `i` is out of range.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::Segment;
    ///
    /// let mut pixels = [Rgb::BLACK; 6];
    /// let segment = Segment::new(1, 4).of(&mut pixels).unwrap();
    /// assert_eq!(segment.get(0), Some(Rgb::BLACK));
    /// assert_eq!(segment.get(4), None);
    /// ```
    pub fn get(&self, i: usize) -> Option<P>
    where
        P: Copy,
    {
        self.slot(i).map(|slot| self.pixels[slot])
    }

    /// Sets every LED of the segment to `color`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::Segment;
    ///
    /// let mut pixels = [Rgb::BLACK; 6];
    /// Segment::new(1, 4).of(&mut pixels).unwrap().fill(Rgb::GREEN);
    /// assert_eq!(pixels[4], Rgb::GREEN);
    /// assert_eq!(pixels[5], Rgb::BLACK);
    /// ```
    pub fn fill(&mut self, color: impl Into<P>)
    where
        P: Clone,
    {
        let color: P = color.into();
        for slot in self.pixels.iter_mut() {
            *slot = color.clone();
        }
    }

    /// Every LED of the segment, in segment order (far end first if the
    /// segment is reversed).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::Segment;
    ///
    /// let mut pixels = [Rgb::BLACK; 4];
    /// let mut segment = Segment::new(0, 4).reversed().of(&mut pixels).unwrap();
    /// for (i, led) in segment.iter_mut().enumerate() {
    ///     *led = Rgb::new(i as u8, 0, 0);
    /// }
    /// assert_eq!(pixels[3], Rgb::new(0, 0, 0));
    /// assert_eq!(pixels[0], Rgb::new(3, 0, 0));
    /// ```
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut P> {
        let (forward, backward) = if self.reversed {
            (None, Some(self.pixels.iter_mut().rev()))
        } else {
            (Some(self.pixels.iter_mut()), None)
        };
        forward
            .into_iter()
            .flatten()
            .chain(backward.into_iter().flatten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ledkit_color::Rgb;

    #[test]
    fn reversed_segment_counts_from_its_far_end() {
        let mut px = [Rgb::BLACK; 10];
        let mut s = Segment::new(2, 5).reversed().of(&mut px).unwrap();
        s.set(0, Rgb::RED);
        assert_eq!(px[6], Rgb::RED);
    }

    #[test]
    fn forward_segment_fill_get_iter_and_bounds() {
        let mut px = [Rgb::BLACK; 10];
        let mut s = Segment::new(2, 3).of(&mut px).unwrap();
        assert_eq!(s.len(), 3);
        assert!(!s.is_empty());
        s.set(0, Rgb::RED);
        s.set(3, Rgb::RED); // out of range: ignored
        assert_eq!(s.get(0), Some(Rgb::RED));
        assert_eq!(s.get(3), None);
        s.fill(Rgb::GREEN);
        assert_eq!(s.iter_mut().count(), 3);
        assert!(px[2..5].iter().all(|p| *p == Rgb::GREEN));
        assert_eq!(px[1], Rgb::BLACK);
        assert_eq!(px[5], Rgb::BLACK);
    }

    #[test]
    fn reversed_iter_runs_backwards() {
        let mut px = [Rgb::BLACK; 5];
        let mut s = Segment::new(0, 5).reversed().of(&mut px).unwrap();
        for (n, p) in s.iter_mut().enumerate() {
            *p = Rgb::new(n as u8, 0, 0);
        }
        assert_eq!(px[4], Rgb::new(0, 0, 0));
        assert_eq!(px[0], Rgb::new(4, 0, 0));
    }

    #[test]
    fn segment_past_the_end_is_an_error() {
        let mut px = [Rgb::BLACK; 10];
        let err = Segment::new(8, 5).of(&mut px).err();
        assert_eq!(
            err,
            Some(LayoutTooBig {
                needed: 13,
                available: 10
            })
        );
        assert!(Segment::new(usize::MAX, 2).of(&mut px).is_err());
    }
}
