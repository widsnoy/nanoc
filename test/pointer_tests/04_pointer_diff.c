// Test pointer subtraction (pointer - pointer)
int main() {
    int arr[10] = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9};
    int *p1 = &arr[2];
    int *p2 = &arr[7];
    
    // pointer - pointer should give element count difference
    int diff = p2 - p1;  // 7 - 2 = 5
    
    return diff;
}
