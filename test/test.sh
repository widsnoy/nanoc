#!/bin/bash
testdir="./compiler-dev-test-cases/testcases"
my_compiler="../target/debug/airyc-compiler"
runtime_lib="../target/debug/libairyc_runtime.a"

PLAY_GROUND="../playground"
RESULT_FILE="$PLAY_GROUND/result.txt"

cargo build --workspace

mkdir -p "$PLAY_GROUND/ll"
rm -f $RESULT_FILE
touch $RESULT_FILE

for level in "$@"; do
    for i in $testdir/$level/*.c; do
        name=$(basename "${i%.c}")
        clang -Wno-implicit-function-declaration $i $runtime_lib -o $PLAY_GROUND/std.out 
        # 可能有 .in
        if [ -f "${i%.c}.in" ]; then
            cat "${i%.c}.in" | $PLAY_GROUND/std.out > $PLAY_GROUND/ans.txt
        else
            $PLAY_GROUND/std.out > $PLAY_GROUND/ans.txt
        fi
        
        std_return_value="return: $?"
        echo $std_return_value >> $PLAY_GROUND/ans.txt
        
        rm $PLAY_GROUND/std.out

        $my_compiler -i $i -o $PLAY_GROUND -r $runtime_lib -O o0
        mv $PLAY_GROUND/$name $PLAY_GROUND/my.out
        if [ $? -ne 0 ]; then
            exit 1
        fi

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

        diff $PLAY_GROUND/ans.txt $PLAY_GROUND/my.txt > $PLAY_GROUND/diff.txt
        diff_result=$?

        if [ $diff_result -eq 0 ]; then
            echo -e "\033[32m$name: Passed\033[0m"
        else
            echo -e "\033[31m$name: Failed\033[0m - $i"
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
