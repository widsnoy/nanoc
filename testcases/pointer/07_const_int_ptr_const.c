// Test const int *const (const pointer to const int)
// Neither the pointer nor the value can be modified
int main() {
    int a = 42;
    const int *const p = &a;  // const pointer to const int
    
    int v = *p;  // read is OK
    
    return v;  // 42
}
