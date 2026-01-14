# Static Analysis

This is a program I made for the final project of my **Software Verification** course at **UniPD**. It performs sound analysis on programs of the toy **While** language using abstract interpretation.

## The **While** language
Here's the definition of our **While** language

```
e ::= x | n | e1 op e2 | ~e ∈ AExp
b ::= 1 | 0 | e1 == e2 | e1 < e2 | b1 & b2 | n!b ∈ BExp
S ::=
    x := e
    | skip
    | S1
      S2
    | if b then
      {
          S1
      }
      else
      {
          S1
      }
    | while b do
      {
          S
      }
∈ While
```

You can find multiple `.while` file examples in `examples/`.

## Usage
The usage guide will use `cargo run --bin <bin>` but of course you can substitute with a direct call to the binaries.

### CLI
`cargo run --bin cli -- [-d <domain>] [-f <filename>] [-w <widening delay>] [-n <narrowing steps]`

If no file is selected then the program will be read from stdin.
The currently implemented domains are `interval` and `sign`. The default is the former.

### GUI
Simply run

`cargo run --bin gui`

The GUI is pretty self explanatory. It doesn't support the `sign` domain.

### Initial state
If you want to specify and initial state, you can do so in a json format at the beginning of your code, and end with `===`.


## Example

Here's the `bool_guard.while` example.

The input code is
```
{
	"x": [0,10],
	"y": [2,10],
	"z": [3,5]
}
===

if x + y < z then
{
	skip
}
else
{
	skip
}
```

and here's the output of `cargo run --bin cli -- -f examples/bool_guard.while`

```
if x + y < z then             | {y: [2, 5], x: [0, 3], z: [3, 5]}
{                             |
    skip                      | {y: [2, 5], x: [0, 3], z: [3, 5]}
}                             |
else                          | {y: [2, 10], x: [0, 10], z: [3, 5]}
{                             |
    skip                      | {y: [2, 10], x: [0, 10], z: [3, 5]}
}                             |

FINAL STATE : {y: [2, 10], x: [0, 10], z: [3, 5]}
```
