;##############################################################################
;#
;# simpleos.asm -- a simple LC-3 operating system aiming to provide the bare
;# minimum for a LC3 machine. This *should* be a drop in replacement for
;# the OS packaged with the lc3tools software. TODO: This is not true at the
;# moment, I will remove this message once correctly implemented and tested
;#
;# DISCLAMER: this OS attemps to support any lc3 emulator but is designed to
;# be used by the emulator packaged with the OS. TODO: test os with lc3tools
;#
;# Copyright (c) 2025 Jack Crump-Leys (jackcrumpleys@gmail.com)
;#
;# This program is free software: you can redistribute it and/or modify
;# it under the terms of the GNU Affero General Public License as published
;# by the Free Software Foundation, either version 3 of the License, or
;# (at your option) any later version.
;#
;# This program is distributed in the hope that it will be useful,
;# but WITHOUT ANY WARRANTY; without even the implied warranty of
;# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
;# GNU Affero General Public License for more details.
;#
;# You should have received a copy of the GNU Affero General Public License
;# along with this program.  If not, see <https://www.gnu.org/licenses/>.
;#
;##############################################################################

        .ORIG x0000

; TRAP vector table
        .FILL BAD_TRAP   ; x00
        .FILL BAD_TRAP   ; x01
        .FILL BAD_TRAP   ; x02
        .FILL BAD_TRAP   ; x03
        .FILL BAD_TRAP   ; x04
        .FILL BAD_TRAP   ; x05
        .FILL BAD_TRAP   ; x06
        .FILL BAD_TRAP   ; x07
        .FILL BAD_TRAP   ; x08
        .FILL BAD_TRAP   ; x09
        .FILL BAD_TRAP   ; x0A
        .FILL BAD_TRAP   ; x0B
        .FILL BAD_TRAP   ; x0C
        .FILL BAD_TRAP   ; x0D
        .FILL BAD_TRAP   ; x0E
        .FILL BAD_TRAP   ; x0F
        .FILL BAD_TRAP   ; x10
        .FILL BAD_TRAP   ; x11
        .FILL BAD_TRAP   ; x12
        .FILL BAD_TRAP   ; x13
        .FILL BAD_TRAP   ; x14
        .FILL BAD_TRAP   ; x15
        .FILL BAD_TRAP   ; x16
        .FILL BAD_TRAP   ; x17
        .FILL BAD_TRAP   ; x18
        .FILL BAD_TRAP   ; x19
        .FILL BAD_TRAP   ; x1A
        .FILL BAD_TRAP   ; x1B
        .FILL BAD_TRAP   ; x1C
        .FILL BAD_TRAP   ; x1D
        .FILL BAD_TRAP   ; x1E
        .FILL BAD_TRAP   ; x1F
        .FILL TRAP_GETC  ; x20 - Last keyboard input -> R0
        .FILL TRAP_OUT   ; x21 - R0 -> output (one char)
        .FILL TRAP_PUTS  ; x22 - Write each char starting at mem[R0] until we get to a null
        .FILL TRAP_IN    ; x23 - Prompt the user for a char of input
        .FILL TRAP_PUTSP ; x24
        .FILL TRAP_HALT  ; x25
    	.FILL BAD_TRAP   ; x26
    	.FILL BAD_TRAP   ; x27
    	.FILL BAD_TRAP   ; x28
    	.FILL BAD_TRAP   ; x29
    	.FILL BAD_TRAP   ; x2A
    	.FILL BAD_TRAP   ; x2B
    	.FILL BAD_TRAP   ; x2C
    	.FILL BAD_TRAP   ; x2D
    	.FILL BAD_TRAP   ; x2E
        .FILL BAD_TRAP   ; x2F
    	.FILL BAD_TRAP   ; x30
    	.FILL BAD_TRAP   ; x31
    	.FILL BAD_TRAP   ; x32
    	.FILL BAD_TRAP   ; x33
    	.FILL BAD_TRAP   ; x34
    	.FILL BAD_TRAP   ; x35
    	.FILL BAD_TRAP   ; x36
    	.FILL BAD_TRAP   ; x37
    	.FILL BAD_TRAP   ; x38
    	.FILL BAD_TRAP   ; x39
    	.FILL BAD_TRAP   ; x3A
    	.FILL BAD_TRAP   ; x3B
    	.FILL BAD_TRAP   ; x3C
    	.FILL BAD_TRAP   ; x3D
    	.FILL BAD_TRAP   ; x3E
    	.FILL BAD_TRAP   ; x3F
    	.FILL BAD_TRAP   ; x40
    	.FILL BAD_TRAP   ; x41
    	.FILL BAD_TRAP   ; x42
    	.FILL BAD_TRAP   ; x43
    	.FILL BAD_TRAP   ; x44
    	.FILL BAD_TRAP   ; x45
    	.FILL BAD_TRAP   ; x46
    	.FILL BAD_TRAP   ; x47
    	.FILL BAD_TRAP   ; x48
    	.FILL BAD_TRAP   ; x49
    	.FILL BAD_TRAP   ; x4A
    	.FILL BAD_TRAP   ; x4B
    	.FILL BAD_TRAP   ; x4C
    	.FILL BAD_TRAP   ; x4D
    	.FILL BAD_TRAP   ; x4E
    	.FILL BAD_TRAP   ; x4F
    	.FILL BAD_TRAP   ; x50
    	.FILL BAD_TRAP   ; x51
    	.FILL BAD_TRAP   ; x52
    	.FILL BAD_TRAP   ; x53
    	.FILL BAD_TRAP   ; x54
    	.FILL BAD_TRAP   ; x55
    	.FILL BAD_TRAP   ; x56
    	.FILL BAD_TRAP   ; x57
    	.FILL BAD_TRAP   ; x58
    	.FILL BAD_TRAP   ; x59
    	.FILL BAD_TRAP   ; x5A
    	.FILL BAD_TRAP   ; x5B
    	.FILL BAD_TRAP   ; x5C
    	.FILL BAD_TRAP   ; x5D
    	.FILL BAD_TRAP   ; x5E
    	.FILL BAD_TRAP   ; x5F
    	.FILL BAD_TRAP   ; x60
    	.FILL BAD_TRAP   ; x61
    	.FILL BAD_TRAP   ; x62
    	.FILL BAD_TRAP   ; x63
    	.FILL BAD_TRAP   ; x64
    	.FILL BAD_TRAP   ; x65
    	.FILL BAD_TRAP   ; x66
    	.FILL BAD_TRAP   ; x67
    	.FILL BAD_TRAP   ; x68
    	.FILL BAD_TRAP   ; x69
    	.FILL BAD_TRAP   ; x6A
    	.FILL BAD_TRAP   ; x6B
    	.FILL BAD_TRAP   ; x6C
    	.FILL BAD_TRAP   ; x6D
    	.FILL BAD_TRAP   ; x6E
    	.FILL BAD_TRAP   ; x6F
    	.FILL BAD_TRAP   ; x70
    	.FILL BAD_TRAP   ; x71
    	.FILL BAD_TRAP   ; x72
    	.FILL BAD_TRAP   ; x73
    	.FILL BAD_TRAP   ; x74
    	.FILL BAD_TRAP   ; x75
    	.FILL BAD_TRAP   ; x76
    	.FILL BAD_TRAP   ; x77
    	.FILL BAD_TRAP   ; x78
    	.FILL BAD_TRAP   ; x79
    	.FILL BAD_TRAP   ; x7A
    	.FILL BAD_TRAP   ; x7B
    	.FILL BAD_TRAP   ; x7C
    	.FILL BAD_TRAP   ; x7D
    	.FILL BAD_TRAP   ; x7E
    	.FILL BAD_TRAP   ; x7F
    	.FILL BAD_TRAP   ; x80
    	.FILL BAD_TRAP   ; x81
    	.FILL BAD_TRAP   ; x82
    	.FILL BAD_TRAP   ; x83
    	.FILL BAD_TRAP   ; x84
    	.FILL BAD_TRAP   ; x85
    	.FILL BAD_TRAP   ; x86
    	.FILL BAD_TRAP   ; x87
    	.FILL BAD_TRAP   ; x88
    	.FILL BAD_TRAP   ; x89
    	.FILL BAD_TRAP   ; x8A
    	.FILL BAD_TRAP   ; x8B
    	.FILL BAD_TRAP   ; x8C
    	.FILL BAD_TRAP   ; x8D
    	.FILL BAD_TRAP   ; x8E
    	.FILL BAD_TRAP   ; x8F
    	.FILL BAD_TRAP   ; x90
    	.FILL BAD_TRAP   ; x91
    	.FILL BAD_TRAP   ; x92
    	.FILL BAD_TRAP   ; x93
    	.FILL BAD_TRAP   ; x94
    	.FILL BAD_TRAP   ; x95
    	.FILL BAD_TRAP   ; x96
    	.FILL BAD_TRAP   ; x97
    	.FILL BAD_TRAP   ; x98
    	.FILL BAD_TRAP   ; x99
    	.FILL BAD_TRAP   ; x9A
    	.FILL BAD_TRAP   ; x9B
    	.FILL BAD_TRAP   ; x9C
    	.FILL BAD_TRAP   ; x9D
    	.FILL BAD_TRAP   ; x9E
    	.FILL BAD_TRAP   ; x9F
    	.FILL BAD_TRAP   ; xA0
    	.FILL BAD_TRAP   ; xA1
    	.FILL BAD_TRAP   ; xA2
    	.FILL BAD_TRAP   ; xA3
    	.FILL BAD_TRAP   ; xA4
    	.FILL BAD_TRAP   ; xA5
    	.FILL BAD_TRAP   ; xA6
    	.FILL BAD_TRAP   ; xA7
    	.FILL BAD_TRAP   ; xA8
    	.FILL BAD_TRAP   ; xA9
    	.FILL BAD_TRAP   ; xAA
    	.FILL BAD_TRAP   ; xAB
    	.FILL BAD_TRAP   ; xAC
    	.FILL BAD_TRAP   ; xAD
    	.FILL BAD_TRAP   ; xAE
    	.FILL BAD_TRAP   ; xAF
    	.FILL BAD_TRAP   ; xB0
    	.FILL BAD_TRAP   ; xB1
    	.FILL BAD_TRAP   ; xB2
    	.FILL BAD_TRAP   ; xB3
    	.FILL BAD_TRAP   ; xB4
    	.FILL BAD_TRAP   ; xB5
    	.FILL BAD_TRAP   ; xB6
    	.FILL BAD_TRAP   ; xB7
    	.FILL BAD_TRAP   ; xB8
    	.FILL BAD_TRAP   ; xB9
    	.FILL BAD_TRAP   ; xBA
    	.FILL BAD_TRAP   ; xBB
    	.FILL BAD_TRAP   ; xBC
    	.FILL BAD_TRAP   ; xBD
    	.FILL BAD_TRAP   ; xBE
    	.FILL BAD_TRAP   ; xBF
    	.FILL BAD_TRAP   ; xC0
    	.FILL BAD_TRAP   ; xC1
    	.FILL BAD_TRAP   ; xC2
    	.FILL BAD_TRAP   ; xC3
    	.FILL BAD_TRAP   ; xC4
    	.FILL BAD_TRAP   ; xC5
    	.FILL BAD_TRAP   ; xC6
    	.FILL BAD_TRAP   ; xC7
    	.FILL BAD_TRAP   ; xC8
    	.FILL BAD_TRAP   ; xC9
    	.FILL BAD_TRAP   ; xCA
    	.FILL BAD_TRAP   ; xCB
    	.FILL BAD_TRAP   ; xCC
    	.FILL BAD_TRAP   ; xCD
    	.FILL BAD_TRAP   ; xCE
    	.FILL BAD_TRAP   ; xCF
    	.FILL BAD_TRAP   ; xD0
    	.FILL BAD_TRAP   ; xD1
    	.FILL BAD_TRAP   ; xD2
    	.FILL BAD_TRAP   ; xD3
    	.FILL BAD_TRAP   ; xD4
    	.FILL BAD_TRAP   ; xD5
    	.FILL BAD_TRAP   ; xD6
    	.FILL BAD_TRAP   ; xD7
    	.FILL BAD_TRAP   ; xD8
    	.FILL BAD_TRAP   ; xD9
    	.FILL BAD_TRAP   ; xDA
    	.FILL BAD_TRAP   ; xDB
    	.FILL BAD_TRAP   ; xDC
    	.FILL BAD_TRAP   ; xDD
    	.FILL BAD_TRAP   ; xDE
    	.FILL BAD_TRAP   ; xDF
    	.FILL BAD_TRAP   ; xE0
    	.FILL BAD_TRAP   ; xE1
    	.FILL BAD_TRAP   ; xE2
    	.FILL BAD_TRAP   ; xE3
    	.FILL BAD_TRAP   ; xE4
    	.FILL BAD_TRAP   ; xE5
    	.FILL BAD_TRAP   ; xE6
    	.FILL BAD_TRAP   ; xE7
    	.FILL BAD_TRAP   ; xE8
    	.FILL BAD_TRAP   ; xE9
    	.FILL BAD_TRAP   ; xEA
    	.FILL BAD_TRAP   ; xEB
    	.FILL BAD_TRAP   ; xEC
    	.FILL BAD_TRAP   ; xED
    	.FILL BAD_TRAP   ; xEE
    	.FILL BAD_TRAP   ; xEF
    	.FILL BAD_TRAP   ; xF0
    	.FILL BAD_TRAP   ; xF1
    	.FILL BAD_TRAP   ; xF2
    	.FILL BAD_TRAP   ; xF3
    	.FILL BAD_TRAP   ; xF4
    	.FILL BAD_TRAP   ; xF5
    	.FILL BAD_TRAP   ; xF6
    	.FILL BAD_TRAP   ; xF7
    	.FILL BAD_TRAP   ; xF8
    	.FILL BAD_TRAP   ; xF9
    	.FILL BAD_TRAP   ; xFA
    	.FILL BAD_TRAP   ; xFB
    	.FILL BAD_TRAP   ; xFC
    	.FILL BAD_TRAP   ; xFD
    	.FILL BAD_TRAP   ; xFE
    	.FILL BAD_TRAP   ; xFF

