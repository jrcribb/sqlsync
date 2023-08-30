use std::{
    fmt::{Debug, Display},
    ops::Range,
};

use serde::{Deserialize, Serialize};

pub type Lsn = u64;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LsnRange {
    /// first marks the beginning of the range, inclusive.
    first: Lsn,
    /// last marks the end of the range, inclusive.
    last: Lsn,
}

impl LsnRange {
    pub fn new(first: Lsn, last: Lsn) -> Self {
        assert!(first <= last, "first must be <= last");
        LsnRange { first, last }
    }

    pub fn last(&self) -> Lsn {
        self.last
    }

    pub fn len(&self) -> usize {
        (self.last - self.first + 1) as usize
    }

    pub fn contains(&self, lsn: Lsn) -> bool {
        self.first <= lsn && lsn <= self.last
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.last >= other.first && self.first <= other.last
    }

    pub fn immediately_preceeds(&self, other: &Self) -> bool {
        self.last + 1 == other.first
    }

    pub fn immediately_follows(&self, other: &Self) -> bool {
        other.immediately_preceeds(self)
    }

    pub fn offset(&self, lsn: Lsn) -> Option<usize> {
        if self.contains(lsn) {
            Some((lsn - self.first) as usize)
        } else {
            None
        }
    }

    pub fn intersection_offsets(&self, other: &Self) -> Range<usize> {
        if self.intersects(other) {
            let start = std::cmp::max(self.first, other.first) - self.first;
            let end = std::cmp::min(self.last, other.last) - self.first + 1;
            start as usize..end as usize
        } else {
            0..0
        }
    }

    // returns a new LsnRange with all lsns <= up_to removed
    // returns None if the resulting range is empty
    pub fn trim_prefix(&self, up_to: Lsn) -> Option<LsnRange> {
        if up_to >= self.last {
            return None;
        }
        if up_to < self.first {
            return Some(*self);
        }
        Some(LsnRange::new(up_to + 1, self.last))
    }

    pub fn extend_by(&self, len: u64) -> LsnRange {
        assert!(len > 0, "len must be >= 0");
        LsnRange::new(self.first, self.last + len)
    }

    pub fn intersect(&self, other: &Self) -> Option<LsnRange> {
        if self.intersects(other) {
            Some(LsnRange::new(
                std::cmp::max(self.first, other.first),
                std::cmp::min(self.last, other.last),
            ))
        } else {
            None
        }
    }

    /// Unions this range with another.
    /// Panics if the two ranges do not overlap.
    pub fn union(&self, other: &Self) -> LsnRange {
        // Check for overlap
        assert!(
            self.intersects(other)
                || self.immediately_preceeds(other)
                || self.immediately_follows(other),
            "ranges do not intersect or meet. self: {:?}, other: {:?}",
            self,
            other
        );

        // Union the two overlapping ranges
        LsnRange::new(
            std::cmp::min(self.first, other.first),
            std::cmp::max(self.last, other.last),
        )
    }
}

impl Debug for LsnRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LsnRange")
            .field(&self.first)
            .field(&self.last)
            .finish()
    }
}

impl Display for LsnRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.first, self.last)
    }
}

// write some tests for LsnRange
#[cfg(test)]
mod tests {
    use testutil::assert_panic;

    use super::LsnRange;

    #[test]
    #[should_panic(expected = "first must be <= last")]
    fn lsnrange_invariant() {
        LsnRange::new(5, 0);
    }

    #[test]
    fn lsnrange_len() {
        assert_eq!(LsnRange::new(0, 0).len(), 1);
        assert_eq!(LsnRange::new(0, 1).len(), 2);
        assert_eq!(LsnRange::new(5, 10).len(), 6);
    }

    #[test]
    fn lsnrange_contains() {
        let range = LsnRange::new(5, 10);

        assert!(!range.contains(0));
        assert!(range.contains(5));
        assert!(range.contains(6));
        assert!(range.contains(10));
        assert!(!range.contains(11));
    }

