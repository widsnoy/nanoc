// struct 中包含指针字段
struct Node {
    int value;
    int *ptr;
};

int main() {
    int x = 100;
    struct Node n;
    n.value = 50;
    n.ptr = &x;
    return n.value + *n.ptr;  // 应返回 150
}
