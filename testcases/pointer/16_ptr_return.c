// Test pointer return from function
int global_val = 42;

int *get_global_ptr() {
    return &global_val;
}

int main() {
    int *p = get_global_ptr();
    int v1 = *p;  // 42
    
    *p = 84;
    int v2 = global_val;  // 84
    
    return v1 + v2;  // 42 + 84 = 126
}
