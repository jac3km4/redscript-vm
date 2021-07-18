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

## shell
This project includes a simple shell that you can use to build/test/debug your code.

The shell looks for a `redscript.toml` file in the directory it's being run from.
The file should look something like this:
```toml
# where to look for the compiled bytecode
bundle_path = "D:\\games\\Cyberpunk 2077\\r6\\cache\\final.redscripts.bk"
# where to look for project sources ("src" is the default)
source_dir = "src"
# where to look for test sources ("test" is the default)
test_dir = "test"
```

After the shell starts, you can try defining a `src/main.reds` file:
```swift
func main() {
    Log("Hello world");
}
```
You can then invoke it from the shell:
```
>> runMain
Hello world
```
You can also invoke scripts from the compiled bytecode:
```
>> run GetFunFact
Crocodile poop used to be used as a contraception
```
The shell comes with a basic test framework too.
You can use it to test your mods against the game by defining some test suites, for instance a `test/myModSuite.reds`:
```swift
public class MyModTestSuite {
  public func SpawnVehicleFlagShouldBeTrue() {
    let system = new PreventionSystem();
    AssertEq(system.ShouldSpawnVehicle(), true);
  }
}
```
You can run your test suite with a shell command:
```
>> test MyModTestSuite
+  spawn vehicle flag should be true
```
