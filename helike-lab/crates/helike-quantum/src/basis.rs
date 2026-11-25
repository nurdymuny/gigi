//! Occupation states of six oscillators with total quanta ≤ nmax, in the order of
//! `helike_condensate_fock.states_up_to` (by total, then `combinations_with_replacement` order).

use std::collections::HashMap;

pub type State = [u8; 6];

pub fn states_up_to(nmax: usize) -> Vec<State> {
    // non-decreasing index sequences of length tot over 0..6, in lexicographic order
    fn rec(start: usize, left: usize, occ: &mut State, out: &mut Vec<State>) {
        if left == 0 {
            out.push(*occ);
            return;
        }
        for i in start..6 {
            occ[i] += 1;
            rec(i, left - 1, occ, out);
            occ[i] -= 1;
        }
    }
    let mut out = vec![];
    for tot in 0..=nmax {
        rec(0, tot, &mut [0u8; 6], &mut out);
    }
    out
}

pub fn index_of(states: &[State]) -> HashMap<State, usize> {
    states.iter().enumerate().map(|(i, s)| (*s, i)).collect()
}

pub fn dimension(nmax: usize) -> usize {
    // C(nmax + 6, 6)
    (1..=6).fold(1usize, |acc, k| acc * (nmax + k) / k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_order() {
        for n in 0..7 {
            let s = states_up_to(n);
            assert_eq!(s.len(), dimension(n), "nmax={n}");
            let uniq: std::collections::HashSet<_> = s.iter().collect();
            assert_eq!(uniq.len(), s.len());
        }
        let s = states_up_to(2);
        assert_eq!(s[0], [0; 6]);
        assert_eq!(s[1], [1, 0, 0, 0, 0, 0]);
        assert_eq!(s[6], [0, 0, 0, 0, 0, 1]);
        assert_eq!(s[7], [2, 0, 0, 0, 0, 0]);
        assert_eq!(s[8], [1, 1, 0, 0, 0, 0]);
        assert_eq!(s[13], [0, 2, 0, 0, 0, 0]);
        assert_eq!(*s.last().unwrap(), [0, 0, 0, 0, 0, 2]);
    }
}
