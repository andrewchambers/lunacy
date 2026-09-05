#ifndef LUNACY_COMPAT_H
#define LUNACY_COMPAT_H

/* C99 has no explicit alignment operators. GCC and Clang provide these
   extensions in C99 mode, including when compiling with -pedantic-errors. */
#if defined(__GNUC__) || defined(__clang__)
#define LUNACY_ALIGNAS(n) __attribute__((__aligned__(n)))
#define LUNACY_ALIGNOF(type) __alignof__(type)
#define LUNACY_UNUSED __attribute__((__unused__))
#else
#error "lunacy requires GCC-compatible alignment attributes and __alignof__"
#endif

#define LUNACY_CONCAT_INNER(a, b) a##b
#define LUNACY_CONCAT(a, b) LUNACY_CONCAT_INNER(a, b)
/* A negative array bound rejects an incompatible ABI at compile time. The
   message remains at each call site; line numbers distinguish the typedefs. */
#define LUNACY_STATIC_ASSERT(condition, message) \
    typedef char LUNACY_CONCAT(lunacy_static_assert_, __LINE__)[(condition) ? 1 : -1] LUNACY_UNUSED

/* malloc can hold this ordinary C type. Its alignment is a conservative
   threshold, not a claim about the target's maximum supported alignment.
   Stronger Rust alignments go through posix_memalign. */
union lunacy_malloc_alignment {
    long double floating;
    long long integer;
    void *object;
    void (*function)(void);
};
#define LUNACY_MALLOC_ALIGNMENT LUNACY_ALIGNOF(union lunacy_malloc_alignment)

#endif
