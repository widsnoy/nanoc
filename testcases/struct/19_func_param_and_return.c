// struct 作为函数参数（通过指针修改）并返回指针
struct Point {
    int x;
    int y;
};

struct Point* modify_and_return(struct Point *p) {
    p->x = p->x * 2;
    p->y = p->y * 2;
    return p;
}

int main() {
    struct Point p = {10, 20};
    struct Point *result = modify_and_return(&p);
    return result->x + result->y;  // 应返回 60
}
