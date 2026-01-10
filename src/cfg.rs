use crate::semantics::*;
use crate::domains::*;

pub enum NodeExtension<T: AbstractDomain> {
	While(Option<Box<Node<T>>>, BExp<T>),
	IfElse(Option<Box<Node<T>>>, Option<Box<Node<T>>>),
	Normal,
}

pub struct Node<T: AbstractDomain> {
	pub statement: Box<dyn StateMap<D = T>>,
	pub next: Option<Box<Node<T>>>,
	pub ext: NodeExtension<T>,
	pub num: u64,
	pub state: Option<State<T>>,
}

impl<T: AbstractDomain> StateMap for Node<T> {
	type D = T;
	fn apply(&mut self, mut state: State<T>) -> State<T> {
		match &mut self.ext {
			NodeExtension::Normal => {
				state = self.statement.apply(state);
				self.state = Some(state.clone());
				match &mut self.next {
					Some(n) => n.apply(state),
					None => state,
				}
			}

			NodeExtension::IfElse(a, b) => {
				let ifstate = self.statement.apply(state.clone());
				self.state = Some(ifstate.clone());
				let elsestate = match b.as_mut() {
					Some(n) => n.apply(state),
					None => state,
				};
				let outstate = match a.as_mut() {
					Some(n) => n.apply(ifstate).union(&elsestate),
					None => ifstate.union(&elsestate),
				};
				match &mut self.next {
					Some(n) => n.apply(outstate),
					None => outstate,
				}
			}

			NodeExtension::While(n, b) => {
				let mut prev = state.clone();
				let mut current = prev.clone();
				current = self.statement.apply(current);
                if let Some(x) = n.as_mut() {
                    current = x.apply(current);
                }
                current = current.union(&state);

				while current != prev {
					prev = current.clone();
					current = self.statement.apply(current);
                    if let Some(x) = n.as_mut() {
                        current = x.apply(current);
                    }
                    current = current.union(&state);
				}

                self.state = Some(self.statement.apply(prev));

				let outstate = b.apply(current);
				match &mut self.next {
					Some(n) => n.apply(outstate),
					None => outstate,
				}
			}
		}
	}
}

impl<T: AbstractDomain> Default for Node<T> {
	fn default() -> Node<T> {
		Node {
			statement: Box::new(Skip::new()),
			next: None,
			ext: NodeExtension::Normal,
			num: 0,
			state: None,
		}
	}
}

pub fn execute_program<T: AbstractDomain>(mut prog: &str, graph: &mut Node<T>, init_state: State<T>) -> Result<String, String> {
    let mut ret = String::new();

    let _final_state = graph.apply(init_state);

    let mut stack = Vec::<&Node<T>>::new();
    let mut current: &Node<T> = graph;
    while let Some(node) = &current.next {
        stack.push(&node);
        current = &node;
    }
    stack.reverse();

    let mut current: Option<&Node<T>> = Some(graph);

    while let Some(node) = current.as_ref() {
        let (line, next) = prog
            .trim()
            .split_once("\n")
            .unwrap_or((prog.trim(), ""));
        prog = next;

        if line == "{" || line == "}" {
            ret.push_str(format!("{line:30}|\n").as_str());
            continue;
        }

        // println!("{line}\n");
        let state = node.state.as_ref().unwrap();
        ret.push_str(format!("{line:30}| {state}\n").as_str());

        let mut ext_stack = match &node.ext {
            NodeExtension::Normal => Vec::new(),
            NodeExtension::IfElse(a, b) => {
                let mut ret = Vec::new();
                let mut curr = a.as_deref();
                while let Some(n) = curr {
                    ret.push(n);
                    curr = n.next.as_deref();
                }
                curr = b.as_deref();
                while let Some(n) = curr {
                    ret.push(n);
                    curr = n.next.as_deref();
                }
                ret.reverse();
                ret
            },
            NodeExtension::While(x, _) => {
                let mut ret = Vec::new();
                let mut curr = x.as_deref();
                while let Some(n) = curr {
                    ret.push(n);
                    curr = n.next.as_deref();
                }
                ret.reverse();
                ret
            }
        };

        stack.append(&mut ext_stack);

        current = stack.pop();
    }

    Ok(ret)
}

