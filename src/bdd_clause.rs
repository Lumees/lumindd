// lumindd — Two-literal clause extraction from BDDs
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::manager::Manager;
use crate::node::NodeId;

/// A literal: a variable with a polarity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Literal {
    /// Variable index.
    pub var: u16,
    /// True = positive literal (xi), false = negated literal (!xi).
    pub positive: bool,
}

impl Literal {
    /// Create a new positive literal.
    pub fn pos(var: u16) -> Self {
        Literal { var, positive: true }
    }

    /// Create a new negative literal.
    pub fn neg(var: u16) -> Self {
        Literal { var, positive: false }
    }
}

impl Manager {
    /// Extract all two-literal clauses implied by `f`.
    ///
    /// A two-literal clause is `(l1 OR l2)` where each literal is a variable
    /// or its negation. The function returns all such clauses that are implied
    /// by `f` (i.e., every satisfying assignment of `f` also satisfies the clause).
    ///
    /// This is done by checking, for every pair of literals, whether `f` implies
    /// the disjunction of the two literals.
    pub fn bdd_two_literal_clauses(&mut self, f: NodeId) -> Vec<(Literal, Literal)> {
        if f.is_constant() {
            return Vec::new();
        }

        let n = self.num_vars;
        let mut clauses = Vec::new();

        // For each pair of variables (i, j) with i < j, and for each polarity
        // combination, check whether f implies (li OR lj).
        // f implies (li OR lj) iff f AND !li AND !lj = ZERO.
        for i in 0..n {
            let vi = self.bdd_ith_var(i);
            for j in (i + 1)..n {
                let vj = self.bdd_ith_var(j);

                // Check all four polarity combinations: (pi, pj) in {T,F}x{T,F}
                // Clause (li OR lj) is implied by f iff f AND !li AND !lj = 0
                // Where !li = if li is positive, then !vi, else vi
                let polarities: [(bool, bool); 4] = [
                    (true, true),
                    (true, false),
                    (false, true),
                    (false, false),
                ];

                for (pi, pj) in polarities {
                    let neg_li = if pi { vi.not() } else { vi };
                    let neg_lj = if pj { vj.not() } else { vj };

                    // f AND (negation of literal i) AND (negation of literal j)
                    let tmp = self.bdd_and(f, neg_li);
                    let check = self.bdd_and(tmp, neg_lj);

                    if check.is_zero() {
                        clauses.push((
                            Literal { var: i, positive: pi },
                            Literal { var: j, positive: pj },
                        ));
                    }
                }
            }
        }

        clauses
    }

    /// Extract all implications: if variable `xi=val_i` implies `xj=val_j` under `f`.
    ///
    /// Returns tuples `(var_i, val_i, var_j, val_j)` meaning:
    /// for all assignments satisfying `f`, if `xi = val_i` then `xj = val_j`.
    ///
    /// An implication `(xi=a) => (xj=b)` under `f` is equivalent to:
    /// `f AND (xi=a) AND (xj=!b) = ZERO`.
    pub fn bdd_implication_pairs(&mut self, f: NodeId) -> Vec<(u16, bool, u16, bool)> {
        if f.is_constant() {
            return Vec::new();
        }

        let n = self.num_vars;
        let mut implications = Vec::new();

        for i in 0..n {
            let vi = self.bdd_ith_var(i);
            for j in 0..n {
                if i == j {
                    continue;
                }
                let vj = self.bdd_ith_var(j);

                // Check all 4 combinations: (val_i, val_j)
                for val_i in [true, false] {
                    for val_j in [true, false] {
                        // Premise: xi = val_i
                        let premise = if val_i { vi } else { vi.not() };
                        // Negation of conclusion: xj = !val_j
                        let neg_conc = if val_j { vj.not() } else { vj };

                        let tmp = self.bdd_and(f, premise);
                        let check = self.bdd_and(tmp, neg_conc);

                        if check.is_zero() {
                            implications.push((i, val_i, j, val_j));
                        }
                    }
                }
            }
        }

        implications
    }
}

#[cfg(test)]
mod tests {
    use super::Literal;
    use crate::Manager;

    #[test]
    fn clause_from_implication_bdd() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        let y = mgr.bdd_new_var(); // x1
        // f = x0 OR x1 — this is itself a two-literal clause
        let f = mgr.bdd_or(x, y);
        let clauses = mgr.bdd_two_literal_clauses(f);
        // Should contain (x0, x1) with positive polarities
        assert!(clauses.contains(&(Literal::pos(0), Literal::pos(1))));
    }

    #[test]
    fn clause_from_implies() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        let y = mgr.bdd_new_var(); // x1
        // f = !x0 OR x1 (x0 implies x1)
        let f = mgr.bdd_or(x.not(), y);
        let clauses = mgr.bdd_two_literal_clauses(f);
        // Should contain (!x0 OR x1)
        assert!(clauses.contains(&(Literal::neg(0), Literal::pos(1))));
    }

    #[test]
    fn no_clauses_for_tautology() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();
        // ONE implies every clause, so all 2-literal clauses are implied
        let clauses = mgr.bdd_two_literal_clauses(mgr.one());
        // Tautology is a constant — returns empty
        assert!(clauses.is_empty());
    }

    #[test]
    fn implications_x_implies_y() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        let y = mgr.bdd_new_var(); // x1
        // f = !x0 OR x1 means x0=true implies x1=true
        let f = mgr.bdd_or(x.not(), y);
        let impls = mgr.bdd_implication_pairs(f);
        // Should contain (0, true, 1, true): x0=T => x1=T
        assert!(impls.contains(&(0, true, 1, true)));
        // Contrapositive: x1=F => x0=F
        assert!(impls.contains(&(1, false, 0, false)));
    }

    #[test]
    fn implications_and() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        // f = x AND y: both must be true
        let f = mgr.bdd_and(x, y);
        let impls = mgr.bdd_implication_pairs(f);
        // Under f, x0=true always, so x0=T => x1=T and x1=T => x0=T
        assert!(impls.contains(&(0, true, 1, true)));
        assert!(impls.contains(&(1, true, 0, true)));
    }

    #[test]
    fn no_implications_for_constant() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();
        let impls = mgr.bdd_implication_pairs(mgr.zero());
        assert!(impls.is_empty());
    }
}
