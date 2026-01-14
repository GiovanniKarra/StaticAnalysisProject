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

impl<T: AbstractDomain> Node<T> {
	fn apply(&mut self, mut state: State<T>, config: &ExecConfig<T>) -> State<T> {
		match &mut self.ext {
			NodeExtension::Normal => {
				state = self.statement.apply(state);
				self.state = Some(state.clone());
				match &mut self.next {
					Some(n) => n.apply(state, config),
					None => state,
				}
			}

			NodeExtension::IfElse(a, b) => {
				let ifstate = self.statement.apply(state.clone());
				self.state = Some(ifstate.clone());
				let elsestate = match b.as_mut() {
					Some(n) => n.apply(state, config),
					None => state,
				};
				let outstate = match a.as_mut() {
					Some(n) => n.apply(ifstate, config).union(&elsestate),
					None => ifstate.union(&elsestate),
				};
				match &mut self.next {
					Some(n) => n.apply(outstate, config),
					None => outstate,
				}
			}

			NodeExtension::While(n, b) => {
                let mut iter_count = 0;

				let mut prev = state.clone();
				let mut current = prev.clone();

                let mut f = |mut curr, pre: &State<T>, count: &mut i64| {
                    curr = self.statement.apply(curr);
                    if let Some(x) = n.as_mut() {
                        curr = x.apply(curr, config);
                    }
                    curr = curr.union(&state);
                    if *count >= config.widening_delay.into() {
                        curr.iter_mut().for_each(|(k, v)| *v = pre.get(k).unwrap_or(*v).widen(*v));
                    }
                    *count += 1;
                    curr
                };

                current = f(current, &prev, &mut iter_count);
                iter_count += 1;

				while current != prev {
					prev = current.clone();
                    current = f(current, &prev, &mut iter_count);
                    iter_count += 1;
				}

                // current = self.statement.apply(current);
                for _ in 0..config.narrowing_steps {
                    iter_count = -1;
					prev = current.clone();
                    current = f(current, &prev, &mut iter_count);
                    current.iter_mut().for_each(|(k, v)| *v = prev.get(k).unwrap_or(*v).narrow(*v));
                }

                prev = current.clone();
                self.state = Some(self.statement.apply(prev));

				let outstate = b.apply(current);
				match &mut self.next {
					Some(n) => n.apply(outstate, config),
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

pub struct ExecConfig<T: AbstractDomain> {
    pub widening_delay: u32,
    pub narrowing_steps: u32,
    pub init_state: Option<State<T>>
}

pub fn execute_program<T: AbstractDomain>(mut prog: &str, graph: &mut Node<T>, mut config: ExecConfig<T>) -> Result<String, String> {
    let mut ret = String::new();

    let init_state = config.init_state.take().unwrap_or(State::new());
    let final_state = graph.apply(init_state, &config);

    let mut stack = Vec::<&Node<T>>::new();
    let mut current: &Node<T> = graph;
    while let Some(node) = &current.next {
        stack.push(&node);
        current = &node;
    }
    stack.reverse();

    let mut current: Option<&Node<T>> = Some(graph);
    let mut indent = 0;

    while let Some(node) = current.as_ref() {
        let (line, next) = prog
            .trim()
            .split_once("\n")
            .unwrap_or((prog.trim(), ""));
        prog = next;

        if line == "}" {
            indent -= 1
        }
        let mut line_print = String::new();
        for _ in 0..indent {
            line_print.push_str("    ");
        }
        line_print.push_str(line);
        if line == "{" {
            indent += 1;
        }

        if line == "{" || line == "}" {
            ret.push_str(format!("{line_print:30}|\n").as_str());
            continue;
        }

        let state = node.state.as_ref().unwrap();
        ret.push_str(format!("{line_print:30}| {state}\n").as_str());

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
    
    while indent > 0 {
        indent -= 1;
        let mut s = String::new();
        for _ in 0..indent {
            s.push_str("    ");
        }
        s.push('}');
        ret.push_str(format!("{s:30}|\n").as_str());
        continue;
    }

    ret.push_str(&format!("\nFINAL STATE : {final_state}"));
    Ok(ret)
}

pub fn execute<T: AbstractDomain>(prog: &str, init_state: Option<State<T>>, wid: Option<u32>, nar: Option<u32>) -> String {
	let mut graph = match crate::parsing::parse_program::<T>(prog) {
        Ok(n) => n.unwrap_or_default(),
        Err(e) => return e
    };

    let config = ExecConfig {
        widening_delay: wid.unwrap_or(interval::INF as u32),
        narrowing_steps: nar.unwrap_or(0),
        init_state
    };

    let out = execute_program(prog, &mut graph, config).unwrap_or_else(|e| e);

    out
}
