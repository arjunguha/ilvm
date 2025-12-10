use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, digit1, multispace1},
    combinator::{map, opt, recognize, value},
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};
use syntax::*;
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
    Array,
    Comma,
    Free,
    Block,
    Op2(Op2),
    Int32(i32),
    Reg(usize),
    Id(String),
    Eof,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn lex(s: &str) -> Result<Vec<Tok>, Error> {
    let mut tokens = Vec::new();
    let mut input = s.trim_start();
    
    while !input.is_empty() {
        // Skip whitespace
        if let Ok((rest, _)) = multispace1::<_, nom::error::Error<&str>>(input) {
            input = rest;
            continue;
        }
        
        // Try to match tokens
        if let Ok((rest, tok)) = parse_token(input) {
            tokens.push(tok);
            input = rest;
        } else {
            return Err(Error::Parse(format!("Unexpected character at: {}", &input[..input.len().min(20)])));
        }
    }
    
    tokens.push(Tok::Eof);
    Ok(tokens)
}

fn parse_token(input: &str) -> IResult<&str, Tok> {
    alt((
        alt((
            value(Tok::LBrace, tag("{")),
            value(Tok::RBrace, tag("}")),
            value(Tok::LParen, tag("(")),
            value(Tok::RParen, tag(")")),
            value(Tok::Comma, tag(",")),
            value(Tok::Semi, tag(";")),
            value(Tok::Equal, tag("=")),
            value(Tok::Op2(Op2::Eq), tag("==")),
            value(Tok::Op2(Op2::Add), tag("+")),
            value(Tok::Op2(Op2::Sub), tag("-")),
            value(Tok::Op2(Op2::Mul), tag("*")),
            value(Tok::Op2(Op2::Div), tag("/")),
            value(Tok::Op2(Op2::Mod), tag("%")),
            value(Tok::Op2(Op2::LT), tag("<")),
        )),
        alt((
            value(Tok::Ifz, tag("ifz")),
            value(Tok::Else, tag("else")),
            value(Tok::Goto, tag("goto")),
            value(Tok::Abort, tag("abort")),
            value(Tok::Exit, tag("exit")),
            value(Tok::Malloc, tag("malloc")),
            value(Tok::Free, tag("free")),
            value(Tok::Block, tag("block")),
            value(Tok::Print, tag("print")),
            value(Tok::Array, tag("array")),
        )),
        parse_int32,
        parse_reg,
        parse_id,
    ))(input)
}

fn parse_int32(input: &str) -> IResult<&str, Tok> {
    map(
        recognize(pair(opt(alt((char('-'), char('+')))), digit1)),
        |s: &str| {
            let n = s.parse::<i32>().unwrap();
            Tok::Int32(n)
        },
    )(input)
}

fn parse_reg(input: &str) -> IResult<&str, Tok> {
    map(
        preceded(char('r'), digit1),
        |s: &str| Tok::Reg(s.parse::<usize>().unwrap()),
    )(input)
}

fn parse_id(input: &str) -> IResult<&str, Tok> {
    map(
        delimited(char('"'), take_while1(|c: char| c.is_alphanumeric()), char('"')),
        |s: &str| Tok::Id(s.to_string()),
    )(input)
}

fn reg(input: &[Tok]) -> IResult<&[Tok], usize> {
    match input.first() {
        Some(Tok::Reg(n)) => Ok((&input[1..], *n)),
        _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))),
    }
}

fn i32_token(input: &[Tok]) -> IResult<&[Tok], i32> {
    match input.first() {
        Some(Tok::Int32(n)) => Ok((&input[1..], *n)),
        _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))),
    }
}

fn val(input: &[Tok]) -> IResult<&[Tok], Val> {
    alt((
        map(reg, Val::Reg),
        map(i32_token, Val::Imm),
    ))(input)
}

fn op2_token(input: &[Tok]) -> IResult<&[Tok], Op2> {
    match input.first() {
        Some(Tok::Op2(op)) => Ok((&input[1..], op.clone())),
        _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))),
    }
}

fn id_token(input: &[Tok]) -> IResult<&[Tok], String> {
    match input.first() {
        Some(Tok::Id(s)) => Ok((&input[1..], s.clone())),
        _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))),
    }
}

