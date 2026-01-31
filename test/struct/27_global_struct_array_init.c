// struct 数组初始化（全局常量）
struct Point {
    int x;
    int y;
};

const struct Point g_points[3] = {{1, 2}, {3, 4}, {5, 6}};

int main() {
    int sum = 0;
    int i = 0;
    while (i < 3) {
        sum = sum + g_points[i].x + g_points[i].y;
        i = i + 1;
    }
    return sum;  // 应返回 21
}
