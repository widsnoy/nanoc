// Test pointer arithmetic with negative offset
int main() {
    int arr[5] = {1, 2, 3, 4, 5};
    int *p = &arr[3];  // points to 4
    
    int v1 = *p;           // 4
    int v2 = *(p - 1);     // 3
    int v3 = *(p - 2);     // 2
    int v4 = *(p - 3);     // 1
    
    return v1 + v2 + v3 + v4;  // 4 + 3 + 2 + 1 = 10
}
