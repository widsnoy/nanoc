// struct 中包含数组字段的初始化（常量）
struct Data {
    int arr[3];
    int value;
};

int main() {
    const struct Data d = {{1, 2, 3}, 10};
    return d.arr[0] + d.arr[1] + d.arr[2] + d.value;  // 应返回 16
}
