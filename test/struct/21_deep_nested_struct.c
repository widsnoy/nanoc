// 多层嵌套 struct
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
    struct Level1 l1;
    l1.l2.l3.value = 1;
    l1.l2.data = 2;
    l1.info = 3;
    return l1.l2.l3.value + l1.l2.data + l1.info;  // 应返回 6
}
