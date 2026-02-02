// Test const pointer array with runtime values
int main() {
    int a = 42;
    const int *p = &a;
    const int *b[2] = {p, p};
    const int c[2] = {*p, *p + 1};
    
    // b[0] should point to a
    if (*b[0] != 42) return 1;
    if (*b[1] != 42) return 2;
    
    // c should contain values
    if (c[0] != 42) return 3;
    if (c[1] != 43) return 4;
    
    return 0;
}
