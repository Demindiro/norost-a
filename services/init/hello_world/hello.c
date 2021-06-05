#include <fcntl.h>

int main() {
	write(0, "Hello, world!\n", 14);
	return 0;
}
