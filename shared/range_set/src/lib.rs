//! Provides `RangeSet`, a set of non-overlaping and non-contiguous overlaping u32 inclusive ranges.
//! Also includes some convenient methods for extremely simple physical memory allocations.

#![no_std]

const MAX_NUM_RANGES: usize = 32;

/// Describes an inclusive range of addresses, i.e. all addresses such that start <= addr <= end
#[derive(Copy, Clone, Debug)]
pub struct InclusiveRange {
    pub start: u32,
    pub end: u32
}

/// A set of non-overlaping and non-contiguous inclusive ranges
pub struct RangeSet {
    /// An array of ranges in the set
    ranges: [InclusiveRange; MAX_NUM_RANGES],

    /// Number of ranges actually in use. This is a u32 and not a usize so we can pass a RangeSet
    /// from 32 bit mode to 64 bit mode if we need to in the future.
    num_ranges: u32
}

impl RangeSet {
    /// Construct an empty RangeSet
    pub const fn new() -> Self {
        RangeSet {
            ranges: [InclusiveRange{start: 0, end: 0}; MAX_NUM_RANGES],
            num_ranges: 0
        }
    }

    /// Get all ranges in the set
    pub fn ranges(&self) -> &[InclusiveRange] {
        &self.ranges[..self.num_ranges as usize]
    }

    /// Deletes the range at index `idx` in the `ranges` array
    pub fn delete(&mut self, idx: usize) {
        // Make sure that index exists
        assert!(idx < self.num_ranges as usize);

        // Move back the ranges after the deleted range
        for i in idx..self.num_ranges as usize - 1 {
            self.ranges[i] = self.ranges[i+1];
        }

        // Decrease the number of ranges in use
        self.num_ranges -= 1;
    }

    /// Inserts `range` to the range set, merging ranges as necessary
    pub fn insert(&mut self, mut range: InclusiveRange) {
        assert!(range.start <= range.end);

        // We go in a loop and keep trying to find an existing range to merge with. We must do this
        // because after we merge our range with an exisiting one, the result might need to be
        // merged with another existing range and so on.
        'try_merge: loop {
            for i in 0..self.num_ranges as usize {
                if !should_merge_ranges(range, self.ranges[i]) {
                    continue;
                }

                // Update the range we insert to encompass this existing range
                range.start = core::cmp::min(range.start, self.ranges[i].start);
                range.end = core::cmp::max(range.end, self.ranges[i].end);

                // Delete the existing range
                self.delete(i);

                // Start searching for another range to merge with
                continue 'try_merge;
            }

            // If we went through the entire `ranges` array and did not find a range to merge with
            // we are done
            break;
        }

        // Assert we have enough space to insert the new range. We make this assertion here and not
        // at the start of the function because a merging of ranges might have created the space
        // that we needed
        assert!((self.num_ranges as usize) < self.ranges.len());

