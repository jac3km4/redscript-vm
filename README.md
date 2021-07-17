# redscript-vm
Lightweight and fast virtual machine for redscript.

## status
This project is a work-in-progress. It can already be used to run simple programs, but many features remain unimplemented. If you want to play with it, you can try out the WebAssembly version [here](https://try-redscript.surge.sh).

## features
- ✔️ arithmetic
- 🚧 arrays (implemented, but requires testing)
- ✔️ classes and polymorphism
- ✔️ incremental garbage collection
- ✔️ custom native functions
- ✔️ pinned values (out parameters)
- 🚧 structs (implemented, but all structs are boxed for now)
- 🚧 variants (implemented partially)
- ❌ debugger
- ❌ scripted value references
- ❌ statically sized arrays
