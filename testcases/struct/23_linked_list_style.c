// struct 指针链表风格访问
struct Node {
  int value;
  struct Node *next;
};

int main() {
  struct Node n1;
  struct Node n2;
  struct Node n3;

  n1.value = 10;
  n1.next = &n2;

  n2.value = 20;
  n2.next = &n3;

  n3.value = 30;
  n3.next = 0; // null

  // 遍历链表
  int sum = 0;
  struct Node *current = &n1;
  while (current) {
    sum = sum + current->value;
    current = current->next;
  }
  return sum; // 应返回 60
}
