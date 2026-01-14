use std::ops::{Add, Div, Mul, Neg, Sub};
use std::fmt;

use crate::semantics::*;

pub mod sign;
pub mod interval;

pub use sign::Sign;
pub use interval::Interval;

pub trait AbstractDomain:
	Add<Output = Self>
	+ Sub<Output = Self>
	+ Mul<Output = Self>
	+ Div<Output = Self>
	+ Neg<Output = Self>
    + PartialOrd
	+ PartialEq
	+ Sized
	+ Copy
	+ Clone
	+ fmt::Debug
	+ fmt::Display
	+ From<i64>
	+ 'static
{
	fn union(self, other: Self) -> Self;
	fn filter_state(state: State<Self>, bexp: &BExp<Self>) -> State<Self>;
    fn widen(self, other: Self) -> Self {
        self.union(other)
    }
    fn narrow(self, other: Self) -> Self {
        other
    }
}

