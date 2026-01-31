// struct 中包含数组字段
struct Data {
    int arr[3];
    int value;
};

int main() {
    struct Data d;
    d.arr[0] = 1;
    d.arr[1] = 2;
    d.arr[2] = 3;
    d.value = 10;
    return d.arr[0] + d.arr[1] + d.arr[2] + d.value;  // 应返回 16
}
