// 通过指针访问 struct 数组元素
struct Point {
    int x;
    int y;
};

int sum_points(struct Point *arr, int n) {
    int sum = 0;
    int i = 0;
    while (i < n) {
        sum = sum + arr[i].x + arr[i].y;
        i = i + 1;
    }
    return sum;
}

int main() {
    struct Point points[3];
    points[0].x = 1; points[0].y = 2;
    points[1].x = 3; points[1].y = 4;
    points[2].x = 5; points[2].y = 6;
    
    return sum_points(points, 3);  // 应返回 21
}
