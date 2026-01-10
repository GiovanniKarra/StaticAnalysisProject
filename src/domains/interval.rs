use std::{cmp, ops, fmt};
use fmt::Write;
use std::cell::RefCell;
use impl_ops::*;

use crate::semantics::*;
use super::AbstractDomain;

const INF: i64 = i64::MAX;

thread_local!(pub static INT_BOUNDS: RefCell<(i64, i64)> = RefCell::new((-INF, INF)));

pub fn get_bounds() -> (i64, i64) {
    INT_BOUNDS.with_borrow(|(a, b)| (*a, *b))
}

pub fn set_bounds(m: i64, n: i64) {
    INT_BOUNDS.set((m, n));
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interval {
    Int(i64, i64),
    Bottom
}

impl Interval {
    pub fn new(a: i64, b: i64) -> Interval {
        let (m, n) = get_bounds();
        if a > b {
            Interval::Bottom
        } else {
            Interval::Int(a.max(m).min(n), b.max(m).min(n))
        }
    }
}

impl PartialOrd for Interval {
	fn partial_cmp(&self, other: &Interval) -> Option<cmp::Ordering> {
        Some(cmp::Ordering::Greater)
	}
}

impl_op_ex!(- |x: &Interval| -> Interval {
    if let Interval::Int(a, b) = *x {
        Interval::new(-b, -a)
    } else {
        Interval::Bottom
    }
});

impl_op_ex!(
	+ |l: &Interval, r: &Interval| -> Interval {
        if let (Interval::Int(a, b), Interval::Int(c, d)) = (*l, *r) {
            Interval::new(a+c, b+d)
        } else {
            Interval::Bottom
        }
	}
);

impl_op_ex!(
    - |l: &Interval, r: &Interval| -> Interval {
        l + (-r)
    }
);

impl_op_ex!(
    * |l: &Interval, r: &Interval| -> Interval {
        if let (Interval::Int(a, b), Interval::Int(c, d)) = (*l, *r) {
            let comb = [a*c, a*d, b*c, b*d];
            Interval::new(
                *comb.iter().min().unwrap(),
                *comb.iter().max().unwrap()
            )
        } else {
            Interval::Bottom
        }
    }
);

impl_op_ex!(
	/ |l: &Interval, r: &Interval| -> Interval {
        if let (Interval::Int(a, b), Interval::Int(c, d)) = (*l, *r) {
            if c >= 1 {
                Interval::new(
                    (a/c).min(a/d),
                    (b/c).max(b/d)
                )
            } else if d <= -1 {
                Interval::new(
                    (b/c).min(b/d),
                    (a/c).max(a/d)
                )
            } else {
                let d = if d == 0 { 1 } else { d };
                let c = if c == 0 { -1 } else { c };
                (l/Interval::new(1, d)).union(l/Interval::new(c, -1))
            }
        } else {
            Interval::Bottom
        }
	}
);

impl AbstractDomain for Interval {
	fn union(self, other: Interval) -> Interval {
        if let (Interval::Int(a, b), Interval::Int(c, d)) = (self, other) {
            Interval::new(a.min(c), b.max(d))
        } else {
            Interval::Bottom
        }
	}

    fn filter_state(mut state: State<Interval>, bexp: &BExp<Interval>) -> State<Interval> {
        match bexp {
            BExp::True => state,
            BExp::False => {
                for (_, v) in state.iter_mut() {
                    *v = Interval::Bottom
                }
                state
            },
            BExp::And(a, b) => b.clone().apply(a.clone().apply(state)),
            BExp::Not(b) => {
                match b.as_ref() {
                    BExp::Lt(l, r) => BExp::Lt(r.clone(), l.clone()).apply(state),
                    BExp::Not(x) => x.clone().apply(state),
                    BExp::And(l, r) => {
                        let left_state = BExp::Not(l.clone()).apply(state.clone());
                        let right_state = BExp::Not(r.clone()).apply(state);
                        left_state.union(&right_state)
                    },
                    BExp::True => BExp::False.apply(state),
                    BExp::False => BExp::False.apply(state),
                    BExp::Eq(_, _) => state,
                }
            },
            BExp::Lt(a, b) => {
                let comb = AExp::Sub(Box::new(a.clone()), Box::new(b.clone()));
                filter_lt_zero(comb, state)
            },
            BExp::Eq(a, b) => {
                let comb_left = AExp::Sub(Box::new(a.clone()), Box::new(b.clone()));
                let comb_right = AExp::Sub(Box::new(b.clone()), Box::new(a.clone()));

                let left_side = filter_lt_zero(comb_left, state.clone());
                state = filter_lt_zero(comb_right, state);

                for (k, v) in state.iter_mut() {
                    let other = left_side.get(&k).expect("Variable not found");
                    *v = v.intersect(other);
                }

                state
            },
        }
    }
}

impl From<i64> for Interval {
    fn from(num: i64) -> Interval {
        Interval::new(num, num)
    }
}

impl fmt::Display for Interval {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Interval::Int(a, b) = *self {
            f.write_str(format!("[{a}, {b}]").as_str())
        } else {
            f.write_char('∅')
        }
	}
}

