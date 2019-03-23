#ifndef __GUARD_ROFL_RUNTIME_TYPES__
#define __GUARD_ROFL_RUNTIME_TYPES__

#include <stdint.h>
#include "./runtime_config.h"

//one instance of this struct is mapped to shared memory for communication
//content of this struct will be memset to zero before each run

typedef struct feedback_data_s{
  uint8_t run_bitmap[ROFL_MAP_SIZE];
  uint64_t magic;
  int status;
} feedback_data_t;

void __afl_init();
void __afl_forkserver();
void __afl_reset();

#endif
