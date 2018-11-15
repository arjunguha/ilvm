#![recursion_limit = "128"]

#[macro_use]
extern crate combine;
extern crate clap;

mod error;
mod eval;
mod parser;
mod syntax;
mod tc;

use clap::{App, Arg};
use error::*;
use std::fs::File;
use std::io::prelude::*;
use std::process;

fn parse_and_eval(
    code: &str,
    mem_limit: usize,
    reg_limit: usize,
) -> Result<i32, Error> {
    let blocks = try!(parser::parse(code));
    let blocks = try!(tc::tc(blocks));
    return eval::eval(mem_limit, reg_limit, blocks);
}

fn main_result() -> Result<i32, Error> {
    let args = App::new("ILVM")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("memlimit")
                .short("m")
                .long("memory-limit")
                .value_name("MEM_LIMIT")
                .default_value("1024")
                .help("Sets a memory limit in words")
                .takes_value(true),
        ).arg(
            Arg::with_name("INPUT")
                .value_name("FILENAME")
                .help("Sets the input file to use")
                .required(true)
                .index(1),
        ).arg(
            Arg::with_name("reglimit")
                .short("r")
                .value_name("REG_LIMIT")
                .default_value("32")
                .long("num-registers")
                .help("Set the number of registers"),
        ).get_matches();
    let path = args.value_of("INPUT").unwrap();
    let mut file = try!(File::open(&path));
    let mut buf = String::new();
    try!(file.read_to_string(&mut buf));
    parse_and_eval(
        &buf[..],
        args.value_of("memlimit").unwrap().parse::<usize>().unwrap(),
        args.value_of("reglimit").unwrap().parse::<usize>().unwrap(),
    )
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

    fn parse_and_eval(code: &str) -> Result<i32, super::error::Error> {
        super::parse_and_eval(code, 500, 10)
    }

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

    #[test]
    fn test_fac() {
        let r = parse_and_eval(
            r#"
            block 0 {
                r2 = 1;
                r1 = 5;
                goto(1);
            }
            block 1 {
                ifz r1 {
                   exit(r2);
                }
                else {
                    r2 = r2 * r1;
                    r1 = r1 - 1;
                    goto(1);
                }
            }"#,
        ).unwrap();
        assert!(r == 120);
    }

}
