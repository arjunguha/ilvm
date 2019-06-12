extern crate combine;
use syntax::*;

use combine::error::ParseError;
use combine::parser::char::{char, digit, spaces, string,alpha_num};
use combine::stream::easy;
use combine::stream::Stream;
use combine::{
    attempt, between, eof, many1, optional, satisfy_map, sep_end_by, token,
    Parser,
};
use error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum Tok {
    LBrace,
    RBrace,
    LParen,
    RParen,
    Ifz,
    Else,
    Semi,
    Equal,
    Goto,
    Abort,
    Exit,
    Malloc,
    Print,
    Free,
    Block,
    Op2(Op2),
    Int32(i32),
    Reg(usize),
    Id(String),
    Eof,
}

impl fmt::Display for Tok {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn lex(s: &str) -> Result<Vec<Tok>, easy::ParseError<&str>> {
    let tok = string("{")
        .map(|_x| Tok::LBrace)
        .or(string("}").map(|_x| Tok::RBrace))
        .or(string("(").map(|_x| Tok::LParen))
        .or(string(")").map(|_x| Tok::RParen))
        .or(string("ifz").map(|_x| Tok::Ifz))
        .or(string("goto").map(|_x| Tok::Goto))
        .or(string("abort").map(|_x| Tok::Abort))
        .or(attempt(string("else")).map(|_x| Tok::Else))
        .or(string("exit").map(|_x| Tok::Exit))
        .or(string("malloc").map(|_x| Tok::Malloc))
        .or(string("free").map(|_x| Tok::Free))
        .or(string("block").map(|_x| Tok::Block))
        .or(string("print").map(|_x| Tok::Print))
        .or(string(";").map(|_x| Tok::Semi))
        .or(attempt(string("==")).map(|_x| Tok::Op2(Op2::Eq)))
        .or(string("=").map(|_x| Tok::Equal))
        .or(string("+").map(|_x| Tok::Op2(Op2::Add)))
        .or(string("-").map(|_x| Tok::Op2(Op2::Sub)))
        .or(string("*").map(|_x| Tok::Op2(Op2::Mul)))
        .or(string("/").map(|_x| Tok::Op2(Op2::Div)))
        .or(string("%").map(|_x| Tok::Op2(Op2::Mod)))
        .or(string("<").map(|_x| Tok::Op2(Op2::LT)))
        .or((optional(char('-').or(char('+'))), many1(digit())).map(
            |(sign, digits): (Option<char>, String)| {
                let n = digits.parse::<i32>().unwrap();
                match sign {
                    Some('-') => Tok::Int32(-n),
                    _ => Tok::Int32(n),
                }
            },
        )).or(char('r')
            .with(many1(digit()))
            .map(|n: String| Tok::Reg(n.parse::<usize>().unwrap())))
        .or(between(char('"'), char('"'), many1(alpha_num())).map(|x: String| Tok::Id(x)));

    let ws = spaces();

    let mut toks = spaces().with(sep_end_by(tok, ws)).skip(eof()).map(
        |mut tokens: Vec<Tok>| {
            tokens.push(Tok::Eof);
            tokens
        },
    );
    toks.easy_parse(s).map(|tuple| tuple.0)
}

fn reg<I>() -> impl Parser<Input = I, Output = usize>
where
    I: Stream<Item = Tok>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    satisfy_map(|t| match t {
        Tok::Reg(n) => Option::Some(n),
        _ => Option::None,
    })
}

fn i32<I>() -> impl Parser<Input = I, Output = i32>
where
    I: Stream<Item = Tok>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    satisfy_map(|t| match t {
        Tok::Int32(n) => Option::Some(n),
        _ => Option::None,
    })
}

fn val<I>() -> impl Parser<Input = I, Output = Val>
where
    I: Stream<Item = Tok>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    reg().map(|r| Val::Reg(r)).or(i32().map(|n| Val::Imm(n)))
}

fn op2<I>() -> impl Parser<Input = I, Output = Op2>
where
    I: Stream<Item = Tok>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    satisfy_map(|t| match t {
        Tok::Op2(op) => Option::Some(op),
        _ => Option::None,
    })
}

fn printable<I>() -> impl Parser<Input = I, Output = Printable>
where
    I: Stream<Item = Tok>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    let id = satisfy_map(|p| match p {
        Tok::Id(s) => Option::Some(Printable::Id(s)),
        _ => Option::None,
    });

    let v = val().map(|v| Printable::Val(v));

    id.or(v)
}

enum AfterReg {
    Load(Val), // *v
    Copy(Val),
    Op2(Op2, Val, Val),
    Malloc(Val)
}

