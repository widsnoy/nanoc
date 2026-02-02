// Test basic pointer operations: declaration, assignment, dereference
int main() {
    int a = 10;
    int *p = &a;
    
    // Test dereference read
    int b = *p;
    
    // Test dereference write
    *p = 20;
    
    // Return sum to verify
    return a + b; // 20 + 10 = 30
}
