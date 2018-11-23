ILVM
====

The *intermediate-level virtual machine* (ILVM) is an interpreter for a
language that has a few features that make it higher-level than typical
assembly languages:

1.  ILVM has *malloc* and *free* instructions that work similar to the C
    functions. However, the free list is not in program memory, so a program
    cannot accidentally clobber it.

2. ILVM has an *if ... then .. else* instruction, so it supports structured
   control flow.

3. ILVM has primitive instructions to print to screen.

Note: ILVM implements a Harvard architecture and does not have a stack.

Language Overview
-----------------

### Blocks

A program in ILVM is a collection of *blocks*, where each contains a sequence
of instructions. Each block has a unique number and execution always starts
at block zero. For example, the following program has a single block
and prints *30*:

```
block 0 {
    exit(30);
}
```

The following program uses the *goto* instruction to jump to block 1 and then
print *20*:

```
block 1 {
    exit(20);
}
block 0 {
    goto(1);
}
```

Note that the order in which blocks appear is not relevant.

### Register, Loads, and Stores

ILVM has registers numbered *r0* through *rn*, where *n* can be set by the
user. ILVM supports basic binary operations (+, -, *) that take either
registers or constants as arguments, and store their results in registers.
It also supports operations to load data into registers from the heap, and to
store values from registers in the heap. Some examples of these operations
are given below:

```
block 0 {
    r0 = 10;     // set r0 to 10
    r1 = r0 * 2; // set r1 to 20
    *r1 = 50;    // store 50 at heap address 20
    r2 = *r1;    // load the value at heap address in r1 into r2 (i.e., set
                 // r2 to 50
    exit(0);
}
```

### Control Flow

The *goto(n);* instruction jumps to block *n*. The argument may either be
a literal constant, or it may be value stored in a register. For example,
the following program calculates the address of the block to jump to:

```
block 0 {
    r0 = 10;
    r1 = r0 - 9;
    goto(r1); // jumps to block 1
}
block 1 {
    exit(0);
}
```

The *ifz r trueBlock else falseBlock* instruction is a conditional with two
sub-blocks. If the value in the register is zero, it executes *trueBlock*, else
it executes *falseBlock*. For example, the following program calculates
*factorial(5)*:

```
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
}
```

Note that the sub-blocks may have nested *ifz* instructions. Also note
that sub-blocks are not numbered. Therefore, a program cannot use
*goto* to jump to a sub-block.

### Termination and (lack of) fall-through

The *exit(n)* instruction terminates the program normally, and produces
the value *n*. The *abort;* instruction is an abnormal exit and should be
avoided if possible. Note that the sequence of instructions
in every block *must* end with either *exit*, *goto*, or *abort*. In other
words, a program cannot "fall-through" from one block to the next, and must
explicitly jump to another block or terminate.

### Memory allocation

The *word size* of ILVM is 32-bits.

A program can read and write to any memory address. The initial value stored at
all memory addresses and registers is zero. Each memory location and register
is one word long (i.e., 32 bits).

ILVM has a *malloc(n)* instruction that returns the address of
a free block of memory that is *n* **words** long, and a *free(a)* instruction that
frees the block that was allocated at the address *a*. It may be convenient
to use these functions instead of writing an allocation manually. Note that
these operators maintain their metadata in an independent part of memory, so
it is not possible for an ill-behaved program to corrupt the state that
*malloc* and *free* require.

Programs do not have to use *malloc* and *free*. However, it may be convenient
to do so.

Concrete Syntax
---------------

```
Registers         r ::= "r0" | ... | "r64"

Values          val ::= r
                      | i                     Signed 32-bit integers

Operators        op ::= "+"
                      | "-"
                      | "*"
                      | "/"
                      | "%"
                      | "=="
                      | "<"

Instructions  instr ::= "goto" "(" val ")" ";"
                      | "exit" "(" val ")" ";"
                      | "abort" ";"
                      | r "=" val op val ";" instr
                      | r "=" val ";" instr
                      | r "=" "*" val ";" instr
                      | "*" r "=" val ";" instr
                      | "ifz" val "{" instr "}" "else" "{" instr "}""
                      | r "=" "malloc" "(" val ")" ";" instr
                      | "free" "("r ")" ";" instr

Blocks        block ::= "block" n "{" instr "}"

Programs          p ::= block
                      | block p
```

Command-Line Interface
----------------------

Run `ilvm --help` for documentation.
