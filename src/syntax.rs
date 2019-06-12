pub type Reg = usize;

#[derive(Debug, PartialEq)]
pub enum Val {
    Reg(usize),
    Imm(i32),
}

// Clone is needed to tokenize.
#[derive(Debug, PartialEq, Clone)]
pub enum Op2 {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    LT,
    Eq
}

#[derive(Debug, PartialEq)]
pub enum Printable {
    Id(String),
    Val(Val)
}

#[derive(Debug, PartialEq)]
pub enum Instr {
    Goto(Val),
    Exit(Val),
    Abort(),
    Op2(Reg, Op2, Val, Val, Box<Instr>),
    Copy(Reg, Val, Box<Instr>),
    Load(Reg, Val, Box<Instr>),
    Store(Reg, Val, Box<Instr>),
    IfZ(Val, Box<Instr>, Box<Instr>),
    Malloc(Reg, Val, Box<Instr>),
    Print(Printable, Box<Instr>),
    Free(Reg, Box<Instr>),
}

pub type Block = (i32, Instr);
