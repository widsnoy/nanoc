// const struct 初始化（编译时常量）
struct Point {
    int x;
    int y;
};

int main() {
    const struct Point p = {10, 20};
    return p.x + p.y;  // 应返回 30
}
