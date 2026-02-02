// Test int *const *const (const pointer to const pointer to int)
int main() {
    int a = 50;
    int *const p = &a;
    int *const *const pp = &p;
    
    // Can read through the chain
    int v1 = **pp;  // 50
    
    // Can modify the int value through the chain
    **pp = 60;
    int v2 = a;     // 60
    
    return v1 + v2; // 50 + 60 = 110
}
