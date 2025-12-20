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

/*============================================================================
 * Math Operations
 *============================================================================*/

int64_t trq_pow_int(int64_t base, int64_t exp) {
    if (exp < 0) {
        // Integer power with negative exponent returns 0
        // (would be fraction in float, but truncates to 0 in int)
        return 0;
    }

    if (exp == 0) {
        return 1;
    }

    int64_t result = 1;
    int64_t b = base;
    int64_t e = exp;

    // Fast exponentiation by squaring
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

/*============================================================================
 * Exception Handling
 *============================================================================*/

// Thread-local current exception
static __thread TrqException* current_exception = NULL;

void trq_throw(TrqException* exception) {
    if (current_exception) {
        // Release previous exception
        if (current_exception->message) {
            trq_release(current_exception->message);
        }
        if (current_exception->type) {
            trq_release(current_exception->type);
        }
        trq_free(current_exception);
    }

    current_exception = exception;

    // In a full implementation, this would unwind the stack
    // For now, print error and abort
    if (exception && exception->message && exception->message->data) {
        fprintf(stderr, "Exception / استثناء: %s\n", exception->message->data);
    } else {
        fprintf(stderr, "Exception / استثناء: (unknown)\n");
    }

    // TODO: Implement proper stack unwinding
    // For now, this is a fatal error
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

// Type tags for runtime type info
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
    // This is a simplified implementation
    // A full implementation would use tagged pointers or type metadata
    if (!value) {
        return trq_string_from_cstr("فارغ"); // null
    }

    // Default to object type
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

    // Initialize any global state
    current_exception = NULL;

    // Set up locale for proper UTF-8 output
    // Note: This may need to be configured based on platform
    #ifdef _WIN32
    // Windows-specific UTF-8 setup would go here
    #else
    // Unix-like systems typically handle UTF-8 by default
    #endif

    runtime_initialized = true;
}

void trq_runtime_cleanup(void) {
    if (!runtime_initialized) {
        return;
    }

    // Clean up any remaining exceptions
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

    // Call the user's main function
    __main__();

    trq_runtime_cleanup();

    return 0;
}