fn instr_<I>() -> impl Parser<Input = I, Output = Instr>
where
    I: Stream<Item = Tok>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    let goto = token(Tok::Goto)
        .with(token(Tok::LParen))
        .with(val())
        .skip(token(Tok::RParen))
        .skip(token(Tok::Semi))
        .map(|v| Instr::Goto(v));

    let abort = token(Tok::Abort)
        .skip(token(Tok::Semi))
        .map(|_x| Instr::Abort());

    let exit = token(Tok::Exit)
        .with(token(Tok::LParen))
        .with(val())
        .skip(token(Tok::RParen))
        .skip(token(Tok::Semi))
        .map(|v| Instr::Exit(v));

    let copy_or_op2 = reg()
        .skip(token(Tok::Equal))
        .and(
            token(Tok::Op2(Op2::Mul))
                .with(val())
                .skip(token(Tok::Semi))
                .map(|v| AfterReg::Load(v))
                .or(val()
                    .and(
                        token(Tok::Semi).map(|_x| None).or(op2()
                            .and(val())
                            .skip(token(Tok::Semi))
                            .map(|p| Some(p))),
                    ).map(|(v1, v2opt)| match v2opt {
                        None => AfterReg::Copy(v1),
                        Some((op, v2)) => AfterReg::Op2(op, v1, v2),
                    }))
                .or(token(Tok::Malloc)
                    .with(between(token(Tok::LParen), token(Tok::RParen),
                        val()))
                    .skip(token(Tok::Semi))
                    .map(|v| AfterReg::Malloc(v)))
        ).and(instr())
        .map(|((r, k), rest)| match k {
            AfterReg::Load(v) => Instr::Load(r, v, Box::new(rest)),
            AfterReg::Copy(v) => Instr::Copy(r, v, Box::new(rest)),
            AfterReg::Op2(op, v1, v2) =>
                Instr::Op2(r, op, v1, v2, Box::new(rest)),
            AfterReg::Malloc(v) => Instr::Malloc(r, v, Box::new(rest))
        });

    let load = reg()
        .skip(token(Tok::Equal))
        .skip(token(Tok::Op2(Op2::Mul)))
        .and(val())
        .skip(token(Tok::Semi))
        .and(instr())
        .map(|((r, v), rest)| Instr::Load(r, v, Box::new(rest)));

    let store = token(Tok::Op2(Op2::Mul))
        .with(reg())
        .skip(token(Tok::Equal))
        .and(val())
        .skip(token(Tok::Semi))
        .and(instr())
        .map(|((r, v), rest)| Instr::Store(r, v, Box::new(rest)));

    let ifz = token(Tok::Ifz)
        .with(val())
        .and(between(token(Tok::LBrace), token(Tok::RBrace), instr()))
        .skip(token(Tok::Else))
        .and(between(token(Tok::LBrace), token(Tok::RBrace), instr()))
        .map(|((v, tru), fls)| Instr::IfZ(v, Box::new(tru), Box::new(fls)));

    let free = token(Tok::Free)
        .with(between(token(Tok::LParen), token(Tok::RParen), reg()))
        .skip(token(Tok::Semi))
        .and(instr())
        .map(|(r, rest)| Instr::Free(r, Box::new(rest)));

    let print = token(Tok::Print)
        .with(between(token(Tok::LParen), token(Tok::RParen), printable()))
        .skip(token(Tok::Semi))
        .and(instr())
        .map(|(r, rest)| Instr::Print(r, Box::new(rest)));

    goto.or(abort)
        .or(exit)
        .or(copy_or_op2)
        .or(load)
        .or(store)
        .or(ifz)
        .or(free)
        .or(print)
}

parser!{
    fn instr[I]()(I) -> Instr
    where [I: Stream<Item = Tok>]
    {
        instr_()
    }
}

fn block<I>() -> impl Parser<Input = I, Output = Block>
where
    I: Stream<Item = Tok>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    token(Tok::Block).with(i32()).and(between(
        token(Tok::LBrace),
        token(Tok::RBrace),
        instr(),
    ))
}

pub fn parse(input: &str) -> Result<Vec<Block>, Error> {
    match lex(input) {
        Result::Err(e) => Result::Err(Error::Parse(format!("{:?}", e))),
        Result::Ok(tokens) => {
            let mut ast = many1(block()).skip(token(Tok::Eof));
            match ast.easy_parse(&tokens[..]) {
                Result::Err(e) => Result::Err(Error::Parse(format!("{:?}", e))),
                Result::Ok(tuple) => Result::Ok(tuple.0),
            }
        }
    }
}