; the interrupt vector table
        .FILL INT_PRIV   ; x00 - attempted to execute `RTI`
        .FILL INT_ILL    ; x01 - attempted to run invalid instruction
        .FILL INT_ACV    ; x02 - attempted to accsess outside of permissions
        .FILL BAD_INT    ; x03
        .FILL KBDINT     ; x04 - keyboard interrupt
        .FILL BAD_INT    ; x05
        .FILL BAD_INT    ; x06
        .FILL BAD_INT    ; x07
        .FILL BAD_INT    ; x08
        .FILL BAD_INT    ; x09
        .FILL BAD_INT    ; x0A
        .FILL BAD_INT    ; x0B
        .FILL BAD_INT    ; x0C
        .FILL BAD_INT    ; x0D
        .FILL BAD_INT    ; x0E
        .FILL BAD_INT    ; x0F
        .FILL BAD_INT    ; x10
        .FILL BAD_INT    ; x11
        .FILL BAD_INT    ; x12
        .FILL BAD_INT    ; x13
        .FILL BAD_INT    ; x14
        .FILL BAD_INT    ; x15
        .FILL BAD_INT    ; x16
        .FILL BAD_INT    ; x17
        .FILL BAD_INT    ; x18
        .FILL BAD_INT    ; x19
        .FILL BAD_INT    ; x1A
        .FILL BAD_INT    ; x1B
        .FILL BAD_INT    ; x1C
        .FILL BAD_INT    ; x1D
        .FILL BAD_INT    ; x1E
        .FILL BAD_INT    ; x1F
        .FILL BAD_INT    ; x20
        .FILL BAD_INT    ; x21
        .FILL BAD_INT    ; x22
        .FILL BAD_INT    ; x23
        .FILL BAD_INT    ; x24
        .FILL BAD_INT    ; x25
        .FILL BAD_INT    ; x26
        .FILL BAD_INT    ; x27
        .FILL BAD_INT    ; x28
        .FILL BAD_INT    ; x29
        .FILL BAD_INT    ; x2A
        .FILL BAD_INT    ; x2B
        .FILL BAD_INT    ; x2C
        .FILL BAD_INT    ; x2D
        .FILL BAD_INT    ; x2E
        .FILL BAD_INT    ; x2F
        .FILL BAD_INT    ; x30
        .FILL BAD_INT    ; x31
        .FILL BAD_INT    ; x32
        .FILL BAD_INT    ; x33
        .FILL BAD_INT    ; x34
        .FILL BAD_INT    ; x35
        .FILL BAD_INT    ; x36
        .FILL BAD_INT    ; x37
        .FILL BAD_INT    ; x38
        .FILL BAD_INT    ; x39
        .FILL BAD_INT    ; x3A
        .FILL BAD_INT    ; x3B
        .FILL BAD_INT    ; x3C
        .FILL BAD_INT    ; x3D
        .FILL BAD_INT    ; x3E
        .FILL BAD_INT    ; x3F
        .FILL BAD_INT    ; x40
        .FILL BAD_INT    ; x41
        .FILL BAD_INT    ; x42
        .FILL BAD_INT    ; x43
        .FILL BAD_INT    ; x44
        .FILL BAD_INT    ; x45
        .FILL BAD_INT    ; x46
        .FILL BAD_INT    ; x47
        .FILL BAD_INT    ; x48
        .FILL BAD_INT    ; x49
        .FILL BAD_INT    ; x4A
        .FILL BAD_INT    ; x4B
        .FILL BAD_INT    ; x4C
        .FILL BAD_INT    ; x4D
        .FILL BAD_INT    ; x4E
        .FILL BAD_INT    ; x4F
        .FILL BAD_INT    ; x50
        .FILL BAD_INT    ; x51
        .FILL BAD_INT    ; x52
        .FILL BAD_INT    ; x53
        .FILL BAD_INT    ; x54
        .FILL BAD_INT    ; x55
        .FILL BAD_INT    ; x56
        .FILL BAD_INT    ; x57
        .FILL BAD_INT    ; x58
        .FILL BAD_INT    ; x59
        .FILL BAD_INT    ; x5A
        .FILL BAD_INT    ; x5B
        .FILL BAD_INT    ; x5C
        .FILL BAD_INT    ; x5D
        .FILL BAD_INT    ; x5E
        .FILL BAD_INT    ; x5F
        .FILL BAD_INT    ; x60
        .FILL BAD_INT    ; x61
        .FILL BAD_INT    ; x62
        .FILL BAD_INT    ; x63
        .FILL BAD_INT    ; x64
        .FILL BAD_INT    ; x65
        .FILL BAD_INT    ; x66
        .FILL BAD_INT    ; x67
        .FILL BAD_INT    ; x68
        .FILL BAD_INT    ; x69
        .FILL BAD_INT    ; x6A
        .FILL BAD_INT    ; x6B
        .FILL BAD_INT    ; x6C
        .FILL BAD_INT    ; x6D
        .FILL BAD_INT    ; x6E
        .FILL BAD_INT    ; x6F
        .FILL BAD_INT    ; x70
        .FILL BAD_INT    ; x71
        .FILL BAD_INT    ; x72
        .FILL BAD_INT    ; x73
        .FILL BAD_INT    ; x74
        .FILL BAD_INT    ; x75
        .FILL BAD_INT    ; x76
        .FILL BAD_INT    ; x77
        .FILL BAD_INT    ; x78
        .FILL BAD_INT    ; x79
        .FILL BAD_INT    ; x7A
        .FILL BAD_INT    ; x7B
        .FILL BAD_INT    ; x7C
        .FILL BAD_INT    ; x7D
        .FILL BAD_INT    ; x7E
        .FILL BAD_INT    ; x7F
        .FILL BAD_INT    ; x80
        .FILL BAD_INT    ; x81
        .FILL BAD_INT    ; x82
        .FILL BAD_INT    ; x83
        .FILL BAD_INT    ; x84
        .FILL BAD_INT    ; x85
        .FILL BAD_INT    ; x86
        .FILL BAD_INT    ; x87
        .FILL BAD_INT    ; x88
        .FILL BAD_INT    ; x89
        .FILL BAD_INT    ; x8A
        .FILL BAD_INT    ; x8B
        .FILL BAD_INT    ; x8C
        .FILL BAD_INT    ; x8D
        .FILL BAD_INT    ; x8E
        .FILL BAD_INT    ; x8F
        .FILL BAD_INT    ; x90
        .FILL BAD_INT    ; x91
        .FILL BAD_INT    ; x92
        .FILL BAD_INT    ; x93
        .FILL BAD_INT    ; x94
        .FILL BAD_INT    ; x95
        .FILL BAD_INT    ; x96
        .FILL BAD_INT    ; x97
        .FILL BAD_INT    ; x98
        .FILL BAD_INT    ; x99
        .FILL BAD_INT    ; x9A
        .FILL BAD_INT    ; x9B
        .FILL BAD_INT    ; x9C
        .FILL BAD_INT    ; x9D
        .FILL BAD_INT    ; x9E
        .FILL BAD_INT    ; x9F
        .FILL BAD_INT    ; xA0
        .FILL BAD_INT    ; xA1
        .FILL BAD_INT    ; xA2
        .FILL BAD_INT    ; xA3
        .FILL BAD_INT    ; xA4
        .FILL BAD_INT    ; xA5
        .FILL BAD_INT    ; xA6
        .FILL BAD_INT    ; xA7
        .FILL BAD_INT    ; xA8
        .FILL BAD_INT    ; xA9
        .FILL BAD_INT    ; xAA
        .FILL BAD_INT    ; xAB
        .FILL BAD_INT    ; xAC
        .FILL BAD_INT    ; xAD
        .FILL BAD_INT    ; xAE
        .FILL BAD_INT    ; xAF
        .FILL BAD_INT    ; xB0
        .FILL BAD_INT    ; xB1
        .FILL BAD_INT    ; xB2
        .FILL BAD_INT    ; xB3
        .FILL BAD_INT    ; xB4
        .FILL BAD_INT    ; xB5
        .FILL BAD_INT    ; xB6
        .FILL BAD_INT    ; xB7
        .FILL BAD_INT    ; xB8
        .FILL BAD_INT    ; xB9
        .FILL BAD_INT    ; xBA
        .FILL BAD_INT    ; xBB
        .FILL BAD_INT    ; xBC
        .FILL BAD_INT    ; xBD
        .FILL BAD_INT    ; xBE
        .FILL BAD_INT    ; xBF
        .FILL BAD_INT    ; xC0
        .FILL BAD_INT    ; xC1
        .FILL BAD_INT    ; xC2
        .FILL BAD_INT    ; xC3
        .FILL BAD_INT    ; xC4
        .FILL BAD_INT    ; xC5
        .FILL BAD_INT    ; xC6
        .FILL BAD_INT    ; xC7
        .FILL BAD_INT    ; xC8
        .FILL BAD_INT    ; xC9
        .FILL BAD_INT    ; xCA
        .FILL BAD_INT    ; xCB
        .FILL BAD_INT    ; xCC
        .FILL BAD_INT    ; xCD
        .FILL BAD_INT    ; xCE
        .FILL BAD_INT    ; xCF
        .FILL BAD_INT    ; xD0
        .FILL BAD_INT    ; xD1
        .FILL BAD_INT    ; xD2
        .FILL BAD_INT    ; xD3
        .FILL BAD_INT    ; xD4
        .FILL BAD_INT    ; xD5
        .FILL BAD_INT    ; xD6
        .FILL BAD_INT    ; xD7
        .FILL BAD_INT    ; xD8
        .FILL BAD_INT    ; xD9
        .FILL BAD_INT    ; xDA
        .FILL BAD_INT    ; xDB
        .FILL BAD_INT    ; xDC
        .FILL BAD_INT    ; xDD
        .FILL BAD_INT    ; xDE
        .FILL BAD_INT    ; xDF
        .FILL BAD_INT    ; xE0
        .FILL BAD_INT    ; xE1
        .FILL BAD_INT    ; xE2
        .FILL BAD_INT    ; xE3
        .FILL BAD_INT    ; xE4
        .FILL BAD_INT    ; xE5
        .FILL BAD_INT    ; xE6
        .FILL BAD_INT    ; xE7
        .FILL BAD_INT    ; xE8
        .FILL BAD_INT    ; xE9
        .FILL BAD_INT    ; xEA
        .FILL BAD_INT    ; xEB
        .FILL BAD_INT    ; xEC
        .FILL BAD_INT    ; xED
        .FILL BAD_INT    ; xEE
        .FILL BAD_INT    ; xEF
        .FILL BAD_INT    ; xF0
        .FILL BAD_INT    ; xF1
        .FILL BAD_INT    ; xF2
        .FILL BAD_INT    ; xF3
        .FILL BAD_INT    ; xF4
        .FILL BAD_INT    ; xF5
        .FILL BAD_INT    ; xF6
        .FILL BAD_INT    ; xF7
        .FILL BAD_INT    ; xF8
        .FILL BAD_INT    ; xF9
        .FILL BAD_INT    ; xFA
        .FILL BAD_INT    ; xFB
        .FILL BAD_INT    ; xFC
        .FILL BAD_INT    ; xFD
        .FILL BAD_INT    ; xFE
        .FILL BAD_INT    ; xFF

