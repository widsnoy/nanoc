// 全局非 const struct 初始化
struct Point {
    int x;
    int y;
};

struct Point g_point = {50, 60};

int main() {
    g_point.x = g_point.x + 10;
    g_point.y = g_point.y + 20;
    return g_point.x + g_point.y;  // 应返回 140
}
