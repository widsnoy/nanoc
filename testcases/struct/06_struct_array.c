// struct 数组
struct Point {
    int x;
    int y;
};

int main() {
    struct Point points[3];
    points[0].x = 1;
    points[0].y = 2;
    points[1].x = 3;
    points[1].y = 4;
    points[2].x = 5;
    points[2].y = 6;
    
    int sum = 0;
    int i = 0;
    while (i < 3) {
        sum = sum + points[i].x + points[i].y;
        i = i + 1;
    }
    return sum;  // 应返回 21
}
