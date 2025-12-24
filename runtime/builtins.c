/**
 * Tarqeem Runtime - Built-in Functions
 *
 * Implements math operations, exception handling, type info, and runtime initialization.
 */

#include "tarqeem_rt.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <math.h>
#include <time.h>

/*============================================================================
 * Math Operations
 *============================================================================*/

int64_t trq_pow_int(int64_t base, int64_t exp) {
    if (exp < 0) {
        return 0;
    }

    if (exp == 0) {
        return 1;
    }

    int64_t result = 1;
    int64_t b = base;
    int64_t e = exp;

    while (e > 0) {
        if (e & 1) {
            result *= b;
        }
        b *= b;
        e >>= 1;
    }

    return result;
}

int64_t trq_abs_int(int64_t value) {
    return value < 0 ? -value : value;
}

double trq_abs_float(double value) {
    return fabs(value);
}

/*============================================================================
 * Additional Math Functions
 *============================================================================*/

double trq_sqrt(double value) {
    return sqrt(value);
}

double trq_sin(double value) {
    return sin(value);
}

double trq_cos(double value) {
    return cos(value);
}

double trq_tan(double value) {
    return tan(value);
}

double trq_log(double value) {
    return log(value);
}

double trq_log10(double value) {
    return log10(value);
}

double trq_exp(double value) {
    return exp(value);
}

double trq_floor(double value) {
    return floor(value);
}

double trq_ceil(double value) {
    return ceil(value);
}

double trq_round(double value) {
    return round(value);
}

int64_t trq_min_int(int64_t a, int64_t b) {
    return a < b ? a : b;
}

int64_t trq_max_int(int64_t a, int64_t b) {
    return a > b ? a : b;
}

double trq_min_float(double a, double b) {
    return a < b ? a : b;
}

double trq_max_float(double a, double b) {
    return a > b ? a : b;
}

double trq_pow_float(double base, double exp) {
    return pow(base, exp);
}

double trq_cbrt(double value) {
    return cbrt(value);
}

double trq_nroot(double value, int64_t n) {
    if (n == 0) return 0.0;
    return pow(value, 1.0 / (double)n);
}

double trq_log2(double value) {
    return log2(value);
}

double trq_trunc(double value) {
    return trunc(value);
}

int64_t trq_clamp_int(int64_t value, int64_t min, int64_t max) {
    if (value < min) return min;
    if (value > max) return max;
    return value;
}

double trq_clamp_float(double value, double min, double max) {
    if (value < min) return min;
    if (value > max) return max;
    return value;
}

int64_t trq_sign(int64_t value) {
    if (value < 0) return -1;
    if (value > 0) return 1;
    return 0;
}

int64_t trq_mod(int64_t a, int64_t b) {
    if (b == 0) return 0;
    int64_t result = a % b;
    if (result < 0) result += (b < 0 ? -b : b);
    return result;
}

int64_t trq_gcd(int64_t a, int64_t b) {
    if (a < 0) a = -a;
    if (b < 0) b = -b;
    while (b != 0) {
        int64_t temp = b;
        b = a % b;
        a = temp;
    }
    return a;
}

int64_t trq_lcm(int64_t a, int64_t b) {
    if (a == 0 || b == 0) return 0;
    int64_t g = trq_gcd(a, b);
    return (a / g) * b;
}

int64_t trq_factorial(int64_t n) {
    if (n < 0) return 0;
    if (n <= 1) return 1;
    int64_t result = 1;
    for (int64_t i = 2; i <= n; i++) {
        result *= i;
    }
    return result;
}

/* Trigonometric functions */
double trq_cot(double value) {
    return 1.0 / tan(value);
}

double trq_sec(double value) {
    return 1.0 / cos(value);
}

double trq_csc(double value) {
    return 1.0 / sin(value);
}

/* Inverse trigonometric functions */
double trq_asin(double value) {
    return asin(value);
}

double trq_acos(double value) {
    return acos(value);
}

double trq_atan(double value) {
    return atan(value);
}

double trq_atan2(double y, double x) {
    return atan2(y, x);
}

/* Hyperbolic functions */
double trq_sinh(double value) {
    return sinh(value);
}

double trq_cosh(double value) {
    return cosh(value);
}

double trq_tanh(double value) {
    return tanh(value);
}

/* Conversion */
double trq_to_radians(double degrees) {
    return degrees * (3.14159265358979323846 / 180.0);
}

double trq_to_degrees(double radians) {
    return radians * (180.0 / 3.14159265358979323846);
}

/*============================================================================
 * Random Number Generation
 *============================================================================*/

static bool random_initialized = false;

