# 整数计算器

> ⚠️ **AI 生成代码警告**  
> 本项目代码由 AI 助手（Kiro/Claude）生成，仅供学习和参考使用。

一个用 airyc 语言实现的交互式整数计算器，支持 +、-、*、/、% 五种运算。

## 语法

```peg
Expr   := Term (('+' | '-') Term)*
Term   := Factor (('*' | '/' | '%') Factor)*
Factor := Number | '(' Expr ')' | ('+' | '-') Factor
Number := [0-9]+
```

运算符优先级：
- 优先级 3 (最高): 一元 +, -
- 优先级 2: *, /, %
- 优先级 1 (最低): 二元 +, -
- 优先级 0: 括号 ()

## 编译和运行

```bash
# 编译
../../airyc/target/debug/airyc-cli calculator.airy stdlib.airy ast.airy lexer.airy parser.airy eval.airy 

./calculator
```

## 使用示例

```
> 1 + 2
3
> 10 * (5 - 3)
20
> 2 + 3 * 4 - 5
9
> 100 / 0
Error: division by zero
> quit
再见！
```

## 注意事项

1. **类型转换**：airyc 不支持有符号/无符号整数隐式转换，代码中使用 if-else 链手动转换
2. **常量导出**：常量不能被 import，使用返回常量值的函数代替
3. **Import 语法**：import 语句末尾不需要分号
4. **字面量类型**：所有字面量需显式标注类型（0i64, 1u8）
5. **限制**：仅支持 i64 整数运算，输入长度限制 256 字符，最多 128 个 token

## 文件结构

```
stdlib.airy   - C 标准库函数声明
ast.airy      - AST 定义、运算符常量
lexer.airy    - 词法分析器
parser.airy   - 递归下降语法分析器
eval.airy     - AST 求值器
main.airy     - REPL 主程序
calc.airy     - 单文件版本（备用）
```

## 参考

本项目参考了 [airyc](../../airyc/README.md) 的设计和语法。
