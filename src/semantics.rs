use std::collections::HashMap;
use std::fmt;
use fmt::Write;

use crate::domains::AbstractDomain;
use crate::parsing::*;

#[derive(Clone, PartialEq)]
pub struct State<T: AbstractDomain> {
	map: HashMap<String, T>,
}

impl<T: AbstractDomain> fmt::Debug for State<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.map.fmt(f)
	}
}

impl<T: AbstractDomain> fmt::Display for State<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_char('{')?;
        for (k, v) in self.iter() {
            f.write_str(format!("{}: {}, ", k, v).as_str())?;
        }
        f.write_char(8 as char)?;
        f.write_char(8 as char)?;
		f.write_char('}')?;
        Ok(())
	}
}

impl<T: AbstractDomain> State<T> {
    pub fn new() -> State<T> {
        State {
            map: HashMap::new()
        }
    }

	pub fn get(&self, var: &str) -> Option<T> {
		self.map.get(var).cloned()
	}

	pub fn set(&mut self, var: String, val: T) {
		self.map.insert(var, val);
	}

	pub fn union(&self, other: &State<T>) -> State<T> {
		let mut ret = self.clone();
		for (key, val) in other.map.iter() {
			match ret.get(&key) {
				Some(v) => ret.set(key.to_owned(), v.union(*val)),
				None => ret.set(key.to_owned(), *val),
			};
		}
		ret
	}

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, T> {
        self.map.iter()
    }

