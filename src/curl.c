#include <curl/curl.h>
#include <limits.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef size_t (*lunacy_curl_write_cb)(void *userdata,
                                       const unsigned char *data, size_t len);
typedef int (*lunacy_curl_xferinfo_cb)(void *userdata, int64_t dltotal,
                                       int64_t dlnow, int64_t ultotal,
                                       int64_t ulnow);

struct lunacy_curl_easy {
    CURL *handle;
    char error[CURL_ERROR_SIZE];
    lunacy_curl_write_cb write_cb;
    void *write_userdata;
    lunacy_curl_xferinfo_cb xferinfo_cb;
    void *xferinfo_userdata;
};

static struct lunacy_curl_easy *lunacy_curl_easy(void *easy) {
    return (struct lunacy_curl_easy *)easy;
}

static size_t lunacy_curl_write_trampoline(char *ptr, size_t size,
                                           size_t nmemb, void *userdata) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(userdata);

    if (nmemb != 0 && size > SIZE_MAX / nmemb) {
        return 0;
    }

    size_t len = size * nmemb;
    if (easy == NULL || easy->write_cb == NULL) {
        return len;
    }

    return easy->write_cb(easy->write_userdata, (const unsigned char *)ptr,
                          len);
}

static int lunacy_curl_xferinfo_trampoline(void *userdata, curl_off_t dltotal,
                                           curl_off_t dlnow,
                                           curl_off_t ultotal,
                                           curl_off_t ulnow) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(userdata);
    if (easy == NULL || easy->xferinfo_cb == NULL) {
        return 0;
    }

    return easy->xferinfo_cb(easy->xferinfo_userdata, (int64_t)dltotal,
                             (int64_t)dlnow, (int64_t)ultotal,
                             (int64_t)ulnow);
}

int lunacy_curl_global_init(void) {
    return (int)curl_global_init(CURL_GLOBAL_DEFAULT);
}

void lunacy_curl_global_cleanup(void) { curl_global_cleanup(); }

const char *lunacy_curl_easy_strerror(int code) {
    return curl_easy_strerror((CURLcode)code);
}

int lunacy_curl_bad_function_argument(void) {
    return (int)CURLE_BAD_FUNCTION_ARGUMENT;
}

int lunacy_curl_easy_new(void **out) {
    if (out == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    struct lunacy_curl_easy *easy =
        (struct lunacy_curl_easy *)malloc(sizeof(*easy));
    if (easy == NULL) {
        return (int)CURLE_OUT_OF_MEMORY;
    }

    memset(easy, 0, sizeof(*easy));
    easy->handle = curl_easy_init();
    if (easy->handle == NULL) {
        free(easy);
        return (int)CURLE_OUT_OF_MEMORY;
    }

    easy->error[0] = '\0';
    CURLcode code = curl_easy_setopt(easy->handle, CURLOPT_ERRORBUFFER,
                                     easy->error);
    if (code != CURLE_OK) {
        curl_easy_cleanup(easy->handle);
        free(easy);
        return (int)code;
    }

    *out = easy;
    return (int)CURLE_OK;
}

void lunacy_curl_easy_cleanup(void *easy_ptr) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL) {
        return;
    }

    if (easy->handle != NULL) {
        curl_easy_cleanup(easy->handle);
    }
    free(easy);
}

const char *lunacy_curl_easy_error_buffer(void *easy_ptr) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL) {
        return "";
    }

    return easy->error;
}

int lunacy_curl_easy_perform(void *easy_ptr) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    easy->error[0] = '\0';
    return (int)curl_easy_perform(easy->handle);
}

int lunacy_curl_easy_get_response_code(void *easy_ptr, uint32_t *out) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL || out == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    long code = 0;
    CURLcode result =
        curl_easy_getinfo(easy->handle, CURLINFO_RESPONSE_CODE, &code);
    if (result != CURLE_OK) {
        return (int)result;
    }
    if (code < 0 || code > UINT32_MAX) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    *out = (uint32_t)code;
    return (int)CURLE_OK;
}

int lunacy_curl_easy_set_url(void *easy_ptr, const char *url) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL || url == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_URL, url);
}

int lunacy_curl_easy_set_follow_location(void *easy_ptr, int enabled) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_FOLLOWLOCATION,
                                 enabled ? 1L : 0L);
}

