// 嵌套 struct 初始化（常量）
struct Inner {
    int a;
    int b;
};

struct Outer {
    struct Inner inner;
    int c;
};

int main() {
    const struct Outer o = {{10, 20}, 30};
    return o.inner.a + o.inner.b + o.c;  // 应返回 60
}
