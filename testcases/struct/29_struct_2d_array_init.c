// struct 包含二维数组的初始化
struct Matrix {
    int data[2][3];
    int sum;
};

int main() {
    const struct Matrix m = {{{1, 2, 3}, {4, 5, 6}}, 21};
    
    int computed_sum = 0;
    int i = 0;
    while (i < 2) {
        int j = 0;
        while (j < 3) {
            computed_sum = computed_sum + m.data[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    
    // 验证 computed_sum 等于 m.sum
    if (computed_sum == m.sum) {
        return m.sum;  // 应返回 21
    }
    return 0;
}
