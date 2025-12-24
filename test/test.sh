#!/bin/bash
testdir="./compiler-dev-test-cases/testcases"
my_compiler="../target/debug/nanoc-compiler"
runtime_lib="../target/debug/libnanoc_runtime.a"

PLAY_GROUND="../playground"
RESULT_FILE="$PLAY_GROUND/result.txt"

mkdir -p "$PLAY_GROUND/ll"
rm -f $RESULT_FILE
touch $RESULT_FILE

for i in "$@"; do
    for i in "$testdir/lv$i"/*.c; do
        name=$(basename "${i%.c}")
        clang -Wno-implicit-function-declaration $i $runtime_lib -o $PLAY_GROUND/std.out &> /dev/null
        
        # 可能有 .in
        if [ -f "${i%.c}.in" ]; then
            cat "${i%.c}.in" | $PLAY_GROUND/std.out > $PLAY_GROUND/ans.txt
        else
            $PLAY_GROUND/std.out > $PLAY_GROUND/ans.txt
        fi
        
        std_return_value="return: $?"
        echo $std_return_value >> $PLAY_GROUND/ans.txt
        
        rm $PLAY_GROUND/std.out

        $my_compiler $i -o $PLAY_GROUND/$name.ll
        if [ $? -ne 0 ]; then
            exit 1
        fi

        clang -x ir $PLAY_GROUND/$name.ll -c -o $PLAY_GROUND/$name.o
        clang $PLAY_GROUND/$name.o $runtime_lib -o $PLAY_GROUND/my.out
        
        # 可能有 .in
        if [ -f "${i%.c}.in" ]; then
             cat "${i%.c}.in" | $PLAY_GROUND/my.out > $PLAY_GROUND/my.txt
        else
            $PLAY_GROUND/my.out > $PLAY_GROUND/my.txt
        fi
        
        my_return_value="return: $?"
        echo $my_return_value >> $PLAY_GROUND/my.txt
        rm $PLAY_GROUND/my.out
        rm $PLAY_GROUND/$name.o
        mv $PLAY_GROUND/$name.ll $PLAY_GROUND/ll/$name.ll

        diff $PLAY_GROUND/ans.txt $PLAY_GROUND/my.txt > $PLAY_GROUND/diff.txt
        diff_result=$?

        if [ $diff_result -eq 0 ]; then
            echo -e "\033[32m$name: Passed\033[0m"
        else
            echo -e "\033[31m$name: Failed\033[0m"
            echo "$name:" >> $RESULT_FILE
            cat $PLAY_GROUND/diff.txt >> $RESULT_FILE
        fi
    done
done

cat $RESULT_FILE

if [ -s $RESULT_FILE ]; then
    exit 1
else
    exit 0
fi