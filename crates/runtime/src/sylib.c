#include "sylib.h"
#include <stdarg.h>
#include <stdio.h>
#include <sys/time.h>
/* Input & output functions */
int getint() {
  int t;
  scanf("%d", &t);
  return t;
}
int getch() {
  char c;
  scanf("%c", &c);
  return (int)c;
}
int getarray(int a[]) {
  int n;
  scanf("%d", &n);
  for (int i = 0; i < n; i++)
    scanf("%d", &a[i]);
  return n;
}
void putint(int a) { printf("%d", a); }
void putch(int a) { printf("%c", a); }
void putarray(int n, int a[]) {
  printf("%d:", n);
  for (int i = 0; i < n; i++)
    printf(" %d", a[i]);
  printf("\n");
}

void starttime() { _sysy_starttime(__LINE__); }

void stoptime() { _sysy_stoptime(__LINE__); }

/* Timing function implementation */
__attribute((constructor)) void before_main() {}
__attribute((destructor)) void after_main() {}
void _sysy_starttime(int lineno) {
  _sysy_l1[_sysy_idx] = lineno;
  gettimeofday(&_sysy_start, NULL);
}
void _sysy_stoptime(int lineno) {
  gettimeofday(&_sysy_end, NULL);
  _sysy_l2[_sysy_idx] = lineno;
  _sysy_us[_sysy_idx] += 1000000 * (_sysy_end.tv_sec - _sysy_start.tv_sec) +
                         _sysy_end.tv_usec - _sysy_start.tv_usec;
  _sysy_s[_sysy_idx] += _sysy_us[_sysy_idx] / 1000000;
  _sysy_us[_sysy_idx] %= 1000000;
  _sysy_m[_sysy_idx] += _sysy_s[_sysy_idx] / 60;
  _sysy_s[_sysy_idx] %= 60;
  _sysy_h[_sysy_idx] += _sysy_m[_sysy_idx] / 60;
  _sysy_m[_sysy_idx] %= 60;
  _sysy_idx++;
}
