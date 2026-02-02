// 全局嵌套 struct 初始化
struct Inner {
    int a;
    int b;
};

struct Outer {
    struct Inner inner;
    int c;
};

const struct Outer g_outer = {{100, 200}, 300};

int main() {
    return g_outer.inner.a + g_outer.inner.b + g_outer.c;  // 应返回 600
}
