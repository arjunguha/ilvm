#![recursion_limit = "128"]

#[macro_use]
extern crate combine;

mod error;
mod eval;
mod parser;
mod syntax;
mod tc;

use error::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::process;

fn parse_and_eval(code: &str) -> Result<i32, Error> {
    let blocks = try!(parser::parse(code));
    let blocks = try!(tc::tc(blocks));
    return eval::eval(1000, 10, blocks);
}

fn main_result() -> Result<i32, Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(Error::Usage(
            "Missing command-line argument.\nUsage: ilvm filename".to_string(),
        ));
    }
    let path = &args[1];
    let mut file = try!(File::open(&path));
    let mut buf = String::new();
    try!(file.read_to_string(&mut buf));
    parse_and_eval(&buf[..])
}

fn main() {
    match main_result() {
        Ok(r) => println!("Normal termination. Result = {}", r),
        Err(err) => {
            println!("An error occurred.\n{}", err);
            process::exit(1)
        }
    }
}

#[cfg(test)]
mod tests {

    use super::parse_and_eval;

    #[test]
    fn test_exit() {
        let r = parse_and_eval(
            r#"
            block 0 {
                exit(200);
            }"#,
        ).unwrap();
        assert!(r == 200);
    }

    #[test]
    fn test_reg_copy() {
        let r = parse_and_eval(
            r#"
            block 0 {
                r0 = 200;
                r2 = r0;
                exit(r2);
            }"#,
        ).unwrap();
        assert!(r == 200);
    }

    #[test]
    fn test_reg_add() {
        let r = parse_and_eval(
            r#"
            block 0 {
                r0 = 200;
                r1 = 11;
                r3 = r0 + r1;
                exit(r3);
            }"#,
        ).unwrap();
        assert!(r == 211);
    }

    #[test]
    fn test_load_store() {
        let r = parse_and_eval(
            r#"
            block 0 {
                r0 = 200;
                *r0 = 42;
                r1 = *r0;
                exit(r1);
            }"#,
        ).unwrap();
        assert!(r == 42);
    }

    #[test]
    fn test_goto() {
        let r = parse_and_eval(
            r#"
            block 0 {
                r2 = 200;
                goto(10);
            }
            block 10 {
                r2 = r2 + 1;
                exit(r2);
            }"#,
        ).unwrap();
        assert!(r == 201);
    }

    #[test]
    fn test_indirect_goto() {
        let r = parse_and_eval(
            r#"
            block 0 {
                r2 = 200;
                r3 = 10;
                goto(r3);
            }
            block 10 {
                r2 = r2 + r3;
                exit(r2);
            }"#,
        ).unwrap();
        assert!(r == 210);
    }

    #[test]
    fn test_ifz() {
        let r = parse_and_eval(
            r#"
            block 0 {
                r2 = 1;
                ifz r2 {
                    exit(20);
                }
                else {
                    exit(30);
                }
            }"#,
        ).unwrap();
        assert!(r == 30);
    }

}
