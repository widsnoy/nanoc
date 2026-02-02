// Test complex pointer types: int **const (const pointer to pointer to int)
int main() {
    int a = 10;
    int b = 20;
    int *p1 = &a;
    int *p2 = &b;
    
    int **const pp = &p1;  // const pointer to pointer to int
    
    int v1 = **pp;  // 10
    
    // Can modify what pp points to (the pointer p1)
    *pp = p2;
    int v2 = **pp;  // 20
    
    return v1 + v2;  // 10 + 20 = 30
}