;------------------------------------------------------------------------------
; OS memory locations and constants
;------------------------------------------------------------------------------

OS_START    ; machine starts executing at x0200
        ; Initialize stack pointer
        LD R6, OS_SP

        LEA R0, OS_START_MSG     ; print welcome message
        PUTS

        ; Set up interrupt enable bits in KBSR
        LDI R0, OS_KBSR
        LD  R1, KB_IE_MASK

        ; Perform bitwise or R2 <- R0 OR R1
        JSR BITWISE_OR
        STI R0, OS_KBSR

        ; Enable interrupts globally
        LD  R0, PSR_MASK_ENABLE_INT
        JSR STACK_PUSH

        ; Push starting PSR onto stack
        LD R0, STARTING_PSR
        JSR STACK_PUSH

        ; Push the first address of the user program onto stack
        LD R0, USER_START
        JSR STACK_PUSH

        ; clear r0 and r1
        AND R0, R0, #0
        AND R1, R1, #0

        RTI ; This will pop our pc and psr and use it to run the program

OS_START_MSG    .STRINGZ "Simple LC-3 OS v1.0\n\n"

; Device register addresses
OS_KBSR     .FILL xFE00  ; keyboard status register
OS_KBDR     .FILL xFE02  ; keyboard data register
OS_DSR      .FILL xFE04  ; display status register
OS_DDR      .FILL xFE06  ; display data register
OS_PSR      .FILL xFFFC  ; processor status register
OS_MCR      .FILL xFFFE  ; machine control register

