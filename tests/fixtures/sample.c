#include <stdlib.h>

/* TODO: free memory */
void alloc_thing() {
    void *p = malloc(64);
    (void)p;
}

// HACK: platform-specific
int platform_val() {
    return 1;
}
