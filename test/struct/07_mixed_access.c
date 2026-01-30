// 混合使用 . 和 -> 访问
struct Inner {
    int value;
};

struct Outer {
    struct Inner inner;
    struct Inner *inner_ptr;
};

int main() {
    struct Inner i;
    i.value = 10;
    
    struct Outer o;
    o.inner.value = 20;
    o.inner_ptr = &i;
    
    return o.inner.value + o.inner_ptr->value;  // 应返回 30
}
