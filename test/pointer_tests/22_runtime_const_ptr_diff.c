// Test pointer difference with runtime const pointers
int main() {
  int na, nb;
  const int *const a = &na;
  const int *const b = &nb;
  int p = a - b;
  putint(p);
  return 0;
}
