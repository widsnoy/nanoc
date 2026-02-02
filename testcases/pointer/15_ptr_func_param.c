// Test pointer as function parameter
int sum_via_ptr(int *a, int *b) {
    return *a + *b;
}

int modify_via_ptr(int *p, int val) {
    *p = val;
    return *p;
}

int main() {
    int x = 10;
    int y = 20;
    
    int s = sum_via_ptr(&x, &y);  // 30
    
    int m = modify_via_ptr(&x, 100);  // 100
    
    return s + m + x;  // 30 + 100 + 100 = 230
}
