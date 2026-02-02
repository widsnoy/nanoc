// Test pointer to pointer (int **)
int main() {
    int a = 100;
    int *p = &a;
    int **pp = &p;
    
    // Access through double pointer
    int v1 = **pp;  // 100
    
    // Modify through double pointer
    **pp = 200;
    int v2 = a;     // 200
    
    return v1 + v2; // 100 + 200 = 300
}
