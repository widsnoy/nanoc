// Test &*ptr == ptr (address of dereference)
int main() {
    int a = 100;
    int *p = &a;
    
    // &*p should equal p
    int *q = &*p;
    
    int v1 = *p;   // 100
    int v2 = *q;   // 100
    
    // Modify through q
    *q = 200;
    int v3 = *p;   // 200
    
    return v1 + v2 + v3;  // 100 + 100 + 200 = 400
}
