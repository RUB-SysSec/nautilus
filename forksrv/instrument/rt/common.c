#define _GNU_SOURCE
#include <signal.h>
#include <unistd.h>
#include <stdio.h>
#include <fcntl.h> //Specified in man 2 open
#include <errno.h>
#include <sys/mman.h>
#include <stdlib.h> 
#include <string.h>
#include <sys/wait.h>
#include <sys/time.h>
#include <stdio.h>

#include "runtime_types.h"

feedback_data_t* rofl_feedback_data;

//so other constructors don't fail before we run our setup
uint8_t __rofl_pre_init_bitmap[ROFL_MAP_SIZE]; 
uint8_t* __rofl_area_ptr = __rofl_pre_init_bitmap;

__thread uint32_t __rofl_prev_loc;

void __rofl_init(){
  memset(__rofl_area_ptr, 0, ROFL_MAP_SIZE);
  __rofl_prev_loc = 0;
}

void __rofl_reset(){
    memset(rofl_feedback_data, 0x0, sizeof(feedback_data_t));
    if(getenv("ROFL_OUT_PATH")){
      int fd = fileno(fopen(getenv("ROFL_OUT_PATH"),"w+"));
      dup2(fd, 1);
    }

    if(getenv("ROFL_ERR_PATH")){
      int fd = fileno(fopen(getenv("ROFL_ERR_PATH"),"w+"));
      dup2(fd, 2);
    }
}

uint8_t* get_shm(size_t size){
    if(getenv("ROFL_SHM_FD") != NULL){
      int shm_fd = atoi(getenv("ROFL_SHM_FD"));
      ftruncate(shm_fd, size);
      void* addr = NULL;
      void* shm = mmap(addr, size, PROT_READ | PROT_WRITE, MAP_SHARED, shm_fd, 0);
      if(shm == (void*)-1){
        fprintf(stderr, "Could not mmap... %s\n", strerror(errno));
        return 0;
      }
      return (uint8_t*)shm;
    } else {
      return 0;
    }
}

void __rofl_forkserver(){
    fsync(0);
    if(getenv("ROFL_SHM_FD")){
      printf("running forkserver\n");
      while(1){
        uint8_t buffer;
        int pid;
        //stop ourself, so that the forkserver can continue us when needed
        kill(getpid(),  SIGSTOP);

        //fork the next running instance
        if((pid = fork()) < 0) {
            fprintf(stderr, "Could not fork... %s\n", strerror(errno));
        } else if(pid == 0) {
             struct itimerval timer;
             //trigger timeout signal after timer expires (20 ms)
             timer.it_value.tv_sec     = 0;
             timer.it_value.tv_usec    = 70000;
             timer.it_interval.tv_sec  = 0;
             timer.it_interval.tv_usec = 0;
             setitimer (ITIMER_VIRTUAL, &timer, NULL);
            __rofl_reset();
            return;
        } else {
          /* Elternprozess */
          int status;
          waitpid(pid, &status, 0);
          rofl_feedback_data->magic = 0x5a5a55464c464f52; //"ROFLFUZZ"
          rofl_feedback_data->status = status;
        }
      }
    }
}

void __attribute__ ((constructor)) get_shm_autorun(){
  size_t size = sizeof(feedback_data_t);
  uint8_t* shm = get_shm(size);
  if(!shm){ 
    shm = malloc(size);
  }

  rofl_feedback_data = (feedback_data_t*)shm;
  __rofl_area_ptr = &(rofl_feedback_data->run_bitmap[0]);

  __rofl_init();
  __rofl_forkserver();
}

