use error::Error;
use std::collections::HashMap;
use syntax::{Instr, Op2, Val};

enum FreeList {
    Nil,
    Node(usize, usize, Box<FreeList>),
}
struct State {
    heap: Vec<i32>,
    registers: Vec<i32>,
    free_list: FreeList,
    alloc_blocks: HashMap<usize, usize>,
}

struct Env {
    instructions: HashMap<i32, Instr>,
}

fn eval_val(reg: &[i32], v: &Val) -> i32 {
    match *v {
        Val::Imm(n) => n,
        Val::Reg(i) => reg[i],
    }
}

fn eval_op2(op2: &Op2, m: i32, n: i32) -> i32 {
    match op2 {
        Op2::Add => m + n,
        Op2::Sub => m - n,
        Op2::Mul => m * n,
        Op2::Div => m / n,
        Op2::Mod => m % n,
        Op2::LT => if m < n { 1 } else { 0 },
        Op2::Eq => if m == n { 1 } else { 0 }
    }
}

type R = Result<i32, Error>;

fn malloc(free_list: FreeList, size: usize) -> Option<(FreeList, usize)> {
    match free_list {
        FreeList::Nil => None,
        FreeList::Node(base, free_size, rest) => {
            if size == free_size {
                Some((*rest, base))
            } else if size < free_size {
                Some((
                    FreeList::Node(base + size, free_size - base, rest),
                    base,
                ))
            } else {
                match malloc(*rest, size) {
                    None => None,
                    Some((rest2, base2)) => Some((
                        FreeList::Node(base, free_size, Box::new(rest2)),
                        base2,
                    )),
                }
            }
        }
    }
}

fn free(free_list: FreeList, ptr: usize, size: usize) -> FreeList {
    match free_list {
        FreeList::Nil => FreeList::Node(ptr, size, Box::new(FreeList::Nil)),
        FreeList::Node(base1, size1, rest1) => {
            if ptr + size == base1 {
                FreeList::Node(ptr, size + size1, rest1)
            } else if ptr + size < base1 {
                FreeList::Node(
                    ptr,
                    size,
                    Box::new(FreeList::Node(base1, size1, rest1)),
                )
            } else if ptr == base1 + size1 {
                free(*rest1, base1, size + size1)
            } else {
                FreeList::Node(base1, size1, Box::new(free(*rest1, ptr, size)))
            }
        }
    }
}

fn eval_rec(st: &mut State, env: &Env, instr: &Instr) -> R {
    match instr {
        Instr::Copy(r, v, rest) => {
            st.registers[*r] = eval_val(&st.registers, &v);
            eval_rec(st, env, rest)
        }
        Instr::Op2(r, op, v1, v2, rest) => {
            let m = eval_val(&st.registers, &v1);
            let n = eval_val(&st.registers, &v2);
            st.registers[*r] = eval_op2(&op, m, n);
            eval_rec(st, env, rest)
        }
        Instr::Load(r, v, rest) => {
            let ptr = eval_val(&st.registers, v) as usize;
            if ptr >= st.heap.len() {
                return Err(Error::Runtime(format!(
                    "{} = *{:?} invalid address {}",
                    r, v, ptr
                )));
            }
            st.registers[*r] = st.heap[ptr];
            eval_rec(st, env, rest)
        }
        Instr::Store(r, v, rest) => {
            let ptr = st.registers[*r] as usize;
            if ptr >= st.heap.len() {
                return Err(Error::Runtime(format!(
                    "*{} = {:?} invalid address {}",
                    r, v, ptr
                )));
            }
            st.heap[ptr] = eval_val(&st.registers, v);
            eval_rec(st, env, rest)
        }
        Instr::Goto(v) => {
            let code_ptr = eval_val(&st.registers, v);
            match env.instructions.get(&code_ptr) {
                Option::Some(instr) => eval_rec(st, env, instr),
                Option::None => Err(Error::Runtime(format!(
                    "goto({}) invalid code address",
                    code_ptr
                ))),
            }
        }
        Instr::Print(s, rest) => {
            println!("{}", s);
            eval_rec(st, env, rest)
        },
        Instr::Exit(v) => Result::Ok(eval_val(&st.registers, v)),
        Instr::Abort() => Result::Err(Error::Runtime("called abort".to_string())),
        Instr::IfZ(v, true_part, false_part) => {
            if eval_val(&st.registers, v) == 0 {
                eval_rec(st, env, true_part)
            } else {
                eval_rec(st, env, false_part)
            }
        }
        Instr::Malloc(r, v, rest) => {
            let n = eval_val(&st.registers, v) as usize;
            if n == 0 {
                st.registers[*r] = 0;
            }
            else {
                let mut nil_list = FreeList::Nil;
                std::mem::swap(&mut st.free_list, &mut nil_list);
                let (free_list2, ptr) = try!(
                    malloc(nil_list, n)
                        .ok_or(Error::Runtime("malloc OOM".to_string()))
                );
                st.free_list = free_list2;
                st.registers[*r] = ptr as i32;
                st.alloc_blocks.insert(ptr, n);
            }
            eval_rec(st, env, rest)
        }
        Instr::Free(r, rest) => {
            let ptr = st.registers[*r] as usize;
            let mut nil_list = FreeList::Nil;
            let size = *try!(
                st.alloc_blocks
                    .get(&ptr)
                    .ok_or(Error::Runtime("free bad ptr".to_string()))
            );
            std::mem::swap(&mut st.free_list, &mut nil_list);
            st.free_list = free(nil_list, ptr, size);
            eval_rec(st, env, rest)
        }
    }
}

pub fn eval(
    heap_size: usize,
    num_registers: usize,
    blocks: HashMap<i32, Instr>,
) -> R {
    let mut st = State {
        heap: vec![0; heap_size],
        registers: vec![0; num_registers],
        free_list: FreeList::Node(1, heap_size - 1, Box::new(FreeList::Nil)),
        alloc_blocks: HashMap::new(),
    };
    let env = Env {
        instructions: blocks,
    };
    env.instructions
        .get(&0)
        .ok_or(Error::Usage("Expected block 0".to_string()))
        .and_then(|instr| eval_rec(&mut st, &env, instr))
}
