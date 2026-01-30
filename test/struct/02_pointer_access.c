// 指针成员访问 (->)
struct Point {
    int x;
    int y;
};

int main() {
    struct Point p;
    struct Point *ptr = &p;
    ptr->x = 100;
    ptr->y = 200;
    return ptr->x + ptr->y;  // 应返回 300
}
