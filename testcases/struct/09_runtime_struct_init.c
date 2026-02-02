// 非常量 struct 初始化（运行时值）
struct Point {
    int x;
    int y;
};

int main() {
    int a = 15;
    int b = 25;
    struct Point p = {a, b};
    return p.x + p.y;  // 应返回 40
}
