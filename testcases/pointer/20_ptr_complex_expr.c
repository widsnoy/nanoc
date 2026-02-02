// Test complex expression with pointers
int main() {
    int a = 5;
    int b = 10;
    int *pa = &a;
    int *pb = &b;
    
    // Complex expressions
    int v1 = *pa + *pb;           // 5 + 10 = 15
    int v2 = *pa * *pb;           // 5 * 10 = 50
    int v3 = (*pa + 1) * (*pb - 2);  // 6 * 8 = 48
    
    return v1 + v2 + v3;  // 15 + 50 + 48 = 113
}
