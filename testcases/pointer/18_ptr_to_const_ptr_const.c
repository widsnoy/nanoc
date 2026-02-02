// Test const int * const * (pointer to const pointer to const int)
int main() {
    int a = 11;
    int b = 22;
    const int *p1 = &a;
    const int *p2 = &b;
    
    const int * const * pp = &p1;
    
    int v1 = **pp;  // 11
    
    // Can change pp to point to different const int*
    pp = &p2;
    int v2 = **pp;  // 22
    
    return v1 + v2;  // 11 + 22 = 33
}
