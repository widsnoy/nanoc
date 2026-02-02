// Test pointer arithmetic: pointer + int, pointer - int
int main() {
    int arr[5] = {10, 20, 30, 40, 50};
    int *p = &arr[0];
    
    int sum = 0;
    
    // pointer + int
    sum = sum + *(p + 0);  // 10
    sum = sum + *(p + 1);  // 20
    sum = sum + *(p + 2);  // 30
    
    // Move pointer forward
    p = p + 3;
    sum = sum + *p;        // 40
    
    // pointer - int
    p = p - 1;
    sum = sum + *p;        // 30
    
    return sum; // 10 + 20 + 30 + 40 + 30 = 130
}
