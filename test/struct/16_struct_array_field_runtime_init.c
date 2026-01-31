// struct 中包含数组字段的运行时初始化
struct Data {
    int arr[3];
    int value;
};

int main() {
    int a = 1;
    int b = 2;
    int c = 3;
    int v = 10;
    struct Data d = {{a, b, c}, v};
    return d.arr[0] + d.arr[1] + d.arr[2] + d.value;  // 应返回 16
}
