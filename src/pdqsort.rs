//! Port of Go's `slices.SortFunc` (pattern-defeating quicksort) from the Go
//! standard library (`slices/zsortanyfunc.go`).
//!
//! This is ported verbatim (including Go's non-stable behavior for equal
//! elements) so that the order of equal-level commemorations and readings
//! matches the Go reference CLI byte-for-byte.

#[derive(Clone, Copy, PartialEq)]
enum Hint {
    Unknown,
    Increasing,
    Decreasing,
}

/// Sorts data using the same algorithm and tie-breaking as Go's
/// `slices.SortFunc`.
pub fn pdqsort<T>(data: &mut [T], cmp: &dyn Fn(&T, &T) -> i64) {
    let n = data.len() as i64;
    pdqsort_rec(data, 0, n, bits_len(data.len() as u64) as i64, cmp);
}

fn bits_len(x: u64) -> u32 {
    if x == 0 {
        0
    } else {
        64 - x.leading_zeros()
    }
}

fn next_power_of_two(length: i64) -> u64 {
    1u64 << bits_len(length as u64)
}

/// insertionSortCmpFunc sorts data[a:b] using insertion sort.
fn insertion_sort<T>(data: &mut [T], a: i64, b: i64, cmp: &dyn Fn(&T, &T) -> i64) {
    for i in a + 1..b {
        let mut j = i;
        while j > a && cmp(&data[j as usize], &data[(j - 1) as usize]) < 0 {
            data.swap(j as usize, (j - 1) as usize);
            j -= 1;
        }
    }
}

/// siftDownCmpFunc implements the heap property on data[lo:hi].
/// first is an offset into the array where the root of the heap lies.
fn sift_down<T>(data: &mut [T], lo: i64, hi: i64, first: i64, cmp: &dyn Fn(&T, &T) -> i64) {
    let mut root = lo;
    loop {
        let mut child = 2 * root + 1;
        if child >= hi {
            break;
        }
        if child + 1 < hi
            && cmp(
                &data[(first + child) as usize],
                &data[(first + child + 1) as usize],
            ) < 0
        {
            child += 1;
        }
        if !(cmp(
            &data[(first + root) as usize],
            &data[(first + child) as usize],
        ) < 0)
        {
            return;
        }
        data.swap((first + root) as usize, (first + child) as usize);
        root = child;
    }
}

fn heap_sort<T>(data: &mut [T], a: i64, b: i64, cmp: &dyn Fn(&T, &T) -> i64) {
    let first = a;
    let lo = 0;
    let hi = b - a;

    // Build heap with greatest element at top.
    let mut i = (hi - 1) / 2;
    while i >= 0 {
        sift_down(data, i, hi, first, cmp);
        i -= 1;
    }

    // Pop elements, largest first, into end of data.
    for i in (0..hi).rev() {
        data.swap(first as usize, (first + i) as usize);
        sift_down(data, lo, i, first, cmp);
    }
}

/// pdqsortCmpFunc sorts data[a:b].
/// limit is the number of allowed bad (very unbalanced) pivots before falling back to heapsort.
fn pdqsort_rec<T>(
    data: &mut [T],
    mut a: i64,
    mut b: i64,
    mut limit: i64,
    cmp: &dyn Fn(&T, &T) -> i64,
) {
    const MAX_INSERTION: i64 = 12;

    let mut was_balanced = true; // whether the last partitioning was reasonably balanced
    let mut was_partitioned = true; // whether the slice was already partitioned

    loop {
        let length = b - a;

        if length <= MAX_INSERTION {
            insertion_sort(data, a, b, cmp);
            return;
        }

        // Fall back to heapsort if too many bad choices were made.
        if limit == 0 {
            heap_sort(data, a, b, cmp);
            return;
        }

        // If the last partitioning was imbalanced, we need to breaking patterns.
        if !was_balanced {
            break_patterns(data, a, b);
            limit -= 1;
        }

        let (mut pivot, mut hint) = choose_pivot(data, a, b, cmp);
        if hint == Hint::Decreasing {
            reverse_range(data, a, b);
            // The chosen pivot was pivot-a elements after the start of the array.
            // After reversing it is pivot-a elements before the end of the array.
            // The idea came from Rust's implementation.
            pivot = (b - 1) - (pivot - a);
            hint = Hint::Increasing;
        }

        // The slice is likely already sorted.
        if was_balanced && was_partitioned && hint == Hint::Increasing {
            if partial_insertion_sort(data, a, b, cmp) {
                return;
            }
        }

        // Probably the slice contains many duplicate elements, partition the
        // slice into elements equal to and elements greater than the pivot.
        if a > 0 && !(cmp(&data[(a - 1) as usize], &data[pivot as usize]) < 0) {
            let mid = partition_equal(data, a, b, pivot, cmp);
            a = mid;
            continue;
        }

        let (mid, already_partitioned) = partition(data, a, b, pivot, cmp);
        was_partitioned = already_partitioned;

        let (left_len, right_len) = (mid - a, b - mid);
        let balance_threshold = length / 8;
        if left_len < right_len {
            was_balanced = left_len >= balance_threshold;
            pdqsort_rec(data, a, mid, limit, cmp);
            a = mid + 1;
        } else {
            was_balanced = right_len >= balance_threshold;
            pdqsort_rec(data, mid + 1, b, limit, cmp);
            b = mid;
        }
    }
}

