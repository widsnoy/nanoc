// 全局 const struct
struct Point {
    int x;
    int y;
};

const struct Point g_point = {100, 200};

int main() {
    return g_point.x + g_point.y;  // 应返回 300
}
