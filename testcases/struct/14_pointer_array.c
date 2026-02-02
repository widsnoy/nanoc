// 指针数组
struct Point {
    int x;
    int y;
};

int main() {
    struct Point p1;
    struct Point p2;
    struct Point p3;
    
    p1.x = 1; p1.y = 2;
    p2.x = 3; p2.y = 4;
    p3.x = 5; p3.y = 6;
    
    struct Point *ptrs[3];
    ptrs[0] = &p1;
    ptrs[1] = &p2;
    ptrs[2] = &p3;
    
    int sum = 0;
    int i = 0;
    while (i < 3) {
        sum = sum + ptrs[i]->x + ptrs[i]->y;
        i = i + 1;
    }
    return sum;  // 应返回 21
}
