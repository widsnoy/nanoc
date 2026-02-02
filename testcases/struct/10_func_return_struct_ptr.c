// 函数返回 struct 指针
struct Point {
    int x;
    int y;
};

struct Point g_point;

struct Point* get_point() {
    return &g_point;
}

int main() {
    struct Point *p = get_point();
    p->x = 100;
    p->y = 200;
    return p->x + p->y;  // 应返回 300
}
