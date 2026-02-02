// 复杂嵌套：struct 包含 struct 数组
struct Point {
    int x;
    int y;
};

struct Triangle {
    struct Point vertices[3];
};

int main() {
    struct Triangle t;
    t.vertices[0].x = 0; t.vertices[0].y = 0;
    t.vertices[1].x = 10; t.vertices[1].y = 0;
    t.vertices[2].x = 5; t.vertices[2].y = 10;
    
    int sum = 0;
    int i = 0;
    while (i < 3) {
        sum = sum + t.vertices[i].x + t.vertices[i].y;
        i = i + 1;
    }
    return sum;  // 应返回 25
}
