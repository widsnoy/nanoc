// Test pointer with array indexing
int main() {
    int arr[5] = {1, 2, 3, 4, 5};
    int *p = arr;  // array decays to pointer
    
    int sum = 0;
    sum = sum + p[0];  // 1
    sum = sum + p[1];  // 2
    sum = sum + p[2];  // 3
    sum = sum + p[3];  // 4
    sum = sum + p[4];  // 5
    
    return sum;  // 15
}
