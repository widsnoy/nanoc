// Test const int* (pointer to const int)
// The pointer can be changed, but the value it points to cannot be modified through it
int main() {
    int a = 10;
    int b = 20;
    const int *p = &a;  // pointer to const int
    
    int v1 = *p;  // read is OK
    
    // p can be reassigned to point to different address
    p = &b;
    int v2 = *p;
    
    return v1 + v2;  // 10 + 20 = 30
}
