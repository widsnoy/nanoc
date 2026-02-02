// Test int + pointer (commutative)
int main() {
    int arr[5] = {10, 20, 30, 40, 50};
    int *p = arr;
    
    // int + pointer should work same as pointer + int
    int v1 = *(p + 2);   // 30
    int v2 = *(2 + p);   // 30
    
    return v1 + v2;  // 30 + 30 = 60
}
