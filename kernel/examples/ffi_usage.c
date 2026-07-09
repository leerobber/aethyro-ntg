/*
 * Example: How to use aethyro-ntg FFI from C
 *
 * This demonstrates:
 * - Calling Rust ternary matmul from C
 * - Zero-copy interface (caller owns buffers)
 * - Collecting performance stats
 * - Error handling
 *
 * Compile:
 *   gcc -o ffi_usage ffi_usage.c -L../target/release -lntg_kernel -lm
 *
 * Run:
 *   ./ffi_usage
 */

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* C interface defined in kernel/include/ntg.h */
typedef struct {
    uint64_t latency_us;
    uint64_t memory_bytes;
    uint8_t simd_path;
    uint64_t timestamp_ns;
} OpStats;

/* FFI function declaration */
int ntg_matmul_ffi(
    const int8_t *a, uint32_t m, uint32_t k,
    const int8_t *b, uint32_t k_b, uint32_t n,
    float *out,
    OpStats *stats
);

uint64_t ntg_get_op_count(void);
void ntg_reset_op_count(void);

/* Helper: SIMD path name */
const char* get_simd_path_name(uint8_t path) {
    switch (path) {
        case 0: return "Scalar";
        case 1: return "AVX2";
        case 2: return "NEON";
        case 3: return "SSE4.1";
        default: return "Unknown";
    }
}

int main() {
    printf("=== Aethyro-NTG FFI Example ===\n\n");

    /* Example 1: Simple 2x2x2 matmul */
    printf("[Example 1] Simple 2x2x2 matmul\n");
    {
        int8_t a[4] = {1, -1, 0, 1};
        int8_t b[4] = {1, 0, -1, 1};
        float out[4] = {0};
        OpStats stats = {0};

        int result = ntg_matmul_ffi(
            a, 2, 2,
            b, 2, 2,
            out,
            &stats
        );

        if (result != 0) {
            printf("ERROR: matmul failed with code %d\n", result);
            return 1;
        }

        printf("  Result: [%f, %f, %f, %f]\n", out[0], out[1], out[2], out[3]);
        printf("  Latency: %lu us\n", stats.latency_us);
        printf("  Memory: %lu bytes\n", stats.memory_bytes);
        printf("  SIMD path: %s\n", get_simd_path_name(stats.simd_path));
        printf("  Total ops: %lu\n\n", ntg_get_op_count());
    }

    /* Example 2: Larger matrix (10x10x10) */
    printf("[Example 2] Larger matrix 10x10x10\n");
    {
        int size = 10;
        int8_t *a = (int8_t*)malloc(size * size * sizeof(int8_t));
        int8_t *b = (int8_t*)malloc(size * size * sizeof(int8_t));
        float *out = (float*)malloc(size * size * sizeof(float));
        OpStats stats = {0};

        /* Initialize with ternary values */
        for (int i = 0; i < size * size; i++) {
            a[i] = (i % 3) - 1;  /* -1, 0, 1 pattern */
            b[i] = ((i + 1) % 3) - 1;
        }

        int result = ntg_matmul_ffi(
            a, size, size,
            b, size, size,
            out,
            &stats
        );

        if (result == 0) {
            printf("  Result[0]: %f\n", out[0]);
            printf("  Latency: %lu us\n", stats.latency_us);
            printf("  Memory: %lu bytes\n", stats.memory_bytes);
            printf("  SIMD path: %s\n", get_simd_path_name(stats.simd_path));
        } else {
            printf("  ERROR: matmul failed\n");
        }

        free(a);
        free(b);
        free(out);
        printf("  Total ops: %lu\n\n", ntg_get_op_count());
    }

    /* Example 3: Error handling (null pointer) */
    printf("[Example 3] Error handling - null pointer\n");
    {
        int result = ntg_matmul_ffi(
            NULL,  /* null pointer should be rejected */
            2, 2,
            NULL,
            2, 2,
            NULL,
            NULL
        );

        if (result != 0) {
            printf("  ✓ Correctly rejected null pointer (error code: %d)\n", result);
        } else {
            printf("  ✗ Should have rejected null pointer\n");
        }
        printf("\n");
    }

    /* Example 4: Performance comparison */
    printf("[Example 4] Performance characteristics\n");
    {
        int sizes[] = {10, 50, 100};
        printf("  Size   | Latency (us) | Memory (bytes) | SIMD Path\n");
        printf("  -------|--------------|----------------|----------\n");

        for (int i = 0; i < 3; i++) {
            int size = sizes[i];
            int8_t *a = (int8_t*)malloc(size * size * sizeof(int8_t));
            int8_t *b = (int8_t*)malloc(size * size * sizeof(int8_t));
            float *out = (float*)malloc(size * size * sizeof(float));
            OpStats stats = {0};

            memset(a, 1, size * size * sizeof(int8_t));
            memset(b, 1, size * size * sizeof(int8_t));

            ntg_matmul_ffi(a, size, size, b, size, size, out, &stats);

            printf("  %2d    | %12lu | %14lu | %s\n",
                   size, stats.latency_us, stats.memory_bytes,
                   get_simd_path_name(stats.simd_path));

            free(a);
            free(b);
            free(out);
        }
    }

    printf("\n=== Summary ===\n");
    printf("Total operations executed: %lu\n", ntg_get_op_count());
    printf("FFI interface working correctly!\n");

    return 0;
}
