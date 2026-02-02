// struct 包含二维数组
struct Matrix {
    int data[2][3];
    int rows;
    int cols;
};

int main() {
    struct Matrix m;
    m.rows = 2;
    m.cols = 3;
    
    int i = 0;
    while (i < 2) {
        int j = 0;
        while (j < 3) {
            m.data[i][j] = i * 3 + j + 1;
            j = j + 1;
        }
        i = i + 1;
    }
    
    // 计算所有元素之和
    int sum = 0;
    i = 0;
    while (i < m.rows) {
        int j = 0;
        while (j < m.cols) {
            sum = sum + m.data[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    return sum;  // 应返回 21 (1+2+3+4+5+6)
}