; Useful constants
MASK_HI         .FILL x7FFF
LOW_8_BITS      .FILL x00FF
PSR_MASK_ENABLE_INT   .FILL x8000  ; enable all interrupts
KB_IE_MASK      .FILL x4000  ; keyboard interrupt enable
STARTING_PSR    .FILL x8001  ; starting PSR value (Z=1, USERMODE)

USER_START      .FILL x3000  ; default user program start

; OS stack at the end of OS space (note init with 0 size
; so then pushing we will actually store at 0x2FFF)
OS_SP       .FILL x3000

SEED        .FILL xABCD  ; seed for random number generation

;-----------------------------------------------------------------------------
; OS only Utils
;-----------------------------------------------------------------------------

BITWISE_OR ; R0 IS INPUT1/OUTPUT, R1 IS INPUT2
        NOT R0, R0      ; ~R0
        NOT R1, R1      ; ~R1
        AND R0, R0, R1  ; (~R0 & ~R1) = ~(R0 | R1)
        NOT R0, R0      ; ~(~R0 & ~R1) = R0 | R1
        NOT R1, R1      ; Restore R1
        RET

STACK_PUSH ; R0 IS INPUT
        ADD R6, R6, #-1
        STR R0, R6, #0
        RET

STACK_POP ; R0 IS OUTPUT
        LDR R0, R6, #0
        ADD R6, R6 , #1
        RET

