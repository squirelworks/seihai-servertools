//! LevelFiller's object language: the script of a complex (`<name>*`)
//! theme object, which builds the object it places (Graal_analysis
//! `tools/levelfiller.md`, "Object language").
//!
//! ```text
//! setsize(5,4);                 // the object's size, before any add
//! n = r(2,4);                   // variables; + - * / ^, / is whole division
//! if (n > 2 and not c(50%,1)) { add(<flower>, n); } // n copies at random
//! add(*AB, 0, 0);               // one tile (or object) at a spot
//! for (i = 0, 4) { add(<rock>, i, 3); }
//! while (n > 0) { n = n - 1; }
//! setworkarea(1,1,3,2); ... unsetworkarea;
//! return(<bush>);               // place another object instead
//! msg('done' & 10 'next line'); debug(n); debug(*);
//! ```
//!
//! Choices this port made where the teardown is silent: an object is named
//! `<name>`, `"name"` or a bare word that is not a variable; `*AA` is one
//! tile. `add(obj, n)` tries up to three random spots per copy and puts the
//! copy only where every cell it would draw is still empty in the object
//! being built; `add(obj, x, y)` draws over whatever is there. `for (v = a,
//! b)` runs `v` from `a` to `b` inclusive. Conditions are true when not 0.
//! Loops stop after 100,000 rounds.

use crate::generate::DelphiRandom;
use std::collections::HashMap;

