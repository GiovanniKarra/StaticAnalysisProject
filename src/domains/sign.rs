use std::{cmp, ops, fmt};
use fmt::Write;
use impl_ops::*;

use crate::semantics::*;
use super::AbstractDomain;


#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Sign {
	Plus,
	Minus,
	Zero,
	Top,
	Bottom,
}
use Sign::*;

impl PartialOrd for Sign {
	fn partial_cmp(&self, other: &Sign) -> Option<cmp::Ordering> {
        match (self, other) {
            (Plus, Minus) => Some(cmp::Ordering::Greater),
            (Plus, Zero) => Some(cmp::Ordering::Greater),
            (Minus, Plus) => Some(cmp::Ordering::Less),
            (Minus, Zero) => Some(cmp::Ordering::Less),
            _ => None
        }
	}
}

impl_op_ex!(-|a: &Sign| -> Sign {
	match a {
		Plus => Minus,
		Minus => Plus,
		s => s.clone(),
	}
});

impl_op_ex!(
	+ |a: &Sign, b: &Sign| -> Sign {
		match (a, b) {
			(s1, s2) if s1 == s2 => s1.clone(),
			(Zero, s) => s.clone(),
			(s, Zero) => s.clone(),
			(Bottom, _) => Bottom,
			(_, Bottom) => Bottom,
			_ => Top
		}
	}
);

impl_op_ex!(-|a: &Sign, b: &Sign| -> Sign { a + (-b) });

impl_op_ex!(*|a: &Sign, b: &Sign| -> Sign {
	match (a, b) {
		(Plus, Plus) => Plus,
		(Minus, Minus) => Minus,
		(Minus, Plus) => Minus,
		(Plus, Minus) => Minus,
		(Bottom, _) => Bottom,
		(_, Bottom) => Bottom,
		(Zero, _) => Zero,
		(_, Zero) => Zero,
		_ => Top,
	}
});

impl_op_ex!(
	/ |a: &Sign, b: &Sign| -> Sign {
		match (a, b) {
			(Plus, Plus) => Plus,
			(Minus, Minus) => Minus,
			(Minus, Plus) => Minus,
			(Plus, Minus) => Minus,
			(Bottom, _) => Bottom,
			(_, Bottom) => Bottom,
			(Zero, _) => Zero,
			(_, Zero) => Bottom,
			_ => Top
		}
	}
);

impl fmt::Display for Sign {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_char(match self {
			Plus => '+',
			Minus => '-',
			Zero => '0',
			Top => '⊤',
			Bottom => '⊥',
		})
	}
}

impl AbstractDomain for Sign {
	fn union(self, other: Sign) -> Sign {
		match (self, other) {
			(Bottom, s) => s,
			(s, Bottom) => s,
			(a, b) if a == b => a,
			_ => Top,
		}
	}

    fn filter_state(mut state: State<Sign>, bexp: &BExp<Sign>) -> State<Sign> {
        match bexp {
            BExp::True => state,
            BExp::And(a, b) => Self::filter_state(Self::filter_state(state, a), b),

            BExp::False => {
                for (_, v) in state.iter_mut() {
                    *v = Sign::Bottom;
                }
                state
            },

            BExp::Not(b) => {
                state = Self::filter_state(state, b);
                for (_, v) in state.iter_mut() {
                    let val = match v {
                        Bottom => Bottom,
                        Top => Top,
                        Plus => Top,
                        Minus => Top,
                        Zero => Top
                    };
                    *v = val
                }
                state
            },

            BExp::Eq(a, b) => {
                let left = a.apply(&state);
                let right = b.apply(&state);
                if left == Bottom || right == Bottom || (left != right && left != Top && right != Top) { 
                    let _ = state.iter_mut().map(|(_, v)| *v = Bottom);
                }
                state
            },

            BExp::Lt(a, b) => {
                let left = a.apply(&state);
                let right = b.apply(&state);
                if left == Bottom
                    || right == Bottom
                    || left.partial_cmp(&right).is_some_and(|x| x == cmp::Ordering::Greater) {
                    let _ = state.iter_mut().map(|(_, v)| *v = Bottom);
                }
                state
            }
        }
    }
}

impl From<i64> for Sign {
	fn from(n: i64) -> Sign {
		if n > 0 {
			Plus
		} else if n < 0 {
			Minus
		} else {
			Zero
		}
	}
}