int lunacy_curl_easy_set_verbose(void *easy_ptr, int enabled) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_VERBOSE,
                                 enabled ? 1L : 0L);
}

int lunacy_curl_easy_set_ssl_verifypeer(void *easy_ptr, int enabled) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_SSL_VERIFYPEER,
                                 enabled ? 1L : 0L);
}

int lunacy_curl_easy_set_ssl_verifyhost(void *easy_ptr, int enabled) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_SSL_VERIFYHOST,
                                 enabled ? 2L : 0L);
}

int lunacy_curl_easy_set_ca_info(void *easy_ptr, const char *path) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_CAINFO, path);
}

int lunacy_curl_easy_set_ca_path(void *easy_ptr, const char *path) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_CAPATH, path);
}

static int lunacy_curl_easy_set_milliseconds(void *easy_ptr, long option,
                                             int64_t millis) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL || millis < 0 ||
        millis > LONG_MAX) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, (CURLoption)option,
                                 (long)millis);
}

int lunacy_curl_easy_set_timeout_ms(void *easy_ptr, int64_t millis) {
    return lunacy_curl_easy_set_milliseconds(easy_ptr, CURLOPT_TIMEOUT_MS,
                                             millis);
}

int lunacy_curl_easy_set_connect_timeout_ms(void *easy_ptr, int64_t millis) {
    return lunacy_curl_easy_set_milliseconds(easy_ptr,
                                             CURLOPT_CONNECTTIMEOUT_MS,
                                             millis);
}

int lunacy_curl_easy_set_post(void *easy_ptr, int enabled) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_POST,
                                 enabled ? 1L : 0L);
}

int lunacy_curl_easy_set_copy_post_fields(void *easy_ptr, const void *data,
                                          uint64_t len) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL || (data == NULL && len != 0)) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    curl_off_t curl_len = (curl_off_t)len;
    if ((uint64_t)curl_len != len) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    CURLcode code =
        curl_easy_setopt(easy->handle, CURLOPT_POSTFIELDSIZE_LARGE, curl_len);
    if (code != CURLE_OK) {
        return (int)code;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_COPYPOSTFIELDS, data);
}

int lunacy_curl_easy_set_http_headers(void *easy_ptr, void *list) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_HTTPHEADER,
                                 (struct curl_slist *)list);
}

int lunacy_curl_easy_set_write_function(void *easy_ptr,
                                        lunacy_curl_write_cb callback,
                                        void *userdata) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL || callback == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    easy->write_cb = callback;
    easy->write_userdata = userdata;
    CURLcode code = curl_easy_setopt(easy->handle, CURLOPT_WRITEFUNCTION,
                                     lunacy_curl_write_trampoline);
    if (code != CURLE_OK) {
        return (int)code;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_WRITEDATA, easy);
}

int lunacy_curl_easy_set_xferinfo_function(void *easy_ptr,
                                           lunacy_curl_xferinfo_cb callback,
                                           void *userdata) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL || callback == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    easy->xferinfo_cb = callback;
    easy->xferinfo_userdata = userdata;
    CURLcode code = curl_easy_setopt(easy->handle, CURLOPT_XFERINFOFUNCTION,
                                     lunacy_curl_xferinfo_trampoline);
    if (code != CURLE_OK) {
        return (int)code;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_XFERINFODATA, easy);
}

int lunacy_curl_easy_set_noprogress(void *easy_ptr, int enabled) {
    struct lunacy_curl_easy *easy = lunacy_curl_easy(easy_ptr);
    if (easy == NULL || easy->handle == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    return (int)curl_easy_setopt(easy->handle, CURLOPT_NOPROGRESS,
                                 enabled ? 1L : 0L);
}

int lunacy_curl_slist_append(void **list, const char *value) {
    if (list == NULL || value == NULL) {
        return (int)CURLE_BAD_FUNCTION_ARGUMENT;
    }

    struct curl_slist *next =
        curl_slist_append((struct curl_slist *)*list, value);
    if (next == NULL) {
        return (int)CURLE_OUT_OF_MEMORY;
    }

    *list = next;
    return (int)CURLE_OK;
}

void lunacy_curl_slist_free_all(void *list) {
    curl_slist_free_all((struct curl_slist *)list);
}
