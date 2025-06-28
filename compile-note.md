# LLVM Binary Generation

## Quick Start

1. **Start the calculator**:
   ```bash
   cargo run
   ```

2. **Define a function**:
   ```
   >> fn square(x) { x * x }
   ```

3. **Compile to executable**:
   ```
   >> :compile square my_square_app 7.0
   ✓ Executable created successfully
   ```

4. **Run your binary**:
   ```bash
   ./my_square_app
   ```

## More Examples

### Compile Simple Expressions
```
>> :compile_expr "10 + 5 * 3" math_calc
>> ./math_calc
```

### Multi-parameter Functions
```
>> fn area_circle(radius) { 3.14159 * radius * radius }
>> :compile area_circle circle_calc 5.0
>> ./circle_calc
```

### Complex Functions
```
>> fn fibonacci_approx(n) { (1.618 * n - 0.618) / 2.236 }
>> :compile fibonacci_approx fib_calc 10.0
>> ./fib_calc
```

## Features
- ✅ Native executable generation
- ✅ LLVM optimization
- ✅ Cross-platform support
- ✅ Function parameter binding
- ✅ Expression compilation
- ✅ Automatic linking

## Files Generated
All executables are created in the current directory with full execution permissions. 
