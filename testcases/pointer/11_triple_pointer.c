// Test triple pointer (int ***)
int main() {
    int a = 7;
    int *p = &a;
    int **pp = &p;
    int ***ppp = &pp;
    
    // Access through triple pointer
    int v1 = ***ppp;  // 7
    
    // Modify through triple pointer
    ***ppp = 14;
    int v2 = a;       // 14
    
    return v1 + v2;   // 7 + 14 = 21
}
