// 嵌套成员访问
struct Inner {
    int value;
};

struct Outer {
    struct Inner inner;
    int other;
};

int main() {
    struct Outer o;
    o.inner.value = 42;
    o.other = 8;
    return o.inner.value + o.other;  // 应返回 50
}
