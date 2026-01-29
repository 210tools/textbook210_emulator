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
- **Step-by-Step Execution**: Execute one instruction or micro-operation at a time
- **CPU State Visualization**: See the processor cycle in action with color-coded state transitions
- **Breakpoints**: Set breakpoints at specific memory addresses
- **Machine Code Display**: View the assembled binary representation of your program

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

Every small task uses different panes, you can drag pains around by their top tab. Here is the basics:

1. **Write Code**: Use the Editor pane to write your LC-3 assembly code
2. **Compile**: Click "Reset & Compile" to assemble your program \[Editor pane]
3. **Run**: Use the control buttons to: \[control pane]
   - "Run" - Execute continuously
   - "Pause" - Stop execution
   - "Step" - Execute one full instruction
   - "Small Step" - Execute one micro-operation
4. **Debug**: Use the Memory, Registers, and CPU State panes to inspect program state
5. **Set Breakpoints**: Click the 🛑 button next to a line in the Machine Code view (TODO: move this to memory pane)


### TODO: Add better help
**The handy help pane has infomation on each pane and LC3 in general**


## 📋 Technical Details

The emulator implements:
- All standard LC-3 instructions
- Fetch-decode-execute cycle with visible micro-states
- Memory-mapped I/O
- Privilege levels and memory protection
- Trap routines for system services
- Exception handling
- Full OS support with a simple OS implementing everything the OS from the UOA COMPSCI-210 class

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