impl Interval {
    fn intersect(self, other: Interval) -> Interval {
        if let (Interval::Int(a, b), Interval::Int(c, d)) = (self, other) {
            Interval::new(a.max(c), b.min(d))
        } else {
            Interval::Bottom
        }
    }
}

enum ExpChild {
    One(Box<ExpTree>),
    Two(Box<ExpTree>, Box<ExpTree>),
    None
}

struct ExpTree {
    exp: AExp<Interval>,
    val: Interval,
    child: ExpChild
}

impl ExpTree {
    fn from_aexp(exp: AExp<Interval>) -> ExpTree {
        match exp.clone() {
            AExp::Val(_) => ExpTree { exp, val: Interval::Bottom, child: ExpChild::None},
            AExp::Var(_) => ExpTree { exp, val: Interval::Bottom, child: ExpChild::None},
            AExp::Add(a, b) => ExpTree {
                exp,
                val: Interval::Bottom,
                child: ExpChild::Two(
                    Box::new(Self::from_aexp(a.as_ref().clone())),
                    Box::new(Self::from_aexp(b.as_ref().clone()))
                )
            },
            AExp::Sub(a, b) => ExpTree {
                exp,
                val: Interval::Bottom,
                child: ExpChild::Two(
                    Box::new(Self::from_aexp(a.as_ref().clone())),
                    Box::new(Self::from_aexp(b.as_ref().clone()))
                )
            },
            AExp::Mul(a, b) => ExpTree {
                exp,
                val: Interval::Bottom,
                child: ExpChild::Two(
                    Box::new(Self::from_aexp(a.as_ref().clone())),
                    Box::new(Self::from_aexp(b.as_ref().clone()))
                )
            },
            AExp::Div(a, b) => ExpTree {
                exp,
                val: Interval::Bottom,
                child: ExpChild::Two(
                    Box::new(Self::from_aexp(a.as_ref().clone())),
                    Box::new(Self::from_aexp(b.as_ref().clone()))
                )
            },
            AExp::Min(e) => ExpTree {
                exp,
                val: Interval::Bottom,
                child: ExpChild::One(
                    Box::new(Self::from_aexp(e.as_ref().clone()))
                )
            },
        }
    }

    fn apply(&mut self, state: &State<Interval>) {
        self.val = self.exp.apply(state);
        match &mut self.child {
            ExpChild::Two(a, b) => { a.apply(state); b.apply(state); },
            ExpChild::One(c) => c.apply(state),
            _ => ()
        }
    }

    fn refine(&mut self, state: &mut State<Interval>) {
        match &mut self.child {
            ExpChild::Two(a, b) => {
                match &self.exp {
                    AExp::Add(_, _) => {
                        a.val = a.val.intersect(self.val-b.val);
                        b.val = b.val.intersect(self.val-a.val);
                        a.refine(state);
                        b.refine(state);
                    },
                    AExp::Sub(_, _) => {
                        a.val = a.val.intersect(self.val+b.val);
                        b.val = b.val.intersect(a.val-self.val);
                        a.refine(state);
                        b.refine(state);
                    },
                    AExp::Mul(_, _) => {
                        a.val = a.val.intersect(self.val/b.val);
                        b.val = b.val.intersect(self.val/a.val);
                        a.refine(state);
                        b.refine(state);
                    },
                    AExp::Div(_, _) => {
                        let s = self.val + Interval::new(-1, 1);
                        let z = self.val + Interval::new(0, 0);
                        a.val = a.val.intersect(s*b.val);
                        b.val = b.val.intersect((a.val/s).union(z));
                        a.refine(state);
                        b.refine(state);
                    },
                    _ => ()
                }
            },
            ExpChild::One(c) => {
                match &self.exp {
                    AExp::Min(_) => {
                        c.val = c.val.intersect(-self.val);
                        c.refine(state);
                    },
                    _ => ()
                }
            },
            _ => {
                if let AExp::Var(v) = &self.exp {
                    let current = state.get(v).expect("Variable doesn't exist");
                    state.set(v.clone(), self.val.intersect(current));
                }
            }
        }
    }
}


fn filter_lt_zero(comb: AExp<Interval>, mut state: State<Interval>) -> State<Interval> {
    let mut tree = ExpTree::from_aexp(comb);
    tree.apply(&state);
    tree.val = tree.val.intersect(Interval::Int(-INF, 0));
    tree.refine(&mut state);
    state
}

