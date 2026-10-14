# LC-3 Emulator & UoA CompSci 210 tools

## EMAIL ME SUGGESTIONS AT jackcrumpleys@gmail.com!
## EMAIL ME INSULTS AT jackcrumpleys@gmail.com!

## ✨ Features

### 🔄 Full LC-3 Emulation
- Complete implementation of all LC-3 instructions
- Cycle-accurate simulation with visible micro-operations
- Privilege level support (User/Supervisor mode)
- Memory protection and exceptions handling
- Device I/O through keyboard and display registers

### 🔍 Debugging Capabilities
- **Step by instruction or by state**: Step over, into and out of instructions, or micro step one state of the book's state machine (one clock cycle) at a time, and step back a state
- **Datapath view**: Figure C.8 of the book, with each state's wires, parts and control signals lit as it runs
- **Breakpoints**: Click the editor gutter or the memory pane
- **Interrupts**: Pick a polling OS or an interrupt-driven OS (Run → Operating system) to compare how they read the keyboard
- **Editor**: Syntax highlighting, errors as you type, completion, hover docs, find and replace, several files at once

### 🧰 Additional Tools
- **Base Converter**: Convert between different number bases (binary, decimal, hex)
- **Terminal I/O**: Interact with your programs through a virtual terminal
- **Alot more to be done here**: Email ideas to jackcrumpleys@gmail.com!! (I will likely do everything relevant that is emailed to me)

## 🚀 Getting Started

### Online Version

The stable release can be found at [210tools.github.io](https://210tools.github.io).

A version of the website that is always up to date as long as the code compiles can be found [here](https://210tools.github.io/textbook210_emulator/).

### Offline Use

You can download the standalone application for offline use from the [releases page](https://github.com/210tools/textbook210_emulator/releases/tag/main). (Note that this is the bleeding edge and might be buggy)

## 📝 Quick Guide

Every small task uses different panes; drag panes around by their tabs, or add one with the + on a tab bar. The basics:

1. **Write code** in the editor pane
2. **Assemble** with Ctrl+B (or the Assemble button): the program is loaded and the machine resets
3. **Run** with the toolbar: Run/Pause, Over, Into, Out, Micro (one state) and Back (one state back)
4. **Inspect** the CPU and Memory panes, and View → Datapath to watch the datapath
5. **Breakpoints**: click next to a line number in the editor, or next to an address in the memory pane

The Help pane (and F1 for shortcuts) covers the rest.


## 📋 Technical Details

The emulator implements:
- All standard LC-3 instructions
- Fetch-decode-execute cycle with visible micro-states
- Memory-mapped I/O
- Privilege levels and memory protection
- Trap routines for system services
- Exception handling
- Full OS support with a simple OS implementing everything the OS from the UOA COMPSCI-210 class, in a polling and an interrupt-driven version
- Keyboard interrupts through the book's state machine (states 18, 49, 45, 37 to 55)

NOTE: The implementation of this emulator is based entirely on the 3rd edition of "Introduction to Computer Systems" by Yale n. patt and sanjay j. patel.

## 🔧 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### EMAIL ME SUGGESTIONS AT jackcrumpleys@gmail.com!
### EMAIL ME INSULTS AT jackcrumpleys@gmail.com!

## ⚖️ Important License Notice

This LC-3 Emulator & toolkit is licensed under the GNU Affero General Public License v3 (AGPL-3.0). This license requires that:

1. **Any derivative works must also be released under the AGPL-3.0**
2. **If you modify and use this software over a network (such as a web application), you MUST make the complete source code available to users**
3. **The source code must be made available even if the software is only being run as a service (e.g., on a web server) and not distributed as software**

This is different from other open source licenses - the AGPL specifically requires that if you modify the code and allow others to interact with it remotely, you must publish your modified source code.

For more details, please refer to the full license text included in the LICENCE file.

## 🙏 Acknowledgments

Created with passion by Jack Crump-Leys (jackcrumpleys@gmail.com) to support student learning in computer architecture courses.

Please email me support or hate :)