/// The most rounds a `while` or `for` may run.
const MAX_ROUNDS: usize = 100_000;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Word(String),
    /// `<name>`, `"name"` or `*AA`.
    Obj(String),
    Tile([u8; 2]),
    Sym(&'static str),
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let s: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        // After a value, `*` multiplies and `<` compares; elsewhere they
        // open a tile (`*AA`) or an object name (`<name>`).
        let after_value = matches!(
            out.last(),
            Some(
                Tok::Num(_)
                    | Tok::Str(_)
                    | Tok::Word(_)
                    | Tok::Obj(_)
                    | Tok::Tile(_)
                    | Tok::Sym(")")
            )
        );
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && s.get(i + 1) == Some(&'/') {
            while i < s.len() && s[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && s.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < s.len() && !(s[i] == '*' && s[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c.is_ascii_digit() || (c == '.' && s.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let start = i;
            while i < s.len() && (s[i].is_ascii_digit() || s[i] == '.') {
                i += 1;
            }
            let t: String = s[start..i].iter().collect();
            out.push(Tok::Num(
                t.parse().map_err(|_| format!("{t}: not a number"))?,
            ));
        } else if c == '\'' {
            let start = i + 1;
            i = start;
            while i < s.len() && s[i] != '\'' {
                i += 1;
            }
            if i >= s.len() {
                return Err("Unterminated String.".into());
            }
            out.push(Tok::Str(s[start..i].iter().collect()));
            i += 1;
        } else if c == '"' {
            let start = i + 1;
            i = start;
            while i < s.len() && s[i] != '"' {
                i += 1;
            }
            if i >= s.len() {
                return Err("Unterminated String.".into());
            }
            out.push(Tok::Obj(s[start..i].iter().collect()));
            i += 1;
        } else if c == '*'
            && !after_value
            && s.get(i + 1)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/')
            && s.get(i + 2).is_some()
        {
            out.push(Tok::Tile([s[i + 1] as u8, s[i + 2] as u8]));
            i += 3;
        } else if c == '<'
            && !after_value
            && (s.get(i + 1).is_some_and(|c| c.is_alphabetic() || *c == '_'))
            && object_end(&s, i).is_some()
        {
            let end = object_end(&s, i).unwrap_or(i);
            out.push(Tok::Obj(s[i + 1..end].iter().collect()));
            i = end + 1;
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_') {
                i += 1;
            }
            out.push(Tok::Word(
                s[start..i].iter().collect::<String>().to_ascii_lowercase(),
            ));
        } else {
            let two: String = s[i..(i + 2).min(s.len())].iter().collect();
            let sym = ["==", "<=", ">=", "<>", "!="]
                .into_iter()
                .find(|t| *t == two)
                .or_else(|| {
                    [
                        "(", ")", "{", "}", ",", ";", "=", "+", "-", "*", "/", "^", "<", ">", "%",
                        "&",
                    ]
                    .into_iter()
                    .find(|t| t.starts_with(c))
                })
                .ok_or(format!("unexpected {c:?}"))?;
            i += sym.len();
            out.push(Tok::Sym(sym));
        }
    }
    Ok(out)
}

/// `<name>`: the closing `>` before the line, `;`, `,` or `)` ends.
fn object_end(s: &[char], open: usize) -> Option<usize> {
    s.iter()
        .enumerate()
        .skip(open + 1)
        .take_while(|(_, c)| !matches!(c, '\n' | ';' | ',' | ')' | '<'))
        .find(|(_, c)| **c == '>')
        .map(|(i, _)| i)
}

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Str(String),
    Var(String),
    Obj(String),
    Tile([u8; 2]),
    Star,
    Un(&'static str, Box<Expr>),
    Bin(&'static str, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone)]
enum Stmt {
    Assign(String, Expr),
    Call(String, Vec<Expr>),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    For(String, Expr, Expr, Option<Expr>, Vec<Stmt>),
}

struct Parser {
    toks: Vec<Tok>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.at).cloned();
        self.at += 1;
        t
    }
    fn is(&self, sym: &str) -> bool {
        matches!(self.peek(), Some(Tok::Sym(s)) if *s == sym)
    }
    fn eat(&mut self, sym: &str) -> bool {
        let yes = self.is(sym);
        if yes {
            self.at += 1;
        }
        yes
    }
    fn expect(&mut self, sym: &str) -> Result<(), String> {
        if self.eat(sym) {
            Ok(())
        } else {
            Err(format!("expected {sym:?}, found {:?}", self.peek()))
        }
    }
    fn word(&self) -> Option<&str> {
        match self.peek() {
            Some(Tok::Word(w)) => Some(w),
            _ => None,
        }
    }

    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        if self.eat("{") {
            let mut body = Vec::new();
            while !self.eat("}") {
                if self.peek().is_none() {
                    return Err("a block has no closing }".into());
                }
                body.extend(self.statement()?);
            }
            Ok(body)
        } else {
            Ok(self.statement()?.into_iter().collect())
        }
    }

    fn statement(&mut self) -> Result<Option<Stmt>, String> {
        if self.eat(";") {
            return Ok(None);
        }
        let Some(Tok::Word(w)) = self.next() else {
            return Err(format!(
                "expected a statement, found {:?}",
                self.toks.get(self.at - 1)
            ));
        };
        let stmt = match w.as_str() {
            "if" => {
                self.expect("(")?;
                let c = self.expr()?;
                self.expect(")")?;
                let then = self.block()?;
                let other = if self.word() == Some("else") {
                    self.at += 1;
                    self.block()?
                } else {
                    Vec::new()
                };
                return Ok(Some(Stmt::If(c, then, other)));
            }
            "while" => {
                self.expect("(")?;
                let c = self.expr()?;
                self.expect(")")?;
                return Ok(Some(Stmt::While(c, self.block()?)));
            }
            "for" => {
                self.expect("(")?;
                let Some(Tok::Word(v)) = self.next() else {
                    return Err("for needs a variable".into());
                };
                self.expect("=")?;
                let a = self.expr()?;
                self.expect(",")?;
                let b = self.expr()?;
                let step = if self.eat(",") {
                    Some(self.expr()?)
                } else {
                    None
                };
                self.expect(")")?;
                return Ok(Some(Stmt::For(v, a, b, step, self.block()?)));
            }
            _ if self.eat("=") || self.eat("==") => Stmt::Assign(w, self.expr()?),
            _ if self.is("(") => {
                let Expr::Call(name, args) = self.call(w)? else {
                    unreachable!()
                };
                Stmt::Call(name, args)
            }
            _ => Stmt::Call(w, Vec::new()),
        };
        if !self.eat(";") && !self.is("}") && self.peek().is_some() {
            return Err(format!("expected \";\", found {:?}", self.peek()));
        }
        Ok(Some(stmt))
    }

    fn call(&mut self, name: String) -> Result<Expr, String> {
        self.expect("(")?;
        let mut args = Vec::new();
        if !self.eat(")") {
            loop {
                args.push(self.expr()?);
                if self.eat(")") {
                    break;
                }
                self.expect(",")?;
            }
        }
        Ok(Expr::Call(name, args))
    }

    fn expr(&mut self) -> Result<Expr, String> {
        self.binary(0)
    }

    fn binary(&mut self, level: usize) -> Result<Expr, String> {
        const LEVELS: [&[&str]; 6] = [
            &["or"],
            &["and"],
            &["=", "==", "<>", "!=", "<", ">", "<=", ">="],
            &["+", "-", "&"],
            &["*", "/"],
            &["^"],
        ];
        if level == LEVELS.len() {
            return self.unary();
        }
        let mut left = self.binary(level + 1)?;
        loop {
            let op = match self.peek() {
                Some(Tok::Sym(s)) => LEVELS[level].iter().find(|o| *o == s).copied(),
                Some(Tok::Word(w)) => LEVELS[level].iter().find(|o| *o == w).copied(),
                // `'a' & 10 'b'`: text written next to text joins it.
                Some(Tok::Str(_)) if level == 3 => Some("&"),
                _ => None,
            };
            let Some(op) = op else { break };
            if !matches!(self.peek(), Some(Tok::Str(_))) {
                self.at += 1;
            }
            let right = self.binary(level + 1)?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat("-") {
            return Ok(Expr::Un("-", Box::new(self.unary()?)));
        }
        if self.word() == Some("not") {
            self.at += 1;
            return Ok(Expr::Un("not", Box::new(self.unary()?)));
        }
        match self.next() {
            Some(Tok::Num(n)) => {
                // `5%` in an argument is the number 5 marked as a share.
                if self.eat("%") {
                    return Ok(Expr::Un("%", Box::new(Expr::Num(n))));
                }
                Ok(Expr::Num(n))
            }
            Some(Tok::Str(s)) => Ok(Expr::Str(s)),
            Some(Tok::Obj(o)) => Ok(Expr::Obj(o)),
            Some(Tok::Tile(t)) => Ok(Expr::Tile(t)),
            Some(Tok::Sym("*")) => Ok(Expr::Star),
            Some(Tok::Sym("(")) => {
                let e = self.expr()?;
                self.expect(")")?;
                Ok(e)
            }
            Some(Tok::Word(w)) if self.is("(") => self.call(w),
            Some(Tok::Word(w)) => Ok(Expr::Var(w)),
            t => Err(format!("unexpected {t:?}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Val {
    Num(f64),
    Str(String),
    Obj(String),
    Tile(u16),
    Share(f64),
}

impl Val {
    fn num(&self) -> f64 {
        match self {
            Val::Num(n) | Val::Share(n) => *n,
            Val::Str(s) => s.trim().parse().unwrap_or(0.0),
            _ => 0.0,
        }
    }
    fn text(&self) -> String {
        match self {
            Val::Str(s) | Val::Obj(s) => s.clone(),
            Val::Num(n) => fmt_num(*n),
            Val::Share(n) => format!("{}%", fmt_num(*n)),
            Val::Tile(t) => {
                String::from_utf8_lossy(&seihai_core::assets::tile_code(*t)).into_owned()
            }
        }
    }
}

fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// A plain picture a script can add: `width` by `height` cells, `None`
/// transparent.
#[derive(Debug, Clone, PartialEq)]
pub struct Stamp {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Option<u16>>,
}

/// What a script built.
#[derive(Debug, Clone, PartialEq)]
pub enum Built {
    /// The canvas it drew.
    Stamp(Stamp),
    /// `return(obj)`: place this object instead.
    Return(String),
    /// `return(*AA)`: place this tile instead.
    Tile(u16),
}

enum Flow {
    Next,
    Return(Built),
}

struct Run<'a> {
    vars: HashMap<String, Val>,
    canvas: Option<Stamp>,
    area: Option<(usize, usize, usize, usize)>,
    rng: &'a mut DelphiRandom,
    lookup: &'a dyn Fn(&str) -> Option<Stamp>,
    log: &'a mut Vec<String>,
}

impl Run<'_> {
    fn eval(&mut self, e: &Expr) -> Result<Val, String> {
        Ok(match e {
            Expr::Num(n) => Val::Num(*n),
            Expr::Str(s) => Val::Str(s.clone()),
            Expr::Obj(o) => Val::Obj(o.clone()),
            Expr::Tile(t) => Val::Tile(seihai_core::assets::tile_id(*t)),
            Expr::Star => Val::Str("*".into()),
            Expr::Var(v) => match self.vars.get(v) {
                Some(x) => x.clone(),
                None => Val::Obj(v.clone()),
            },
            Expr::Un(op, x) => {
                let v = self.eval(x)?;
                match *op {
                    "-" => Val::Num(-v.num()),
                    "%" => Val::Share(v.num()),
                    _ => Val::Num(f64::from(u8::from(v.num() == 0.0))),
                }
            }
            Expr::Bin(op, a, b) => {
                let (a, b) = (self.eval(a)?, self.eval(b)?);
                let (x, y) = (a.num(), b.num());
                let flag = |c: bool| Val::Num(f64::from(u8::from(c)));
                match *op {
                    "&" => {
                        // A number next to text is a character code.
                        let piece = |v: &Val| match v {
                            Val::Num(n) => char::from_u32(*n as u32)
                                .map(String::from)
                                .unwrap_or_default(),
                            other => other.text(),
                        };
                        Val::Str(piece(&a) + &piece(&b))
                    }
                    "+" => Val::Num(x + y),
                    "-" => Val::Num(x - y),
                    "*" => Val::Num(x * y),
                    "/" => {
                        if y as i64 == 0 {
                            return Err("division by 0".into());
                        }
                        Val::Num(((x as i64) / (y as i64)) as f64)
                    }
                    "^" => Val::Num(x.powf(y)),
                    "=" | "==" => flag(
                        a.text() == b.text() || (x == y && !matches!(a, Val::Str(_) | Val::Obj(_))),
                    ),
                    "<>" | "!=" => flag(x != y),
                    "<" => flag(x < y),
                    ">" => flag(x > y),
                    "<=" => flag(x <= y),
                    ">=" => flag(x >= y),
                    "and" => flag(x != 0.0 && y != 0.0),
                    _ => flag(x != 0.0 || y != 0.0),
                }
            }
            Expr::Call(name, args) => {
                let vals = args
                    .iter()
                    .map(|a| self.eval(a))
                    .collect::<Result<Vec<_>, _>>()?;
                match (name.as_str(), vals.as_slice()) {
                    ("r", [a, b]) => {
                        let (lo, hi) = (a.num().min(b.num()) as i64, a.num().max(b.num()) as i64);
                        Val::Num((lo + i64::from(self.rng.below((hi - lo + 1) as u32))) as f64)
                    }
                    ("c", [p, n]) => {
                        let Val::Share(p) = p else {
                            return Err("Parameter is not a percentage.".into());
                        };
                        let p = *p;
                        let hits = (0..n.num().max(0.0) as u32)
                            .filter(|_| f64::from(self.rng.below(10_000)) < p * 100.0)
                            .count();
                        Val::Num(hits as f64)
                    }
                    ("round", [x]) => Val::Num(x.num().round()),
                    _ => return Err(format!("{name}: unknown function or wrong arguments")),
                }
            }
        })
    }

    fn stamp_of(&self, v: &Val) -> Result<Stamp, String> {
        match v {
            Val::Tile(t) => Ok(Stamp {
                width: 1,
                height: 1,
                cells: vec![Some(*t)],
            }),
            Val::Obj(name) | Val::Str(name) => {
                (self.lookup)(name).ok_or(format!("{name}: no such plain object"))
            }
            _ => Err(format!("{}: not an object", v.text())),
        }
    }

    fn draw(&mut self, s: &Stamp, x: usize, y: usize, only_empty: bool) -> bool {
        let Some(c) = self.canvas.as_mut() else {
            return false;
        };
        if x + s.width > c.width || y + s.height > c.height {
            return false;
        }
        let cells = || (0..s.height).flat_map(move |dy| (0..s.width).map(move |dx| (dx, dy)));
        if only_empty
            && cells().any(|(dx, dy)| {
                s.cells[dy * s.width + dx].is_some()
                    && c.cells[(y + dy) * c.width + x + dx].is_some()
            })
        {
            return false;
        }
        for (dx, dy) in cells() {
            if let Some(t) = s.cells[dy * s.width + dx] {
                c.cells[(y + dy) * c.width + x + dx] = Some(t);
            }
        }
        true
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> Result<Flow, String> {
        let vals = args
            .iter()
            .map(|a| self.eval(a))
            .collect::<Result<Vec<_>, _>>()?;
        let whole = |v: &Val| v.num().max(0.0) as usize;
        match (name, vals.as_slice()) {
            ("setsize", [w, h]) => {
                let (w, h) = (whole(w).clamp(1, 64), whole(h).clamp(1, 64));
                self.canvas = Some(Stamp {
                    width: w,
                    height: h,
                    cells: vec![None; w * h],
                });
                self.area = None;
            }
            ("setworkarea", [x, y, w, h]) => {
                self.area = Some((whole(x), whole(y), whole(w), whole(h)))
            }
            ("unsetworkarea", []) => self.area = None,
            ("add", [obj, n]) => {
                let stamp = self.stamp_of(obj)?;
                let canvas = self.canvas.as_ref().ok_or("add before setsize")?;
                let (ax, ay, aw, ah) = self.area.unwrap_or((0, 0, canvas.width, canvas.height));
                let (sx, sy) = (
                    aw.saturating_sub(stamp.width) + 1,
                    ah.saturating_sub(stamp.height) + 1,
                );
                for _ in 0..whole(n) {
                    for _ in 0..3 {
                        let y = ay + self.rng.below(sy as u32) as usize;
                        let x = ax + self.rng.below(sx as u32) as usize;
                        if x + stamp.width <= ax + aw
                            && y + stamp.height <= ay + ah
                            && self.draw(&stamp, x, y, true)
                        {
                            break;
                        }
                    }
                }
            }
            ("add", [obj, x, y]) => {
                let stamp = self.stamp_of(obj)?;
                if self.canvas.is_none() {
                    return Err("add before setsize".into());
                }
                self.draw(&stamp, whole(x), whole(y), false);
            }
            ("return", [v]) => {
                return Ok(Flow::Return(match v {
                    Val::Tile(t) => Built::Tile(*t),
                    other => Built::Return(other.text()),
                }))
            }
            ("debug", [Val::Str(s)]) if s == "*" => {
                let mut vars: Vec<_> = self
                    .vars
                    .iter()
                    .map(|(k, v)| format!("{k}={}", v.text()))
                    .collect();
                vars.sort();
                self.log.push(vars.join(", "));
            }
            ("debug", [v]) => {
                let label = match &args[0] {
                    Expr::Var(n) => n.clone(),
                    _ => "value".into(),
                };
                self.log.push(format!("{label}={}", v.text()));
            }
            ("msg", [v]) => self.log.push(v.text()),
            _ => return Err(format!("{name}: unknown command or wrong arguments")),
        }
        Ok(Flow::Next)
    }

    fn run(&mut self, body: &[Stmt], rounds: &mut usize) -> Result<Flow, String> {
        for s in body {
            let flow = match s {
                Stmt::Assign(v, e) => {
                    let x = self.eval(e)?;
                    self.vars.insert(v.clone(), x);
                    Flow::Next
                }
                Stmt::Call(n, a) => self.call(n, a)?,
                Stmt::If(c, t, f) => {
                    if self.eval(c)?.num() != 0.0 {
                        self.run(t, rounds)?
                    } else {
                        self.run(f, rounds)?
                    }
                }
                Stmt::While(c, b) => loop {
                    if self.eval(c)?.num() == 0.0 {
                        break Flow::Next;
                    }
                    *rounds += 1;
                    if *rounds > MAX_ROUNDS {
                        return Err("a loop ran 100,000 rounds; stopped".into());
                    }
                    if let Flow::Return(b) = self.run(b, rounds)? {
                        break Flow::Return(b);
                    }
                },
                Stmt::For(v, a, b, step, body) => {
                    let (mut i, end) = (self.eval(a)?.num(), self.eval(b)?.num());
                    let step = match step {
                        Some(s) => self.eval(s)?.num(),
                        None if end < i => -1.0,
                        None => 1.0,
                    };
                    if step == 0.0 {
                        return Err("for with a step of 0".into());
                    }
                    let mut out = Flow::Next;
                    while (step > 0.0 && i <= end) || (step < 0.0 && i >= end) {
                        *rounds += 1;
                        if *rounds > MAX_ROUNDS {
                            return Err("a loop ran 100,000 rounds; stopped".into());
                        }
                        self.vars.insert(v.clone(), Val::Num(i));
                        if let Flow::Return(b) = self.run(body, rounds)? {
                            out = Flow::Return(b);
                            break;
                        }
                        i += step;
                    }
                    out
                }
            };
            if let Flow::Return(b) = flow {
                return Ok(Flow::Return(b));
            }
        }
        Ok(Flow::Next)
    }
}

/// Check a script's syntax without running it.
pub fn check(script: &str) -> Result<(), String> {
    parse(script).map(|_| ())
}

fn parse(script: &str) -> Result<Vec<Stmt>, String> {
    let mut p = Parser {
        toks: lex(script)?,
        at: 0,
    };
    let mut body = Vec::new();
    while p.peek().is_some() {
        body.extend(p.statement()?);
    }
    Ok(body)
}

/// Run a complex object's script. `lookup` gives a plain object by name;
/// `debug` and `msg` lines go to `log`.
pub fn build(
    script: &str,
    rng: &mut DelphiRandom,
    lookup: &dyn Fn(&str) -> Option<Stamp>,
    log: &mut Vec<String>,
) -> Result<Built, String> {
    let body = parse(script)?;
    let mut run = Run {
        vars: HashMap::new(),
        canvas: None,
        area: None,
        rng,
        lookup,
        log,
    };
    match run.run(&body, &mut 0)? {
        Flow::Return(b) => Ok(b),
        Flow::Next => run
            .canvas
            .map(Built::Stamp)
            .ok_or("the script never called setsize".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flower(name: &str) -> Option<Stamp> {
        (name == "flower").then(|| Stamp {
            width: 1,
            height: 2,
            cells: vec![Some(5), None],
        })
    }

    fn go(script: &str) -> (Result<Built, String>, Vec<String>) {
        let mut log = Vec::new();
        let r = build(script, &mut DelphiRandom(4), &flower, &mut log);
        (r, log)
    }

    #[test]
    fn builds_a_canvas() {
        let (r, log) = go(
            "setsize(4,3); // size\n/* tiles */ add(*AB,0,0); add(*AC,3,2);\nfor (i=1,2) { add(<flower>,i,1); }\nn = 7/2; debug(n); msg('a'&10'b');",
        );
        let Built::Stamp(s) = r.unwrap() else {
            panic!()
        };
        assert_eq!((s.width, s.height), (4, 3));
        assert_eq!(s.cells[0], Some(1));
        assert_eq!(s.cells[11], Some(2));
        assert_eq!((s.cells[5], s.cells[6]), (Some(5), Some(5)));
        assert_eq!(log, ["n=3", "a\nb"]);
    }

    #[test]
    fn random_adds_stay_in_the_work_area_and_do_not_overlap() {
        let (r, _) = go("setsize(6,6); setworkarea(2,2,2,2); add(*AB,10); unsetworkarea;");
        let Built::Stamp(s) = r.unwrap() else {
            panic!()
        };
        for (i, c) in s.cells.iter().enumerate() {
            let (x, y) = (i % 6, i / 6);
            if c.is_some() {
                assert!((2..4).contains(&x) && (2..4).contains(&y));
            }
        }
        assert!(s.cells.iter().filter(|c| c.is_some()).count() <= 4);
    }

    #[test]
    fn control_flow_and_return() {
        let (r, _) =
            go("n = 0; while (n < 5) { n = n + 1; } if (n == 5 and not 0) { return(<rock>); }");
        assert_eq!(r.unwrap(), Built::Return("rock".into()));
        let (r, _) = go("if (2^3 = 9) { return(*AB); } else { return(*AC); }");
        assert_eq!(r.unwrap(), Built::Tile(2));
        let (r, _) = go("x = r(3,3) + round(2.6) + c(100%,4); setsize(x,1);");
        let Built::Stamp(s) = r.unwrap() else {
            panic!()
        };
        assert_eq!(s.width, 10);
    }

    #[test]
    fn reports_errors() {
        assert_eq!(go("msg('oops);").0.unwrap_err(), "Unterminated String.");
        assert_eq!(
            go("x = c(5,1);").0.unwrap_err(),
            "Parameter is not a percentage."
        );
        assert!(go("add(*AB,1);").0.is_err(), "add before setsize");
        assert!(go("while (1) { }").0.unwrap_err().contains("100,000"));
        assert!(go("x = 1").0.unwrap_err().contains("setsize"));
        assert!(check("if (1 { }").is_err());
        assert!(go("setsize(2,2); add(<tree>,1);")
            .0
            .unwrap_err()
            .contains("tree"));
    }
}