;------------------------------------------------------------------------------
; Standard LC-3 Trap Routines (from original OS)
;------------------------------------------------------------------------------

TRAP_GETC
        LDI R0, OS_KBSR      ; wait for a keystroke
        BRzp TRAP_GETC
        LDI R0, OS_KBDR      ; read it and return
        RTI

TRAP_OUT
        ADD R6, R6, #-1      ; push R1
        STR R1, R6, #0
TRAP_OUT_WAIT
        LDI R1, OS_DSR       ; wait for the display to be ready
        BRzp TRAP_OUT_WAIT
        STI R0, OS_DDR       ; write the character and return
        LDR R1, R6, #0       ; pop R1
        ADD R6, R6, #1
        RTI

TRAP_PUTS
        ADD R6, R6, #-1      ; push R0
        STR R0, R6, #0
        ADD R6, R6, #-1      ; push R1
        STR R1, R6, #0
        ADD R6, R6, #-1      ; push R7
        STR R7, R6, #0
        ADD R1, R0, #0       ; move string pointer (R0) into R1

TRAP_PUTS_LOOP
        LDR R0, R1, #0       ; write characters in string using OUT
        BRz TRAP_PUTS_DONE
        OUT
        ADD R1, R1, #1
        BRnzp TRAP_PUTS_LOOP