/// partitionCmpFunc does one quicksort partition.
/// Let p = data[pivot]
/// Moves elements in data[a:b] around, so that data[i]<p and data[j]>=p for
/// i<newpivot and j>newpivot. On return, data[newpivot] = p
fn partition<T>(
    data: &mut [T],
    a: i64,
    b: i64,
    pivot: i64,
    cmp: &dyn Fn(&T, &T) -> i64,
) -> (i64, bool) {
    data.swap(a as usize, pivot as usize);
    let mut i = a + 1;
    let mut j = b - 1; // i and j are inclusive of the elements remaining to be partitioned

    while i <= j && cmp(&data[i as usize], &data[a as usize]) < 0 {
        i += 1;
    }
    while i <= j && !(cmp(&data[j as usize], &data[a as usize]) < 0) {
        j -= 1;
    }
    if i > j {
        data.swap(j as usize, a as usize);
        return (j, true);
    }
    data.swap(i as usize, j as usize);
    i += 1;
    j -= 1;

    loop {
        while i <= j && cmp(&data[i as usize], &data[a as usize]) < 0 {
            i += 1;
        }
        while i <= j && !(cmp(&data[j as usize], &data[a as usize]) < 0) {
            j -= 1;
        }
        if i > j {
            break;
        }
        data.swap(i as usize, j as usize);
        i += 1;
        j -= 1;
    }
    data.swap(j as usize, a as usize);
    (j, false)
}

/// partitionEqualCmpFunc partitions data[a:b] into elements equal to
/// data[pivot] followed by elements greater than data[pivot].
/// It is assumed that data[a:b] does not contain elements smaller than
/// the data[pivot].
fn partition_equal<T>(
    data: &mut [T],
    a: i64,
    b: i64,
    pivot: i64,
    cmp: &dyn Fn(&T, &T) -> i64,
) -> i64 {
    data.swap(a as usize, pivot as usize);
    let mut i = a + 1;
    let mut j = b - 1; // i and j are inclusive of the elements remaining to be partitioned

    loop {
        while i <= j && !(cmp(&data[a as usize], &data[i as usize]) < 0) {
            i += 1;
        }
        while i <= j && cmp(&data[a as usize], &data[j as usize]) < 0 {
            j -= 1;
        }
        if i > j {
            break;
        }
        data.swap(i as usize, j as usize);
        i += 1;
        j -= 1;
    }
    i
}

/// partialInsertionSortCmpFunc partially sorts a slice, returns true if the
/// slice is sorted at the end.
fn partial_insertion_sort<T>(data: &mut [T], a: i64, b: i64, cmp: &dyn Fn(&T, &T) -> i64) -> bool {
    const MAX_STEPS: i64 = 5; // maximum number of adjacent out-of-order pairs that will get shifted
    const SHORTEST_SHIFTING: i64 = 50; // don't shift any elements on short arrays

    let mut i = a + 1;
    for _ in 0..MAX_STEPS {
        while i < b && !(cmp(&data[i as usize], &data[(i - 1) as usize]) < 0) {
            i += 1;
        }

        if i == b {
            return true;
        }

        if b - a < SHORTEST_SHIFTING {
            return false;
        }

        data.swap(i as usize, (i - 1) as usize);

        // Shift the smaller one to the left.
        if i - a >= 2 {
            let mut j = i - 1;
            while j >= 1 {
                if !(cmp(&data[j as usize], &data[(j - 1) as usize]) < 0) {
                    break;
                }
                data.swap(j as usize, (j - 1) as usize);
                j -= 1;
            }
        }
        // Shift the greater one to the right.
        if b - i >= 2 {
            let mut j = i + 1;
            while j < b {
                if !(cmp(&data[j as usize], &data[(j - 1) as usize]) < 0) {
                    break;
                }
                data.swap(j as usize, (j - 1) as usize);
                j += 1;
            }
        }
    }
    false
}

