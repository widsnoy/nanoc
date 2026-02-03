# Performance Test Cases

这些测试用例来自 [PKU MiniC compiler-dev-test-cases](https://github.com/pku-minic/compiler-dev-test-cases/tree/master/testcases/perf)，已从 C 语言翻译成 Airyc 语言。

## 测试用例列表

### Bitset 操作 (00-02)
- `00_bitset1.airy` - 基础位集操作
- `01_bitset2.airy` - 位集操作变体 2
- `02_bitset3.airy` - 位集操作变体 3

### 矩阵乘法 (03-05)
- `03_mm1.airy` - 矩阵乘法实现 1
- `04_mm2.airy` - 矩阵乘法实现 2
- `05_mm3.airy` - 矩阵乘法实现 3

### 矩阵向量乘法 (06-08)
- `06_mv1.airy` - 矩阵向量乘法 1
- `07_mv2.airy` - 矩阵向量乘法 2
- `08_mv3.airy` - 矩阵向量乘法 3

### 稀疏矩阵向量乘法 (09-11)
- `09_spmv1.airy` - 稀疏矩阵向量乘法 1
- `10_spmv2.airy` - 稀疏矩阵向量乘法 2
- `11_spmv3.airy` - 稀疏矩阵向量乘法 3

### 快速傅里叶变换 (12-14)
- `12_fft0.airy` - FFT 实现 0
- `13_fft1.airy` - FFT 实现 1
- `14_fft2.airy` - FFT 实现 2

### 矩阵转置 (15-17)
- `15_transpose0.airy` - 矩阵转置 0
- `16_transpose1.airy` - 矩阵转置 1
- `17_transpose2.airy` - 矩阵转置 2

### Brainfuck 解释器 (18-19)
- `18_brainfuck-bootstrap.airy` - Brainfuck 引导程序
- `19_brainfuck-calculator.airy` - Brainfuck 计算器

## 文件说明

每个测试用例包含三个文件：
- `.airy` - Airyc 源代码
- `.in` - 输入数据
- `.out` - 期望输出

## 翻译说明

从 C/SysY 到 Airyc 的主要语法转换：
- `int` → `i32`
- `float` → `f32`
- `int x;` → `let x: i32;`
- `int x[10];` → `let x: [i32; 10];`
- `int func(int x)` → `fn func(x: i32) -> i32`
- `int x[][N]` → `x: *mut [i32; N]` (函数参数)
