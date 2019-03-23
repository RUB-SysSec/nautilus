#ifndef __GUARD_REDQUEEN_RUNTIME__
#define __GUARD_REDQUEEN_RUNTIME__

#include "./runtime_types.h"

extern feedback_data_t* redqueen_feedback_data;

extern uint8_t* __rofl_area_ptr;
extern __thread uint32_t __rofl_prev_loc;

#endif
