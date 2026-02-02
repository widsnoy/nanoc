// Test address-of operator &
int main() {
    int a = 5;
    int b = 10;
    int *p;
    
    // Take address of a
    p = &a;
    int v1 = *p;
    
    // Take address of b
    p = &b;
    int v2 = *p;
    
    return v1 + v2; // 5 + 10 = 15
}
