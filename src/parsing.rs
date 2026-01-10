use std::collections::HashMap;

use crate::domains::AbstractDomain;
use crate::semantics::*;
use crate::cfg::*;


fn tokenize(exp: &str, mut ops: Vec<char>) -> Vec<String> {
	let mut tokens = Vec::new();

	let mut token = String::new();
	ops.extend_from_slice(&['(', ')']);

	for c in exp.chars().filter(|c| !c.is_whitespace()) {
		if ops.contains(&c) {
			if token.len() > 0 {
				tokens.push(token.clone());
				token.clear();
			}
			tokens.push(c.to_string());
		} else {
			token.push(c);
		}
	}

	if token.len() > 0 {
		tokens.push(token);
	}

	tokens
}

pub fn shunting_yard(exp: &str, precedence: &HashMap<&str, u8>) -> Result<Vec<String>, String> {
	let ops = precedence
		.keys()
		.map(|x| x.chars().next().unwrap_or(' '))
		.collect();
	let tokens = tokenize(exp, ops);

	let mut output = Vec::with_capacity(tokens.len());
	let mut op_stack = Vec::<String>::with_capacity(tokens.len() / 2);

	for token in tokens {
		if let Some(p) = precedence.get(&token.as_str()).cloned() {
			while let Some(o2) = op_stack.last().map(String::as_str)
				&& o2 != "(" && (precedence[o2] > p || (p == precedence[o2] && p < 4))
			{
				output.push(op_stack.pop().expect("Should never happend"));
			}
			op_stack.push(token);
		} else if token == "(" {
			op_stack.push(token);
		} else if token == ")" {
			while let o = op_stack.pop().ok_or("Mismatched parenthesis".to_owned())?
				&& o != "("
			{
				output.push(o);
			}
		} else {
			output.push(token);
		}
	}

	while let Some(o) = op_stack.pop() {
		if o == "(" {
			return Err("Mismatched parenthesis".to_owned());
		}
		output.push(o);
	}

	Ok(output)
}

fn get_block_content(prog: &str) -> Option<&str> {
    // println!("look for block in {prog}");

	let start = prog.find('{')?;
	let mut end = start;
	let mut count = 1;
	for (i, c) in prog.char_indices() {
		if i <= start {
			continue;
		}

		if c == '{' {
			count += 1;
		} else if c == '}' {
			count -= 1;
		}

        if count == 0 {
			end = i;
			break;
		}
	}

    // println!("found {}", &prog[(start+1)..end]);
	match count {
		0 => Some(&prog[(start+1)..end]),
		_ => None,
	}
}

pub fn parse_program<T: AbstractDomain>(prog: &str) -> Result<Option<Node<T>>, String> {
    // println!("parse {prog}");

	if prog.len() == 0 {
		return Ok(None);
	}

	let mut node: Node<T>;

	let (current, mut next) = prog.trim().split_once("\n").unwrap_or((prog.trim(), ""));

	node = match current.split_once(" ") {
		Some(("while", _)) => {
			let whileblock = get_block_content(next)
				.ok_or("while statement should be followed by correct block")?;
			let whilenode = parse_program::<T>(whileblock)?;

            let brack_idx = next.find("{")
                .ok_or("Shouldn't happen".to_owned())?;
			next = &next[whileblock.len()+brack_idx+2..];

			let guard = current
				.trim()
				.strip_prefix("while")
				.ok_or("Failed to parse while guard")?
				.strip_suffix("do")
				.ok_or("Failed to parse while guard")?;

			let bexp = BExp::from_str(guard)?;
			let not = BExp::Not(Box::new(bexp.clone()));

			Node {
				statement: Box::new(bexp),
				ext: NodeExtension::While(whilenode.map(Box::new), not),
				..Default::default()
			}
		}
		Some(("if", _)) => {
			let ifblock = get_block_content(next)
				.ok_or("if statement should be followed by correct block")?;
			let elsestart = next
				.find("else")
				.ok_or("if block should be followed by else block")?;

            let elseblock = get_block_content(&next[elsestart..])
				.ok_or("else statement should be followed by correct block")?;

			let ifnode = parse_program::<T>(ifblock)?;
			let elsenode = parse_program::<T>(elseblock)?;

			let guard = current
				.trim()
				.strip_prefix("if")
				.ok_or("Failed to parse if guard")?
				.strip_suffix("then")
				.ok_or("Failed to parse if guard")?;

			let bexp = BExp::from_str(guard)?;
			let not = BExp::Not(Box::new(bexp.clone()));

			let elsenode = Some(Node {
				statement: Box::new(not),
				next: elsenode.map(Box::new),
				..Default::default()
			});

            let brack_idx = next[1..].find("{")
                .ok_or("Shouldn't happen".to_owned())?;
            // println!("before: {next}");
			next = &next[brack_idx+elseblock.len()+3..];
            // println!("after: {next}");

			Node {
				statement: Box::new(BExp::True),
				ext: NodeExtension::IfElse(ifnode.map(Box::new), elsenode.map(Box::new)),
				..Default::default()
			}
		}
		_ => {
			if let Some((var, exp)) = current.split_once(":=") {
				Node {
					statement: Box::new(Assignment {
						var: var.to_owned(),
						value: AExp::from_str(exp)?,
					}),
					..Default::default()
				}
			} else if current == "skip" {
				Node {
					statement: Box::new(Skip::new()),
					..Default::default()
				}
			} else {
				return Err(format!("Invalid statement: {current}"));
			}
		}
	};

	let next = parse_program::<T>(next);

	node.next = next?.map(|mut x| {
		x.num = node.num + 1;
		Box::new(x)
	});

	Ok(Some(node))
}
