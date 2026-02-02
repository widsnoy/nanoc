// struct 中包含指针数组
struct Container {
    int *ptrs[3];
    int count;
};

int main() {
    int a = 10;
    int b = 20;
    int c = 30;
    
    struct Container cont;
    cont.ptrs[0] = &a;
    cont.ptrs[1] = &b;
    cont.ptrs[2] = &c;
    cont.count = 3;
    
    int sum = 0;
    int i = 0;
    while (i < cont.count) {
        sum = sum + *cont.ptrs[i];
        i = i + 1;
    }
    return sum;  // 应返回 60
}