        // Add the new range to the `ranges` array and increment the number of ranges in use
        self.ranges[self.num_ranges as usize] = range;
        self.num_ranges += 1;
    }

    /// Removes `range` from the range set, trimming, splitting and deleting ranges as necessary
    pub fn remove(&mut self, range: InclusiveRange) {
        assert!(range.start <= range.end);

        // We go in a loop and try to subtract `range` from existing ranges instead of just going
        // over the array once because we might delete some ranges. This can be written in a more
        // effiecent manner only going over the array once, but it will be less clear and the length
        // of `ranges` is fixed anyway.
        'try_subtract: loop {
            for i in 0..self.num_ranges as usize {
                if !do_ranges_overlap(range, self.ranges[i]) {
                    continue;
                }

                // If the existing range is entirely contained in the range to remove we can just
                // delete it
                if does_range_contain(range, self.ranges[i]) {
                    self.delete(i);
                    continue 'try_subtract;
                }

                // If the range to delete does not contain the existing range, that means there is
                // partial overlap, so either need to shorten the existing range or split it
                
                if range.start <= self.ranges[i].start {
                    // If the range to delete overlaps the start of the existing range, just adjust
                    // its start.
                    self.ranges[i].start = range.end.saturating_add(1);
                } else if range.end >= self.ranges[i].end {
                    // If the range to delete overlaps the end of the existing range, just adjust
                    // its end.
                    self.ranges[i].end = range.start.saturating_sub(1);
                } else {
                    // If the range to delete is contained inside the existing range, we split it

                    // Assert we have enough room for a new range
                    assert!((self.num_ranges as usize) < self.ranges.len());
                    // We insert the new right range and increment the range count
                    self.ranges[self.num_ranges as usize] = InclusiveRange {
                        start: range.end.saturating_add(1),
                        end: self.ranges[i].end
                    };
                    self.num_ranges += 1;

                    // We set the existing range to be the new left range
                    self.ranges[i].end = range.start.saturating_sub(1);

                    // Because by definition the range set doesn't containing overlapping ranges,
                    // the fact that the range to delete is entirely contained inside this existing
                    // range means there is no need to subtract from any other ranges - we are done
                    break 'try_subtract;
                }
            }

            // If we went through the entire `ranges` array and did not find a range to subtract we
            // are done
            break;
        }
    }

    /// Adds up the size of all the ranges.
    /// 
    /// This will fail (and return None) if the RangeSet covers the entire address space, because in
    /// that case we can't represent the size (2^32).
    pub fn total_size(&self) -> Option<u32> {
        let mut sum = 0;
        for range in self.ranges() {
            sum += (range.end - range.start).checked_add(1)?;
        }

        Some(sum)
    }

    /// Allocates `size` bytes from the RangeSet under the `align` alignment requirement.
    /// 
    /// The alignment must be a power of two.
    pub fn allocate(&mut self, size: u32, align: u32) -> Option<usize> {
        // We can't allocate a unique address for zero bytes
        if size == 0 {
            return None;
        }

        // `align` must be a power of two
        if align.count_ones() != 1 {
            return None;
        }

        // We want the allocation with the least amount of padding, so we try and fit the allocation
        // in each range, and remember the best allocation with (padding_needed, allocation_addr)
        let mut best_allocation: Option<(u32, u32)> = None;
        for i in 0..self.num_ranges as usize {
            // We round up the start of the range to the alignment, so we can calculate if the
            // aligned allocation will fit in this range.
            let next_aligned_start = round_up_to_pow_of_2(self.ranges[i].start, align);
            if size <= (self.ranges[i].end - next_aligned_start).saturating_add(1) {
                // If it does fit, we calculate the padding needed
                let padding_needed = next_aligned_start - self.ranges[i].start;

                // Save this as the best allocation if we didn't find any allocation yet or it is
                // better than the best one we found
                if best_allocation.is_none() || padding_needed < best_allocation.unwrap().0 {
                    best_allocation = Some((padding_needed, next_aligned_start));
                }
            }
        }

        // Check if we found a valid allocation position
        if let Some((_, allocation_addr)) = best_allocation {
            // Remove the range used by the allocation
            self.remove(InclusiveRange {
                start: allocation_addr,
                end: allocation_addr + (size - 1)
            });

            // Return the allocation position
            Some(allocation_addr as usize)
        } else {
            // If we went through every range and found no range with room, we return `None`
            None
        }
    }

    /// Subtracts the RangeSet `other` from this RangeSet
    pub fn subtract(&mut self, other: &RangeSet) {
        // Remove every range in `other`
        for range in other.ranges() {
            self.remove(*range);
        }
    }
}

/// Checks whether or not the ranges `a` and `b` should be merged, i.e. checks if the ranges overlap
/// or are contiguous.
fn should_merge_ranges(mut a: InclusiveRange, mut b: InclusiveRange) -> bool {
    // Assuming `a` starts before `b` simplies the check, so we swap them if is the other way around
    if a.start > b.start {
        core::mem::swap(&mut a, &mut b);
    }

    // Finally, if `a` starts before `b` to check for overlap we just need to check that `b` starts
    // before `a` ends. To also detect contiguous ranged we add 1 to the end of `a`, saturating is
    // fine because if a extends to the end of the address space `b` cannot start after it anyway.
    b.start <= a.end.saturating_add(1)
}

/// Checks whether or not the ranges `a` and `b` overlap
fn do_ranges_overlap(a: InclusiveRange, b: InclusiveRange) -> bool {
    (a.start <= b.start && b.start <= a.end) || (b.start <= a.start && a.start <= b.end)
}

/// Checks whether the range `a` contains the range `b`
fn does_range_contain(a: InclusiveRange, b: InclusiveRange) -> bool {
    (a.start <= b.start) && (b.end <= a.end)
}

/// Rounds up `val` to the next multiple of `power` which must be a power of 2
fn round_up_to_pow_of_2(val: u32, power: u32) -> u32 {
    // Get a mask
    let mask = power - 1;

    // If we are already at a multiple, nothing to do
    if val & mask == 0 {
        return val;
    }

    // By and-ing with the inverted mask we essentially round down, and then add the power to get
    // the correct result
    (val & !mask) + power
}