TRAP_PUTS_DONE
        LDR R7, R6, #0       ; pop R7
        ADD R6, R6, #1
        LDR R1, R6, #0       ; pop R1
        ADD R6, R6, #1
        LDR R0, R6, #0       ; pop R0
        ADD R6, R6, #1
        RTI

TRAP_IN
        ADD R6, R6, #-1      ; push R7
        STR R7, R6, #0
        LEA R0, TRAP_IN_MSG  ; prompt for input
        PUTS
        GETC                 ; read a character
        OUT                  ; echo back to monitor
        ADD R6, R6, #-1      ; push R0 (the character)
        STR R0, R6, #0
        AND R0, R0, #0       ; write a linefeed, too
        ADD R0, R0, #10
        OUT
        LDR R0, R6, #0       ; pop R0
        ADD R6, R6, #1
        LDR R7, R6, #0       ; pop R7
        ADD R6, R6, #1
        RTI

TRAP_PUTSP
	; NOTE: This trap will end when it sees any NUL, even in
	; packed form, despite the P&P second edition's requirement
	; of a double NUL.

        ADD R6, R6, #-1
	STR R0,R6,#0 		; Push R0 
        ADD R6, R6, #-1
	STR R1,R6,#0            ; Push R1 
        ADD R6, R6, #-1
	STR R2,R6,#0            ; Push R2 
        ADD R6, R6, #-1
	STR R3,R6,#0            ; Push R3 
        ADD R6, R6, #-1
	STR R7,R6,#0            ; Push R7 
	ADD R1,R0,#0		; move string pointer (R0) into R1

