// Test pointer increment in loop
int main() {
    int arr[5] = {1, 2, 3, 4, 5};
    int *p = arr;
    int sum = 0;
    int i = 0;
    
    while (i < 5) {
        sum = sum + *p;
        p = p + 1;
        i = i + 1;
    }
    
    return sum;  // 1 + 2 + 3 + 4 + 5 = 15
}
