// 多层嵌套 struct 初始化（常量）
struct Level3 {
    int value;
};

struct Level2 {
    struct Level3 l3;
    int data;
};

struct Level1 {
    struct Level2 l2;
    int info;
};

int main() {
    const struct Level1 l1 = {{{10}, 20}, 30};
    return l1.l2.l3.value + l1.l2.data + l1.info;  // 应返回 60
}