TRAP_PUTSP_LOOP
	LDR R2,R1,#0		; read the next two characters
	LD R0,LOW_8_BITS	; use mask to get low byte
	AND R0,R0,R2		; if low byte is NUL, quit printing
	BRz TRAP_PUTSP_DONE
	OUT			; otherwise print the low byte

	AND R0,R0,#0		; shift high byte into R0
	ADD R3,R0,#8
TRAP_PUTSP_S_LOOP
	ADD R0,R0,R0		; shift R0 left
	ADD R2,R2,#0		; move MSB from R2 into R0
	BRzp TRAP_PUTSP_MSB_0
	ADD R0,R0,#1
TRAP_PUTSP_MSB_0
	ADD R2,R2,R2		; shift R2 left
	ADD R3,R3,#-1
	BRp TRAP_PUTSP_S_LOOP

	ADD R0,R0,#0		; if high byte is NUL, quit printing
	BRz TRAP_PUTSP_DONE
	OUT			; otherwise print the low byte

	ADD R1,R1,#1		; and keep going
	BRnzp TRAP_PUTSP_LOOP

TRAP_PUTSP_DONE
	LDR R7,R6,#0
        ADD R6, R6, #1          ; Pop R7
	LDR R3,R6,#0
        ADD R6, R6, #1          ; Pop R3
	LDR R2,R6,#0
        ADD R6, R6, #1          ; Pop R2
	LDR R1,R6,#0
        ADD R6, R6, #1          ; Pop R1
	LDR R0,R6,#0
        ADD R6, R6, #1          ; Pop R0
	RTI