fn printable(input: &[Tok]) -> IResult<&[Tok], Printable> {
    alt((
        map(id_token, Printable::Id),
        map(val, Printable::Val),
        map(
            tuple((
                token_match(Tok::Array),
                token_match(Tok::LParen),
                val,
                token_match(Tok::Comma),
                val,
                token_match(Tok::RParen),
            )),
            |(_, _, v1, _, v2, _)| Printable::Array(v1, v2),
        ),
    ))(input)
}

fn token_match(tok: Tok) -> impl Fn(&[Tok]) -> IResult<&[Tok], ()> {
    move |input: &[Tok]| {
        match input.first() {
            Some(t) if std::mem::discriminant(t) == std::mem::discriminant(&tok) => {
                // For Tok::Op2, we need to check the value too
                match (&tok, t) {
                    (Tok::Op2(op1), Tok::Op2(op2)) if op1 == op2 => Ok((&input[1..], ())),
                    (Tok::Op2(_), Tok::Op2(_)) => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))),
                    _ => Ok((&input[1..], ())),
                }
            }
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))),
        }
    }
}


fn instr(input: &[Tok]) -> IResult<&[Tok], Instr> {
    alt((
        parse_goto,
        parse_abort,
        parse_exit,
        parse_copy_or_op2,
        parse_load,
        parse_store,
        parse_ifz,
        parse_free,
        parse_print,
    ))(input)
}

fn parse_goto(input: &[Tok]) -> IResult<&[Tok], Instr> {
    let (input, _) = token_match(Tok::Goto)(input)?;
    let (input, _) = token_match(Tok::LParen)(input)?;
    let (input, v) = val(input)?;
    let (input, _) = token_match(Tok::RParen)(input)?;
    let (input, _) = token_match(Tok::Semi)(input)?;
    Ok((input, Instr::Goto(v)))
}

fn parse_abort(input: &[Tok]) -> IResult<&[Tok], Instr> {
    let (input, _) = token_match(Tok::Abort)(input)?;
    let (input, _) = token_match(Tok::Semi)(input)?;
    Ok((input, Instr::Abort()))
}

fn parse_exit(input: &[Tok]) -> IResult<&[Tok], Instr> {
    let (input, _) = token_match(Tok::Exit)(input)?;
    let (input, _) = token_match(Tok::LParen)(input)?;
    let (input, v) = val(input)?;
    let (input, _) = token_match(Tok::RParen)(input)?;
    let (input, _) = token_match(Tok::Semi)(input)?;
    Ok((input, Instr::Exit(v)))
}

fn parse_copy_or_op2(input: &[Tok]) -> IResult<&[Tok], Instr> {
    let (input, r) = reg(input)?;
    let (input, _) = token_match(Tok::Equal)(input)?;
    
    // Try load: *v
    if let Ok((input_after_mul, _)) = token_match(Tok::Op2(Op2::Mul))(input) {
        if let Ok((input_after_val, v)) = val(input_after_mul) {
            if let Ok((input_after_semi, _)) = token_match(Tok::Semi)(input_after_val) {
                let (input, rest) = instr(input_after_semi)?;
                return Ok((input, Instr::Load(r, v, Box::new(rest))));
            }
        }
    }
    
    // Try malloc: malloc(v)
    if let Ok((input_after_malloc, _)) = token_match(Tok::Malloc)(input) {
        if let Ok((input_after_lparen, _)) = token_match(Tok::LParen)(input_after_malloc) {
            if let Ok((input_after_val, v)) = val(input_after_lparen) {
                if let Ok((input_after_rparen, _)) = token_match(Tok::RParen)(input_after_val) {
                    if let Ok((input_after_semi, _)) = token_match(Tok::Semi)(input_after_rparen) {
                        let (input, rest) = instr(input_after_semi)?;
                        return Ok((input, Instr::Malloc(r, v, Box::new(rest))));
                    }
                }
            }
        }
    }
    
    // Try copy or op2: v or v op v
    let (input, v1) = val(input)?;
    
    // Check if there's an operator
    if let Ok((input_after_op, op)) = op2_token(input) {
        let (input, v2) = val(input_after_op)?;
        let (input, _) = token_match(Tok::Semi)(input)?;
        let (input, rest) = instr(input)?;
        Ok((input, Instr::Op2(r, op, v1, v2, Box::new(rest))))
    } else {
        // Just a copy
        let (input, _) = token_match(Tok::Semi)(input)?;
        let (input, rest) = instr(input)?;
        Ok((input, Instr::Copy(r, v1, Box::new(rest))))
    }
}

