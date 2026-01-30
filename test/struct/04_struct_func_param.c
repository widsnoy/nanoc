// struct 作为函数参数（通过指针）
struct Point {
    int x;
    int y;
};

void set_point(struct Point *p, int x, int y) {
    p->x = x;
    p->y = y;
}

int get_sum(struct Point *p) {
    return p->x + p->y;
}

int main() {
    struct Point p;
    set_point(&p, 15, 25);
    return get_sum(&p);  // 应返回 40
}
