; ModuleID = '11_struct_with_array'
source_filename = "11_struct_with_array"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

declare i32 @getint()

declare i32 @getch()

declare i32 @getarray(ptr)

declare void @putint(i32)

declare void @putch(i32)

declare void @putarray(i32, ptr)

declare void @starttime()

declare void @stoptime()

define i32 @main() {
entry:
  %d = alloca { [3 x i32], i32 }, align 8
  store { [3 x i32], i32 } zeroinitializer, ptr %d, align 4
  %arr = getelementptr inbounds nuw { [3 x i32], i32 }, ptr %d, i32 0, i32 0
  %arr.gep = getelementptr [3 x i32], ptr %arr, i32 0, i32 0
  store i32 1, ptr %arr.gep, align 4
  %arr1 = getelementptr inbounds nuw { [3 x i32], i32 }, ptr %d, i32 0, i32 0
  %arr.gep2 = getelementptr [3 x i32], ptr %arr1, i32 0, i32 1
  store i32 2, ptr %arr.gep2, align 4
  %arr3 = getelementptr inbounds nuw { [3 x i32], i32 }, ptr %d, i32 0, i32 0
  %arr.gep4 = getelementptr [3 x i32], ptr %arr3, i32 0, i32 2
  store i32 3, ptr %arr.gep4, align 4
  %value = getelementptr inbounds nuw { [3 x i32], i32 }, ptr %d, i32 0, i32 1
  store i32 10, ptr %value, align 4
  %arr5 = getelementptr inbounds nuw { [3 x i32], i32 }, ptr %d, i32 0, i32 0
  %arr.gep6 = getelementptr [3 x i32], ptr %arr5, i32 0, i32 0
  %field = load i32, ptr %arr.gep6, align 4
  %arr7 = getelementptr inbounds nuw { [3 x i32], i32 }, ptr %d, i32 0, i32 0
  %arr.gep8 = getelementptr [3 x i32], ptr %arr7, i32 0, i32 1
  %field9 = load i32, ptr %arr.gep8, align 4
  %add = add i32 %field, %field9
  %arr10 = getelementptr inbounds nuw { [3 x i32], i32 }, ptr %d, i32 0, i32 0
  %arr.gep11 = getelementptr [3 x i32], ptr %arr10, i32 0, i32 2
  %field12 = load i32, ptr %arr.gep11, align 4
  %add13 = add i32 %add, %field12
  %value14 = getelementptr inbounds nuw { [3 x i32], i32 }, ptr %d, i32 0, i32 1
  %field15 = load i32, ptr %value14, align 4
  %add16 = add i32 %add13, %field15
  ret i32 %add16
}