void trq_random_seed(int64_t seed) {
    srand((unsigned int)seed);
    random_initialized = true;
}

static void ensure_random_init(void) {
    if (!random_initialized) {
        srand((unsigned int)time(NULL));
        random_initialized = true;
    }
}

int64_t trq_random_int(void) {
    ensure_random_init();
    return ((int64_t)rand() << 32) | (int64_t)rand();
}

int64_t trq_random_int_range(int64_t min, int64_t max) {
    if (min >= max) return min;
    ensure_random_init();
    int64_t range = max - min + 1;
    return min + (trq_random_int() % range);
}

double trq_random_float(void) {
    ensure_random_init();
    return (double)rand() / ((double)RAND_MAX + 1.0);
}

double trq_random_float_range(double min, double max) {
    if (min >= max) return min;
    return min + trq_random_float() * (max - min);
}

bool trq_random_bool(void) {
    ensure_random_init();
    return rand() % 2 == 0;
}

/*============================================================================
 * Exception Handling
 *============================================================================*/

static __thread TrqException* current_exception = NULL;

void trq_throw(TrqException* exception) {
    if (current_exception) {
        if (current_exception->message) {
            trq_release(current_exception->message);
        }
        if (current_exception->type) {
            trq_release(current_exception->type);
        }
        trq_free(current_exception);
    }

    current_exception = exception;

    if (exception && exception->message && exception->message->data) {
        fprintf(stderr, "Exception / استثناء: %s\n", exception->message->data);
    } else {
        fprintf(stderr, "Exception / استثناء: (unknown)\n");
    }

    abort();
}

TrqException* trq_get_exception(void) {
    return current_exception;
}

void trq_clear_exception(void) {
    if (current_exception) {
        if (current_exception->message) {
            trq_release(current_exception->message);
        }
        if (current_exception->type) {
            trq_release(current_exception->type);
        }
        trq_free(current_exception);
        current_exception = NULL;
    }
}

/**
 * Create a new exception.
 */
TrqException* trq_exception_new(TrqString* message, TrqString* type) {
    TrqException* exc = (TrqException*)trq_alloc(sizeof(TrqException));
    if (!exc) {
        return NULL;
    }

    exc->message = message;
    exc->type = type;
    exc->stack_trace = NULL;

    if (message) {
        trq_retain(message);
    }
    if (type) {
        trq_retain(type);
    }

    return exc;
}

/*============================================================================
 * Type Checking
 *============================================================================*/

typedef enum {
    TRQ_TYPE_NULL = 0,
    TRQ_TYPE_BOOL = 1,
    TRQ_TYPE_INT = 2,
    TRQ_TYPE_FLOAT = 3,
    TRQ_TYPE_STRING = 4,
    TRQ_TYPE_ARRAY = 5,
    TRQ_TYPE_OBJECT = 6,
    TRQ_TYPE_FUNCTION = 7,
} TrqTypeTag;

TrqString* trq_type_of(void* value) {
    if (!value) {
        return trq_string_from_cstr("فارغ"); // null
    }

    return trq_string_from_cstr("كائن"); // object
}

/**
 * Check if value is null.
 */
bool trq_is_null(void* value) {
    return value == NULL;
}

/*============================================================================
 * Utility Functions
 *============================================================================*/

/**
 * Assert a condition, abort if false.
 */
void trq_assert(bool condition, TrqString* message) {
    if (!condition) {
        fprintf(stderr, "Assertion failed / فشل التأكيد");
        if (message && message->data) {
            fprintf(stderr, ": %s", message->data);
        }
        fprintf(stderr, "\n");
        abort();
    }
}

/**
 * Panic with a message.
 */
void trq_panic(TrqString* message) {
    fprintf(stderr, "Panic / ذعر");
    if (message && message->data) {
        fprintf(stderr, ": %s", message->data);
    }
    fprintf(stderr, "\n");
    abort();
}

/*============================================================================
 * Runtime Initialization
 *============================================================================*/

static bool runtime_initialized = false;

void trq_runtime_init(void) {
    if (runtime_initialized) {
        return;
    }

    current_exception = NULL;

    #ifdef _WIN32
    #else
    #endif

    runtime_initialized = true;
}

void trq_runtime_cleanup(void) {
    if (!runtime_initialized) {
        return;
    }

    trq_clear_exception();

    runtime_initialized = false;
}

/*============================================================================
 * Program Entry Point Helper
 *============================================================================*/

/**
 * Main entry point wrapper.
 * This is called by the generated code's main function.
 */
extern void __main__(void);

int main(int argc, char** argv) {
    (void)argc;
    (void)argv;

    trq_runtime_init();

    __main__();

    trq_runtime_cleanup();

    return 0;
}
