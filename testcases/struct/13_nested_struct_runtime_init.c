// 嵌套 struct 运行时初始化
struct Inner {
    int a;
    int b;
};

struct Outer {
    struct Inner inner;
    int c;
};

int main() {
    int x = 5;
    int y = 10;
    int z = 15;
    struct Outer o = {{x, y}, z};
    return o.inner.a + o.inner.b + o.c;  // 应返回 30
}