    pub fn into_iter(self) -> std::collections::hash_map::IntoIter<String, T> {
        self.map.into_iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, String, T> {
        self.map.iter_mut()
    }
}

pub trait StateMap {
	type D: AbstractDomain;
	fn apply(&mut self, state: State<Self::D>) -> State<Self::D>;
}

pub trait StateReduce {
	type D: AbstractDomain;
	fn apply(&self, state: &State<Self::D>) -> Self::D;
}

pub struct Skip<T: AbstractDomain>(std::marker::PhantomData<T>);

impl<T: AbstractDomain> Skip<T> {
	pub fn new() -> Skip<T> {
		Skip(std::marker::PhantomData::default())
	}
}

impl<T: AbstractDomain> StateMap for Skip<T> {
	type D = T;
	fn apply(&mut self, state: State<T>) -> State<T> {
		state
	}
}

#[derive(Debug, Clone)]
pub enum AExp<T: AbstractDomain> {
	Val(T),
	Var(String),
	Add(Box<AExp<T>>, Box<AExp<T>>),
	Sub(Box<AExp<T>>, Box<AExp<T>>),
	Mul(Box<AExp<T>>, Box<AExp<T>>),
	Div(Box<AExp<T>>, Box<AExp<T>>),
	Min(Box<AExp<T>>),
}

impl<T: AbstractDomain> StateReduce for AExp<T> {
	type D = T;
	fn apply(&self, state: &State<T>) -> T {
		match self {
			Self::Val(v) => v.clone(),
			Self::Var(s) => state
				.get(&s)
				.expect("I know this is not clean but it should be checked beforehand"),
			Self::Add(a, b) => a.apply(state) + b.apply(state),
			Self::Sub(a, b) => a.apply(state) - b.apply(state),
			Self::Mul(a, b) => a.apply(state) * b.apply(state),
			Self::Div(a, b) => a.apply(state) / b.apply(state),
			Self::Min(x) => -x.apply(state),
		}
	}
}

impl<T: AbstractDomain> AExp<T> {
	fn check_state(&self, state: &State<T>) -> bool {
		match self {
			Self::Val(_) => true,
			Self::Var(s) => state.get(&s).is_some(),
			Self::Add(a, b) => a.check_state(state) && b.check_state(state),
			Self::Sub(a, b) => a.check_state(state) && b.check_state(state),
			Self::Mul(a, b) => a.check_state(state) && b.check_state(state),
			Self::Div(a, b) => a.check_state(state) && b.check_state(state),
			Self::Min(x) => x.check_state(state),
		}
	}
}


impl<T: AbstractDomain> AExp<T> {
	pub fn from_str(exp: &str) -> Result<AExp<T>, String> {
		let precedence = HashMap::from([
			("^", 4_u8),
			("*", 3_u8),
			("/", 3_u8),
			("+", 2_u8),
			("-", 2_u8),
		]);

		let rpn = shunting_yard(exp, &precedence)?;
		if rpn.is_empty() {
			return Err("Invalid arithmetic expression. Shunting Yard algorithm returned an empty Vec".to_owned());
		}

		let mut stack: Vec<AExp<T>> = Vec::new();

		let ops = ["+", "-", "*", "/"];
		for token in rpn {
			if ops.contains(&token.as_str()) {
				let right = Box::new(stack.pop().ok_or("Invalid arithmetic expression. Stack prematurely empty.")?);
				let left = Box::new(stack.pop().ok_or("Invalid arithmetic expression. Stack prematurely empty.")?);

				let applied = match token.as_str() {
					"+" => Ok(AExp::Add(left, right)),
					"-" => Ok(AExp::Sub(left, right)),
					"*" => Ok(AExp::Mul(left, right)),
					"/" => Ok(AExp::Div(left, right)),
					_ => Err("Unsupported operator"),
				};

				stack.push(applied?);
			} else if let Some(unary) = token.strip_prefix('~') {
				if let Ok(n) = unary.parse::<i64>() {
					stack.push(AExp::Min(Box::new(AExp::Val(T::from(n)))));
				} else {
					stack.push(AExp::Min(Box::new(AExp::Var(unary.to_owned()))));
				}
			} else if let Ok(n) = token.parse::<i64>() {
				stack.push(AExp::Val(T::from(n)));
			} else {
				stack.push(AExp::Var(token));
			}
		}

		if stack.len() != 1 {
			return Err("Invalid arithmetic expression. Stack should only have one element at the end of the parsing.".to_owned());
		}

		Ok(stack.pop().expect("Should never happen"))
	}
}

#[derive(Debug, Clone)]
pub enum BExp<T: AbstractDomain> {
	True,
	False,
	Eq(AExp<T>, AExp<T>),
	Lt(AExp<T>, AExp<T>),
	And(Box<BExp<T>>, Box<BExp<T>>),
	Not(Box<BExp<T>>),
}

impl<T: AbstractDomain> StateMap for BExp<T> {
	type D = T;
	fn apply(&mut self, state: State<T>) -> State<T> {
		T::filter_state(state, &self)
	}
}

impl<T: AbstractDomain> BExp<T> {
	pub fn from_str(exp: &str) -> Result<BExp<T>, String> {
		let precedence = HashMap::from([
            ("!", 2_u8),
            ("&", 1_u8),
        ]);

		let rpn = shunting_yard(exp, &precedence)?;
		if rpn.is_empty() {
			return Err("Invalid boolean expression. Shunting yard returned an empty Vec".to_owned());
		}

		let mut stack: Vec<BExp<T>> = Vec::new();

		let ops = ["!", "&"];
		for token in rpn {
            if ops.contains(&token.as_str()) {
				let right = Box::new(stack.pop().ok_or("Invalid boolean expression. Stack prematurely empty.")?);
				let left = Box::new(stack.pop().ok_or("Invalid boolean expression. Stack prematurely empty.")?);

                let applied = match token.as_str() {
                    "&" => Ok(BExp::And(left, right)),
                    "!" => Ok(BExp::Not(right)),
                    _ => Err("This should never happen")
                };

                stack.push(applied?);
            } else if let Some((left, right)) = token.split_once("=") {
                stack.push(BExp::Eq(AExp::from_str(left)?, AExp::from_str(right)?));
            } else if let Some((left, right)) = token.split_once("<") {
                stack.push(BExp::Lt(AExp::from_str(left)?, AExp::from_str(right)?));
            } else if let Ok(num) = token.parse::<i64>() {
                stack.push(if num == 0 { BExp::False } else {BExp::True });
            } else if token == "n" {
                stack.push(BExp::False);
            } else {
                return Err(format!("Invalid boolean expression: {token:?}"));
            }
		}

		if stack.len() != 1 {
			return Err("Invalid arithmetic expression. Stack should only have one element at the end of the parsing.".to_owned());
		}

		Ok(stack.pop().expect("Should never happen"))
	}
}

pub struct Assignment<T: AbstractDomain> {
	pub var: String,
	pub value: AExp<T>,
}

impl<T: AbstractDomain> StateMap for Assignment<T> {
	type D = T;
	fn apply(&mut self, mut state: State<T>) -> State<T> {
		state.set(self.var.trim().to_owned(), self.value.apply(&state));
		state
	}
}

