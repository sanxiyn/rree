use result::{Result, Ok, Err};
use io::{Writer, WriterUtil};

enum Regexp {
    Empty,
    Literal(char),
    Concat(~[@Regexp]),
    Alternate(~[@Regexp]),
    Star(@Regexp),
    Plus(@Regexp),
    Quest(@Regexp),
    Capture(uint, @Regexp),
    LeftParen(uint),
    VerticalBar
}

enum Error {
    MissingParen,
    RepeatArgument
}

impl Regexp {
    fn is_marker(&self) -> bool {
        match *self {
            LeftParen(_) | VerticalBar => true,
            _ => false
        }
    }
    fn is_left_paren(&self) -> bool {
        match *self {
            LeftParen(_) => true,
            _ => false
        }
    }
    fn is_vertical_bar(&self) -> bool {
        match *self {
            VerticalBar => true,
            _ => false
        }
    }
}

struct Parser {
    mut stack: ~[@Regexp],
    mut ncap: uint
}

impl Parser {
    static fn new() -> Parser {
        Parser {stack: ~[], ncap: 0}
    }
    fn concat(&self) {
        let mut i = self.stack.len();
        while i > 0 && !self.stack[i-1].is_marker() {
            i -= 1;
        }
        let subs = vec::tailn(self.stack, i);
        self.stack.truncate(i);
        let re = match subs.len() {
            0 => @Empty,
            1 => subs[0],
            _ => @Concat(subs)
        };
        self.stack.push(re);
    }
    fn alternate(&self) {
        let mut i = self.stack.len();
        while i > 0 && !self.stack[i-1].is_marker() {
            i -= 1;
        }
        let subs = vec::tailn(self.stack, i);
        self.stack.truncate(i);
        let re = match subs.len() {
            0 => fail,
            1 => subs[0],
            _ => @Alternate(subs)
        };
        self.stack.push(re);
    }
    fn swap_vertical_bar(&self) -> bool {
        let n = self.stack.len();
        if n >= 2 && self.stack[n-2].is_vertical_bar() {
            self.stack[n-2] <-> self.stack[n-1];
            return true;
        }
        return false;
    }
}

fn parse(s: &str) -> Result<@Regexp, Error> {
    let p = Parser::new();
    let mut t = s;
    while t.is_not_empty() {
        let (c, u) = str::view_shift_char(t);
        t = u;
        match c {
            '(' => {
                p.ncap += 1;
                p.stack.push(@LeftParen(p.ncap));
            }
            '|' => {
                p.concat();
                if !p.swap_vertical_bar() {
                    p.stack.push(@VerticalBar);
                }
            }
            ')' => {
                p.concat();
                if p.swap_vertical_bar() {
                    p.stack.pop();
                    p.alternate();
                }
                let n = p.stack.len();
                if n < 2 {
                    return Err(MissingParen);
                }
                let sub = p.stack.pop();
                let paren = p.stack.pop();
                let re = match *paren {
                    LeftParen(cap) => @Capture(cap, sub),
                    _ => return Err(MissingParen)
                };
                p.stack.push(re);
            }
            '*' | '+' | '?' => {
                let n = p.stack.len();
                if n < 1 {
                    return Err(RepeatArgument);
                }
                let sub = p.stack.pop();
                if sub.is_marker() {
                    return Err(RepeatArgument);
                }
                let re = match c {
                    '*' => @Star(sub),
                    '+' => @Plus(sub),
                    '?' => @Quest(sub),
                    _ => fail
                };
                p.stack.push(re);
            }
            _ => {
                p.stack.push(@Literal(c));
            }
        }
    }
    p.concat();
    if p.swap_vertical_bar() {
        p.stack.pop();
        p.alternate();
    }
    if p.stack.len() != 1 {
        return Err(MissingParen);
    }
    return Ok(p.stack[0]);
}

#[cfg(test)]
impl Regexp {
    fn name(&self) -> ~str {
        match *self {
            Empty => ~"emp",
            Literal(_) => ~"lit",
            Concat(_) => ~"cat",
            Alternate(_) => ~"alt",
            Star(_) => ~"star",
            Plus(_) => ~"plus",
            Quest(_) => ~"que",
            Capture(_, _) => ~"cap",
            _ => fail
        }
    }
    fn dump(self, writer: @Writer) {
        writer.write_str(self.name());
        writer.write_char('{');
        match self {
            Literal(c) => {
                writer.write_char(c);
            }
            Concat(subs) | Alternate(subs) => {
                for subs.each |sub| {
                    sub.dump(writer);
                }
            }
            Star(sub) | Plus(sub) | Quest(sub) | Capture(_, sub) => {
                sub.dump(writer);
            }
            _ => {}
        }
        writer.write_char('}');
    }
}

#[test]
fn test_parse() {
    fn test_ok(s: &str, t: &str) {
        let result = parse(s);
        assert result.is_ok();
        let u = do io::with_str_writer |writer| {
            result.get().dump(writer);
        };
        assert t == u;
    }
    test_ok("", "emp{}");
    test_ok("a", "lit{a}");
    test_ok("ab", "cat{lit{a}lit{b}}");
    test_ok("a|b", "alt{lit{a}lit{b}}");
    test_ok("a*", "star{lit{a}}");
    test_ok("a+", "plus{lit{a}}");
    test_ok("a?", "que{lit{a}}");
    test_ok("(a)", "cap{lit{a}}");
}
