#!/usr/bin/env bash

for h in `find include/ ! -path 'include/pthread/*.h' -name *.h -printf '%P\n'`
do
	echo "#include \"$h\""
done

echo

TEST_FNS=`grep -r -G -H -h -o '__std_test_.* ' src/`
for fn in $TEST_FNS
do
	echo "void $fn;"
done

echo

echo "int main() {"
for fn in $TEST_FNS
do
	echo "puts(\"testing $fn\\n\\\"\\\"\\\"\");" | sed 's/(void)//g'
	echo "$fn;" | sed 's/void//g'
	echo "puts(\"\\\"\\\"\\\"\\n\");"
done
echo "return 0;"
echo "}"
