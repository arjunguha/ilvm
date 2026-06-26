use error::Error;
use std::collections::HashMap;
use syntax::{Instr, Op1, Op2, Printable, Val};

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
        Op2::BitAnd => m & n,
        Op2::BitOr => m | n,
        Op2::BitXor => m ^ n,
        Op2::Shl => m << n,
        Op2::Shr => m >> n,
        Op2::UShr => ((m as u32) >> n) as i32,
        Op2::LT => {
            if m < n {
                1
            } else {
                0
            }
        }
        Op2::Eq => {
            if m == n {
                1
            } else {
                0
            }
        }
    }
}

fn eval_op1(op1: &Op1, n: i32) -> i32 {
    match op1 {
        Op1::BitNot => !n,
    }
}

type R = Result<i32, Error>;

fn print_printable(st: &mut State, p: &Printable) {
    match p {
        Printable::Id(s) => println!("{}", s),
        Printable::Val(v) => println!("{}", eval_val(&st.registers, &v)),
        Printable::Array(v1, v2) => {
            let ptr = eval_val(&st.registers, &v1) as usize;
            let len = eval_val(&st.registers, &v2) as usize;
            if ptr + len >= st.heap.len() {
                println!("attempted to print invalid address {}", ptr + len);
            } else {
                let idx_list = ptr..(ptr + len);
                let vals = idx_list;
                print!("[");
                for val in vals {
                    print!("{:?}; ", val);
                }
                println!("]");
            }
        }
    }
}

fn print_str(st: &mut State, v: &Val) -> Result<(), Error> {
    let ptr = eval_val(&st.registers, v) as usize;
    let mut bytes = Vec::new();
    let mut word_idx = ptr;

    loop {
        if word_idx >= st.heap.len() {
            return Err(Error::Runtime(format!(
                "print_str({:?}) invalid address {}",
                v, word_idx
            )));
        }

        let word = st.heap[word_idx] as u32;
        for shift in [24, 16, 8, 0].iter() {
            let byte = ((word >> shift) & 0xff) as u8;
            if byte == 0 {
                let s = String::from_utf8(bytes).map_err(|_| {
                    Error::Runtime(
                        "print_str encountered non-ASCII data".to_string(),
                    )
                })?;
                if !s.is_ascii() {
                    return Err(Error::Runtime(
                        "print_str encountered non-ASCII data".to_string(),
                    ));
                }
                println!("{}", s);
                return Ok(());
            }
            if !byte.is_ascii() {
                return Err(Error::Runtime(
                    "print_str encountered non-ASCII data".to_string(),
                ));
            }
            bytes.push(byte);
        }

        word_idx += 1;
    }
}

fn malloc(free_list: FreeList, size: usize) -> Option<(FreeList, usize)> {
    match free_list {
        FreeList::Nil => None,
        FreeList::Node(base, free_size, rest) => {
            if size == free_size {
                Some((*rest, base))
            } else if size < free_size {
                Some((
                    FreeList::Node(base + size, free_size - size, rest),
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
        Instr::Op1(r, op, v, rest) => {
            let n = eval_val(&st.registers, &v);
            st.registers[*r] = eval_op1(&op, n);
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
        Instr::Print(p, rest) => {
            print_printable(st, p);
            eval_rec(st, env, rest)
        }
        Instr::PrintStr(v, rest) => {
            print_str(st, v)?;
            eval_rec(st, env, rest)
        }
        Instr::Exit(v) => Result::Ok(eval_val(&st.registers, v)),
        Instr::Abort() => {
            Result::Err(Error::Runtime("called abort".to_string()))
        }
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
            } else {
                let mut nil_list = FreeList::Nil;
                std::mem::swap(&mut st.free_list, &mut nil_list);
                let (free_list2, ptr) = try!(malloc(nil_list, n)
                    .ok_or(Error::Runtime("malloc OOM".to_string())));
                st.free_list = free_list2;
                st.registers[*r] = ptr as i32;
                st.alloc_blocks.insert(ptr, n);
            }
            eval_rec(st, env, rest)
        }
        Instr::Free(r, rest) => {
            let ptr = st.registers[*r] as usize;
            let mut nil_list = FreeList::Nil;
            let size = *try!(st
                .alloc_blocks
                .get(&ptr)
                .ok_or(Error::Runtime("free bad ptr".to_string())));
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
    cli_args: &[String],
) -> R {
    let (heap, heap_start) = init_heap(heap_size, cli_args)?;
    let mut st = State {
        heap,
        registers: vec![0; num_registers],
        free_list: if heap_start < heap_size {
            FreeList::Node(
                heap_start,
                heap_size - heap_start,
                Box::new(FreeList::Nil),
            )
        } else {
            FreeList::Nil
        },
        alloc_blocks: HashMap::new(),
    };
    if num_registers > 0 {
        st.registers[0] = heap_start as i32;
    }
    let env = Env {
        instructions: blocks,
    };
    env.instructions
        .get(&0)
        .ok_or(Error::Usage("Expected block 0".to_string()))
        .and_then(|instr| eval_rec(&mut st, &env, instr))
}

fn string_words(s: &str) -> usize {
    (s.len() + 1 + 3) / 4
}

fn pack_string(heap: &mut [i32], mut word_idx: usize, s: &str) {
    let mut bytes = s.as_bytes().iter().copied().chain(std::iter::once(0));

    loop {
        let mut word = 0u32;
        let mut saw_byte = false;
        for shift in [24, 16, 8, 0].iter() {
            if let Some(byte) = bytes.next() {
                saw_byte = true;
                word |= (byte as u32) << shift;
            }
        }
        if !saw_byte {
            break;
        }
        heap[word_idx] = word as i32;
        word_idx += 1;
    }
}

fn init_heap(
    heap_size: usize,
    cli_args: &[String],
) -> Result<(Vec<i32>, usize), Error> {
    if cli_args.len() > i32::max_value() as usize {
        return Err(Error::Usage(
            "too many command-line arguments".to_string(),
        ));
    }
    if let Some(arg) = cli_args.iter().find(|arg| !arg.is_ascii()) {
        return Err(Error::Usage(format!(
            "command-line argument is not ASCII: {}",
            arg
        )));
    }

    let arg_vector_words = 1 + cli_args.len();
    let string_data_words =
        cli_args.iter().map(|arg| string_words(arg)).sum::<usize>();
    let heap_start = 1 + arg_vector_words + string_data_words;
    if heap_start > heap_size {
        return Err(Error::Runtime(
            "not enough heap for command-line arguments".to_string(),
        ));
    }

    let mut heap = vec![0; heap_size];
    heap[1] = cli_args.len() as i32;

    let mut string_ptr = 1 + arg_vector_words;
    for (i, arg) in cli_args.iter().enumerate() {
        heap[2 + i] = string_ptr as i32;
        pack_string(&mut heap, string_ptr, arg);
        string_ptr += string_words(arg);
    }

    Ok((heap, heap_start))
}
