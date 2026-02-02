// Test int *const (const pointer to int)
// The pointer itself cannot be changed, but the value it points to can be modified
int main() {
    int a = 10;
    int *const p = &a;  // const pointer to int
    
    int v1 = *p;  // read: 10
    
    // Can modify the value through the pointer
    *p = 25;
    int v2 = *p;  // read: 25
    
    return v1 + v2;  // 10 + 25 = 35
}
