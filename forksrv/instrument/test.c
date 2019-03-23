#include<stdlib.h>
#include<string.h>
#include<stdio.h>

char* read_stdin(size_t* size_out){
  char *input, *p;
  int len, remain, n, size;

  size = 512;
  input = malloc(size);
  len = 0;
  remain = size;
  while (!feof(stdin)) {
    if (remain <= 128) {
      remain += size;
      size *= 2;
      p = realloc(input, size);
      if (p == NULL) {
        free(input);
        return 0;
      }
      input = p;
    }

    fgets(input + len, remain, stdin);
    n = strlen(input + len);
    len += n;
    remain -= n;
  }
  *size_out = len;
  return input;
}

int main() {
	printf("Hello World\n");
  size_t size = 0;
  char* inp =  read_stdin(&size);
  if(size < 10){
    return 0;
  }
  if(inp[0]=='r'){
  if(inp[1]=='o'){
  if(inp[2]=='f'){
  if(inp[3]=='l'){
  if(inp[4]=='f'){
  if(inp[5]=='u'){
  if(inp[6]=='z'){
  if(inp[7]=='z'){
    ((char*)0)[13] = 13;
  }}}}}}}}
	return 0;
}