TRAP_HALT
        ; an infinite loop of lowering OS_MCR's MSB
        LEA R0, TRAP_HALT_MSG    ; give a warning
        PUTS
        AND R0, R0, #0           ; clear the MCR
        STI R0, OS_MCR
        BRnzp TRAP_HALT      ; HALT again...

; -------- trap constants -------
TRAP_IN_MSG     .STRINGZ "\nInput a character> "
TRAP_HALT_MSG   .STRINGZ "\n\n[OS] --- HALT ---\n\n"
BAD_TRAP_MSG    .STRINGZ "\n\n[OS] --- undefined trap executed ---\n\n"
CHARS_PER_ROW .FILL #80
SCREEN_LINES .FILL #24
; -------------------------------

;------------------------------------------------------------------------------
; Interrupt service routines
;------------------------------------------------------------------------------

; Keyboard interrupt handler
KBDINT
        ADD R6, R6, #-1      ; push R0
        STR R0, R6, #0
        ADD R6, R6, #-1      ; push R1
        STR R1, R6, #0
        ADD R6, R6, #-1      ; push R7
        STR R7, R6, #0

        ; Read the key to clear the interrupt
        LDI R0, OS_KBDR

        ; You could do something with the key here
        ; For now, we just acknowledge the interrupt

        LDR R7, R6, #0       ; pop R7
        ADD R6, R6, #1
        LDR R1, R6, #0       ; pop R1
        ADD R6, R6, #1
        LDR R0, R6, #0       ; pop R0
        ADD R6, R6, #1
        RTI

;------------------------------------------------------------------------------
; Error handling routines
;------------------------------------------------------------------------------

BAD_TRAP
        ; print an error message, then HALT
        LEA R0, BAD_TRAP_MSG     ; give an error message
        PUTS
        HALT

BAD_INT
		LEA R1, ERROR_INT
		BRNZP ERR_MSG

INT_PRIV
		; The userspace program has attempted to run RTI
		LEA R1, ERROR_PRIV
		BRNZP ERR_MSG
INT_ILL
		; The userspace program has supplied an invalid instruction
		LEA R1, ERROR_ILL
		BRNZP ERR_MSG
INT_ACV
		; The userspace program has attempted out of bounds read/write/jmp
		LEA R1, ERROR_ACV
		BRNZP ERR_MSG

; at the moment lets just output an error and halt,
; later we can handle more gracefully
ERR_MSG ; Uses pointer at r1 to print an error
		LEA R0, ERROR_TEMPLATE
		PUTS
		ADD R0, R1, #0
		PUTS
		HALT

;------------------------------------------------------------------------------
; Constants and messages
;------------------------------------------------------------------------------


NEWLINE         .FILL x000A  ; newline character
NEG_SIGN        .FILL x002D  ; ASCII '-'
ZERO_CHAR       .FILL x0030  ; ASCII '0

ERROR_TEMPLATE  .STRINGZ "\n[OS] Error in program: "
ERROR_PRIV	  .STRINGZ "Attempted to run op `RTI` in user mode."

; TODO: The os should collect some more info here and print more usefully
ERROR_ILL	   .STRINGZ "Invalid instruction"
ERROR_ACV	   .STRINGZ "Attempted to access out of usermode permissions" ; Here as well
ERROR_INT	   .STRINGZ "Bad interrupt (probably not your fault)"

.END
