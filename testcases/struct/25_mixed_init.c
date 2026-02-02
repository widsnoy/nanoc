// 混合初始化：部分常量部分运行时
struct Point {
    int x;
    int y;
};

int main() {
    int runtime_val = 25;
    struct Point p = {10, runtime_val};
    return p.x + p.y;  // 应返回 35
}
