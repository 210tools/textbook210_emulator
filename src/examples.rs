//! Example programs, shown in the Help pane and the editor's Examples menu.

pub const EXAMPLES: &[(&str, &str)] = &[
    (
        "Hello World",
        r#"; Simple Hello World program
.ORIG x3000
LEA R0, MESSAGE    ; Load address of the message
PUTS               ; Output the string
HALT               ; Halt the program

MESSAGE: .STRINGZ "Hello, World!"
.END"#,
    ),
    (
        "Input and Echo",
        r#"; Program that gets a character and echoes it
.ORIG x3000
LOOP:
    GETC           ; Read a character from keyboard
    OUT            ; Echo the character
    BR LOOP        ; Repeat (BR is an alias for BRnzp)
.END"#,
    ),
    (
        "Simple Counter",
        r#"; Program that counts from 1 to 5
.ORIG x3000
AND R1, R1, #0     ; Clear R1 (counter)
ADD R1, R1, #1     ; Start at 1
LD R3, ASCII_0     ; Load base char for print

LOOP:
    ADD R0, R1, R3  ; Convert to ASCII ('0' + number)
    OUT             ; Print the digit

    ADD R1, R1, #1  ; Increment counter
    ADD R2, R1, #-6 ; Check if counter > 5
    BRn LOOP        ; Continue if negative

HALT               ; Stop execution

ASCII_0 .FILL x30
.END"#,
    ),
    (
        "Fibonacci Sequence",
        r#"; Calculate first 8 Fibonacci numbers
.ORIG x3000
AND R0, R0, #0     ; F(0) = 0
AND R1, R1, #0     ; F(1) = 1
ADD R1, R1, #1

AND R3, R3, #0     ; Counter
ADD R3, R3, #8     ; Calculate 8 numbers

LOOP:
    ADD R2, R0, R1  ; F(n) = F(n-1) + F(n-2)
    ADD R0, R1, #0  ; Shift: F(n-2) = F(n-1)
    ADD R1, R2, #0  ; Shift: F(n-1) = F(n)

    ADD R3, R3, #-1 ; Decrement counter
    BRp LOOP        ; Continue if positive

HALT
.END"#,
    ),
    (
        "String Length Calculator",
        r#"; Calculate length of a null-terminated string
.ORIG x3000
LEA R1, STRING     ; Load string address
AND R0, R0, #0     ; Clear counter

LOOP:
    LDR R2, R1, #0  ; Load character
    BRz DONE        ; If null terminator, done
    ADD R0, R0, #1  ; Increment counter
    ADD R1, R1, #1  ; Next character
    BR LOOP

DONE:
    ; String length is now in R0
    HALT

STRING: .STRINGZ "Hello LC-3!"
.END"#,
    ),
    (
        "Memory Fill Pattern",
        r#"; Fill memory locations with a pattern
.ORIG x3000
LEA R1, DATA_START ; Base address
AND R2, R2, #0     ; Clear pattern
ADD R2, R2, #10    ; Set pattern value
AND R3, R3, #0     ; Counter
ADD R3, R3, #5     ; Fill 5 locations

FILL_LOOP:
    STR R2, R1, #0  ; Store pattern
    ADD R1, R1, #1  ; Next address
    ADD R2, R2, #1  ; Increment pattern
    ADD R3, R3, #-1 ; Decrement counter
    BRp FILL_LOOP   ; Continue if positive

HALT

DATA_START: .BLKW 5 ; Reserve 5 words
.END"#,
    ),
    (
        "Subroutine Example",
        r#"; Program demonstrating subroutine usage
.ORIG x3000
AND R0, R0, #0
ADD R0, R0, #7     ; Set R0 = 7
JSR DOUBLE         ; Call subroutine
HALT               ; R0 now contains 14

DOUBLE:
    ADD R0, R0, R0  ; Double the value in R0
    RET             ; Return to caller
.END"#,
    ),
    (
        "Conditional Branching Demo",
        r#"; Demonstrate different branch conditions
.ORIG x3000
AND R0, R0, #0
ADD R0, R0, #-5    ; Set R0 = -5
BRn NEGATIVE       ; Branch if negative

ZERO_OR_POS:
    LEA R0, POS_MSG
    PUTS
    BR END

NEGATIVE:
    LEA R0, NEG_MSG
    PUTS

END:
    HALT

NEG_MSG: .STRINGZ "Number is negative"
POS_MSG: .STRINGZ "Number is zero or positive"
.END"#,
    ),
];

#[cfg(test)]
mod tests {
    #[test]
    fn examples_assemble() {
        for (name, code) in super::EXAMPLES {
            assert!(
                crate::emulator::Emulator::parse_program(code, None).is_ok(),
                "{name} failed to assemble"
            );
        }
    }
}