    #[test]
    fn lsnrange_intersects() {
        let range = LsnRange::new(5, 10);

        macro_rules! t {
            ($other:expr, $intersection:expr, $offsets:expr) => {
                assert_eq!(
                    !range.intersects(&$other),
                    $offsets.is_empty(),
                    "checking intersects: {:?}, {:?}",
                    range,
                    $other
                );
                assert_eq!(
                    range.intersect(&$other),
                    $intersection,
                    "checking intersection: {:?}, {:?}",
                    range,
                    $other
                );
                assert_eq!(
                    range.intersection_offsets(&$other),
                    $offsets,
                    "checking intersection_offsets: {:?}, {:?}",
                    range,
                    $other
                );
                match $intersection {
                    Some(intersection) => {
                        let first = intersection.first;
                        for i in 0..(intersection.len() as u64) {
                            assert_eq!(
                                range.offset(first + i),
                                Some(($offsets.start + i) as usize)
                            );
                        }
                    }
                    None => {
                        assert_eq!(range.offset($other.first), None);
                    }
                }
            };
        }

        macro_rules! r {
            ($first:expr, $last:expr) => {
                LsnRange::new($first, $last)
            };
        }

        t!(r!(0, 4), None::<LsnRange>, 0..0);
        t!(r!(0, 5), Some(r!(5, 5)), 0..1);
        t!(r!(0, 6), Some(r!(5, 6)), 0..2);
        t!(r!(0, 10), Some(r!(5, 10)), 0..6);
        t!(r!(0, 11), Some(r!(5, 10)), 0..6);
        t!(r!(5, 5), Some(r!(5, 5)), 0..1);
        t!(r!(5, 6), Some(r!(5, 6)), 0..2);
        t!(r!(5, 10), Some(r!(5, 10)), 0..6);
        t!(r!(5, 11), Some(r!(5, 10)), 0..6);
        t!(r!(9, 10), Some(r!(9, 10)), 4..6);
        t!(r!(10, 10), Some(r!(10, 10)), 5..6);
        t!(r!(10, 11), Some(r!(10, 10)), 5..6);
        t!(r!(11, 11), None::<LsnRange>, 0..0);
        t!(r!(20, 30), None::<LsnRange>, 0..0);
    }

    #[test]
    fn lsnrange_preceeds_follows() {
        let range = LsnRange::new(5, 10);

        macro_rules! t {
            ($other:expr, $result:literal) => {
                assert_eq!(range.immediately_preceeds(&$other), $result);
                assert_eq!($other.immediately_follows(&range), $result);
            };
        }

        t!(LsnRange::new(0, 4), false);
        t!(LsnRange::new(0, 5), false);
        t!(LsnRange::new(0, 6), false);
        t!(LsnRange::new(9, 10), false);
        t!(LsnRange::new(10, 10), false);
        t!(LsnRange::new(10, 11), false);
        t!(LsnRange::new(11, 11), true);
        t!(LsnRange::new(11, 12), true);
        t!(LsnRange::new(12, 12), false);
    }

    #[test]
    fn lsnrange_trim_prefix() {
        let range = LsnRange::new(5, 10);

        assert_eq!(range.trim_prefix(0), Some(range));
        assert_eq!(range.trim_prefix(4), Some(range));
        assert_eq!(range.trim_prefix(5), Some(LsnRange::new(6, 10)));
        assert_eq!(range.trim_prefix(6), Some(LsnRange::new(7, 10)));
        assert_eq!(range.trim_prefix(7), Some(LsnRange::new(8, 10)));
        assert_eq!(range.trim_prefix(8), Some(LsnRange::new(9, 10)));
        assert_eq!(range.trim_prefix(9), Some(LsnRange::new(10, 10)));
        assert_eq!(range.trim_prefix(10), None);
        assert_eq!(range.trim_prefix(20), None);
    }

    #[test]
    #[should_panic(expected = "len must be >= 0")]
    fn lsnrange_extend_invariant() {
        LsnRange::new(5, 10).extend_by(0);
    }

    #[test]
    fn lsnrange_extend() {
        let range = LsnRange::new(5, 10);
        assert_eq!(range.extend_by(1), LsnRange::new(5, 11));
        assert_eq!(range.extend_by(2), LsnRange::new(5, 12));
    }

    #[test]
    fn lsnrange_union_invariant() {
        let range = LsnRange::new(5, 10);
        assert_panic!(
            { range.union(&LsnRange::new(0, 0)); },
            String,
            starts with "ranges do not intersect or meet"
        );
        assert_panic!(
            { range.union(&LsnRange::new(0, 3)); },
            String,
            starts with "ranges do not intersect or meet"
        );
        assert_panic!(
            { range.union(&LsnRange::new(12, 12)); },
            String,
            starts with "ranges do not intersect or meet"
        );
        assert_panic!(
            { range.union(&LsnRange::new(15, 20)); },
            String,
            starts with "ranges do not intersect or meet"
        );
    }

    #[test]
    fn lsnrange_union() {
        let range = LsnRange::new(5, 10);
        assert_eq!(range.union(&LsnRange::new(0, 4)), LsnRange::new(0, 10));
        assert_eq!(range.union(&LsnRange::new(4, 4)), LsnRange::new(4, 10));
        assert_eq!(range.union(&LsnRange::new(5, 5)), LsnRange::new(5, 10));
        assert_eq!(range.union(&LsnRange::new(5, 10)), LsnRange::new(5, 10));
        assert_eq!(range.union(&LsnRange::new(7, 10)), LsnRange::new(5, 10));
        assert_eq!(range.union(&LsnRange::new(10, 10)), LsnRange::new(5, 10));
        assert_eq!(range.union(&LsnRange::new(10, 11)), LsnRange::new(5, 11));
        assert_eq!(range.union(&LsnRange::new(11, 11)), LsnRange::new(5, 11));
        assert_eq!(range.union(&LsnRange::new(11, 15)), LsnRange::new(5, 15));
        assert_eq!(range.union(&LsnRange::new(4, 11)), LsnRange::new(4, 11));
        assert_eq!(range.union(&LsnRange::new(0, 100)), LsnRange::new(0, 100));
    }
}
