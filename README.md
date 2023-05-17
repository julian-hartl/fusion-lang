# Fusion-Lang

## Roadmap

### 07.04.2023

- [x] Add support for basic arithmetic operations
  ```rust
  7 - (30 + 7) * 8 / 2
  ```
### 08.04.2023

- [x] Add support for `let` statements

 ```
 let x = 30 * (8 - 1)
 let y = 30
 let z = x + y
 ```

- [x] Add error reporting

### 09.04.2023

- [ ] Add if statements

 ```
 let x = 30 
 let b = if x > 10 {
     x = 10
     10
 } else {
     x = 0
     2
 }
 ```

What do we have to consider?

- New type: boolean
- Conditional executing of statements
- Scoping

- [ ] Add while loops

 ```
 let x = 0
 while x < 10 {
     x = x + 1
 }
 ```

- [ ] Add scoping

 ```
 let x = 0
 {
     let x = 10
 }
 ```

### Next stream

- [ ] Add types & type checking

 ```
 let x: int = 10
 let y: bool = false
 let z: string = "Hello World"
 let a = 10 // type inference => will be int
 ```

- [ ] IR Lowering

 ```
 let x = 10
 let y = 20
 if x > y {
     x = 20
 } else {
     x = 10
 }
 ...
 ```

 ```
 func main() {
     x = 10
     y = 20
     gotoIfFalse x > y else
     x = 20
     goto end
     else: 
     x = 10
     end:
 }
 ...
 ```

- [ ] Add strings

 ```
 let hello_world = "Hello world\""
 ```

# Building LLVM 15.0.0 for Rust Compiler on Mac M1

This guide provides instructions on how to build LLVM 15.0.0 for the Rust compiler on a Mac M1.

## Prerequisites

- Xcode and Command Line Tools
- CMake and Ninja (can be installed via Homebrew)

## Step-by-step Instructions

### 1. Clone the LLVM project repository

```sh
git clone https://github.com/llvm/llvm-project.git
```

### 2. Checkout the specific LLVM version (15.0.0)

```sh
cd llvm-project
git checkout llvmorg-15.0.0
```

### 3. Configure and build LLVM, Clang, and other required sub-projects

   ```sh
   mkdir build
   cd build
   cmake -G "Xcode" -DLLVM_ENABLE_PROJECTS="clang;lld" -DLLVM_TARGETS_TO_BUILD="AArch64" -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr/local ../llvm
   ```

### 4. Build LLVM and the sub-projects

   ```sh
   cmake --build . --config Release -- -jobs $(sysctl -n hw.logicalcpu)5. Run the tests (optional)
   ```

### 5. Run the tests (optional)

```sh
cmake --build . --config Release --target check-all
```

### 6. Install LLVM and the sub-projects

   ```sh
   sudo cmake --build . --config Release --target install.sh
```

### 7. Set LLVM_SYS_150_PREFIX environment variable

```sh
export LLVM_SYS_150_PREFIX=/usr/local
```


### Troubleshooting

#### Unable to parse result of llvm-config --system-libs: was "/opt/homebrew/lib/libzstd.1.5.2.dylib"

1. Check `opt/homebrew/lib` and look for a file with a similar name but other version
2. Create a symlink from the given to the required version
```shell
ln -sf /opt/homebrew/lib/libzstd.1.5.5.dylib /opt/homebrew/lib/libzstd.1.5.4.dylib
```