fn parse_load(input: &[Tok]) -> IResult<&[Tok], Instr> {
    let (input, r) = reg(input)?;
    let (input, _) = token_match(Tok::Equal)(input)?;
    let (input, _) = token_match(Tok::Op2(Op2::Mul))(input)?;
    let (input, v) = val(input)?;
    let (input, _) = token_match(Tok::Semi)(input)?;
    let (input, rest) = instr(input)?;
    Ok((input, Instr::Load(r, v, Box::new(rest))))
}

fn parse_store(input: &[Tok]) -> IResult<&[Tok], Instr> {
    let (input, _) = token_match(Tok::Op2(Op2::Mul))(input)?;
    let (input, r) = reg(input)?;
    let (input, _) = token_match(Tok::Equal)(input)?;
    let (input, v) = val(input)?;
    let (input, _) = token_match(Tok::Semi)(input)?;
    let (input, rest) = instr(input)?;
    Ok((input, Instr::Store(r, v, Box::new(rest))))
}

fn parse_ifz(input: &[Tok]) -> IResult<&[Tok], Instr> {
    let (input, _) = token_match(Tok::Ifz)(input)?;
    let (input, v) = val(input)?;
    let (input, _) = token_match(Tok::LBrace)(input)?;
    let (input, tru) = instr(input)?;
    let (input, _) = token_match(Tok::RBrace)(input)?;
    let (input, _) = token_match(Tok::Else)(input)?;
    let (input, _) = token_match(Tok::LBrace)(input)?;
    let (input, fls) = instr(input)?;
    let (input, _) = token_match(Tok::RBrace)(input)?;
    Ok((input, Instr::IfZ(v, Box::new(tru), Box::new(fls))))
}

fn parse_free(input: &[Tok]) -> IResult<&[Tok], Instr> {
    let (input, _) = token_match(Tok::Free)(input)?;
    let (input, _) = token_match(Tok::LParen)(input)?;
    let (input, r) = reg(input)?;
    let (input, _) = token_match(Tok::RParen)(input)?;
    let (input, _) = token_match(Tok::Semi)(input)?;
    let (input, rest) = instr(input)?;
    Ok((input, Instr::Free(r, Box::new(rest))))
}

fn parse_print(input: &[Tok]) -> IResult<&[Tok], Instr> {
    let (input, _) = token_match(Tok::Print)(input)?;
    let (input, _) = token_match(Tok::LParen)(input)?;
    let (input, p) = printable(input)?;
    let (input, _) = token_match(Tok::RParen)(input)?;
    let (input, _) = token_match(Tok::Semi)(input)?;
    let (input, rest) = instr(input)?;
    Ok((input, Instr::Print(p, Box::new(rest))))
}

fn block(input: &[Tok]) -> IResult<&[Tok], Block> {
    let (input, _) = token_match(Tok::Block)(input)?;
    let (input, n) = i32_token(input)?;
    let (input, _) = token_match(Tok::LBrace)(input)?;
    let (input, instr) = instr(input)?;
    let (input, _) = token_match(Tok::RBrace)(input)?;
    Ok((input, (n, instr)))
}

pub fn parse(input: &str) -> Result<Vec<Block>, Error> {
    let tokens = lex(input)?;
    
    let mut blocks = Vec::new();
    let mut remaining = &tokens[..];
    
    while !remaining.is_empty() {
        if remaining.len() == 1 && matches!(remaining[0], Tok::Eof) {
            break;
        }
        match block(remaining) {
            Ok((rest, blk)) => {
                blocks.push(blk);
                remaining = rest;
            }
            Err(e) => {
                return Err(Error::Parse(format!("Parse error: {:?}", e)));
            }
        }
    }
    
    // Check for Eof token
    if !remaining.is_empty() && !matches!(remaining[0], Tok::Eof) {
        return Err(Error::Parse(format!("Expected end of input, found: {:?}", remaining[0])));
    }
    
    Ok(blocks)
}