/// breakPatternsCmpFunc scatters some elements around in an attempt to break
/// some patterns that might cause imbalanced partitions in quicksort.
///
/// xorshift paper: https://www.jstatsoft.org/article/view/v008i14/xorshift.pdf
fn break_patterns<T>(data: &mut [T], a: i64, b: i64) {
    let length = b - a;
    if length >= 8 {
        let mut random = length as u64;
        let modulus = next_power_of_two(length);

        for idx in (a + (length / 4) * 2 - 1)..=(a + (length / 4) * 2 + 1) {
            random ^= random.wrapping_shl(13);
            random ^= random >> 7;
            random ^= random.wrapping_shl(17);
            let mut other = (random & (modulus - 1)) as i64;
            if other >= length {
                other -= length;
            }
            data.swap(idx as usize, (a + other) as usize);
        }
    }
}

/// choosePivotCmpFunc chooses a pivot in data[a:b].
///
/// [0,8): chooses a static pivot.
/// [8,shortestNinther): uses the simple median-of-three method.
/// [shortestNinther,∞): uses the Tukey ninther method.
fn choose_pivot<T>(data: &mut [T], a: i64, b: i64, cmp: &dyn Fn(&T, &T) -> i64) -> (i64, Hint) {
    const SHORTEST_NINTHER: i64 = 50;
    const MAX_SWAPS: i64 = 4 * 3;

    let l = b - a;

    let mut swaps = 0;
    let mut i = a + l / 4 * 1;
    let mut j = a + l / 4 * 2;
    let mut k = a + l / 4 * 3;

    if l >= 8 {
        if l >= SHORTEST_NINTHER {
            // Tukey ninther method, the idea came from Rust's implementation.
            i = median_adjacent(data, i, &mut swaps, cmp);
            j = median_adjacent(data, j, &mut swaps, cmp);
            k = median_adjacent(data, k, &mut swaps, cmp);
        }
        // Find the median among i, j, k and stores it into j.
        j = median3(data, i, j, k, &mut swaps, cmp);
    }

    let hint = if swaps == 0 {
        Hint::Increasing
    } else if swaps == MAX_SWAPS {
        Hint::Decreasing
    } else {
        Hint::Unknown
    };
    (j, hint)
}

/// order2CmpFunc returns x,y where data[x] <= data[y], where x,y=a,b or x,y=b,a.
fn order2<T>(
    data: &mut [T],
    a: i64,
    b: i64,
    swaps: &mut i64,
    cmp: &dyn Fn(&T, &T) -> i64,
) -> (i64, i64) {
    if cmp(&data[b as usize], &data[a as usize]) < 0 {
        *swaps += 1;
        (b, a)
    } else {
        (a, b)
    }
}

/// medianCmpFunc returns x where data[x] is the median of data[a],data[b],data[c],
/// where x is a, b, or c.
fn median3<T>(
    data: &mut [T],
    a: i64,
    b: i64,
    c: i64,
    swaps: &mut i64,
    cmp: &dyn Fn(&T, &T) -> i64,
) -> i64 {
    let (a, b) = order2(data, a, b, swaps, cmp);
    let (b, _c) = order2(data, b, c, swaps, cmp);
    let (_a, b) = order2(data, a, b, swaps, cmp);
    b
}

/// medianAdjacentCmpFunc finds the median of data[a - 1], data[a], data[a + 1]
/// and stores the index into a.
fn median_adjacent<T>(data: &mut [T], a: i64, swaps: &mut i64, cmp: &dyn Fn(&T, &T) -> i64) -> i64 {
    median3(data, a - 1, a, a + 1, swaps, cmp)
}

fn reverse_range<T>(data: &mut [T], a: i64, b: i64) {
    let mut i = a;
    let mut j = b - 1;
    while i < j {
        data.swap(i as usize, j as usize);
        i += 1;
        j -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdqsort_basic() {
        let mut v: Vec<i64> = vec![5, 3, 8, 1, 9, 2, 7, 4, 6, 0, 11, 10, 13, 12, 14, 15];
        pdqsort(&mut v, &|a: &i64, b: &i64| a.cmp(b) as i64);
        assert_eq!(v, (0..16).collect::<Vec<i64>>());
    }

    #[test]
    fn test_pdqsort_descending_input() {
        let mut v: Vec<i64> = (0..100).map(|i| -i).collect();
        pdqsort(&mut v, &|a: &i64, b: &i64| a.cmp(b) as i64);
        assert_eq!(v, (-99..=0).collect::<Vec<i64>>());
    }

    #[test]
    fn test_pdqsort_all_equal() {
        let mut v: Vec<i64> = vec![7; 64];
        pdqsort(&mut v, &|a: &i64, b: &i64| a.cmp(b) as i64);
        assert_eq!(v, vec![7; 64]);
    }

    #[test]
    fn test_pdqsort_empty() {
        let mut v: Vec<i64> = vec![];
        pdqsort(&mut v, &|a: &i64, b: &i64| a.cmp(b) as i64);
        assert!(v.is_empty());
    }
}
