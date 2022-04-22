#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

bool initialize_agent(const char *application_name,
                      const char *server_address,
                      uint32_t sample_rate,
                      bool detect_subprocesses,
                      const char *tags);

bool drop_agent(void);

bool add_thread_tag(uint64_t thread_id, const char *key, const char *value);

bool remove_thread_tag(uint64_t thread_id, const char *key, const char *value);

bool add_global_tag(const char *key, const char *value);

bool remove_global_tag(const char *key, const char *value);
