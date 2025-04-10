#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stdbool.h>
#include <unistd.h>

#include <sys/types.h>

// Function pointer to the original readline
char *(*og_readline)(const char *prompt) = NULL;
// Are we being executed by a shell
bool in_shell = false;

char *rewrite(char *input) {
    // rewrite the command string to take the last "mp" pipe and
    // wrap the whole expression in it, eg:
    //   cat /etc/passwd | grep root | mp -t5
    //   mp -t5 'cat /etc/passwd | grep root'

    // find the location of the last pipe
    char *pipe = strrchr(input, '|');
    if (!pipe)
        return input;

    // check that it is followed by the "mp" binary
    char *curr = pipe+1;
    while (*curr == ' ' || *curr == '\t')
        curr++;

    if (!strstr(curr, "mp"))
        return input;

    if (curr[2] != 0 && curr[2] != ' ' && curr[2] != '\n')
        return input;

    // we are now guaranteed that pipe points to the string:
    //    | mp ...
    // now rewrite the string without the pipe...
    char *new = malloc(strlen(input) + 4);
    curr = new;

    // mp ...
    curr = stpcpy(curr, pipe+1);

    // mp ... "
    curr[0] = ' ';
    curr[1] = '"';
    curr+=2;

    // mp ... "cat /etc/passwd | grep root | mp -t5
    pipe[0] = 0;
    curr = stpcpy(curr, input);

    // mp ... "cat /etc/passwd | grep root | mp -t5"
    curr[0] = '"';
    curr[1] = 0;

    free(input);
    return new;
}

// check if a path exists in the /etc/shells file
bool check_shell(char *path) {
    FILE *f = fopen("/etc/shells", "r");
    bool skip_nl = false;
    char c;
    char *curr = path;

    while ((c = fgetc(f)) != EOF) {
        // reset on newlines
        if (c == '\n') {
            // if at end of path, we passed the cmp
            if (*curr == 0)
                return true;

            skip_nl = false;
            curr = path;
            continue;
        }

        // skip until next newline
        if (skip_nl)
            continue;

        // skip comments
        if (c == '#' && curr == path) {
            skip_nl = true;
            continue;
        }
        
        // compare the two strings character by character
        if (c == *curr) {
            curr++;
        }
        else
            skip_nl = true;
    }

    return false;
}

char *readline(const char *prompt) {
    if (!og_readline) {
        og_readline = dlsym(RTLD_NEXT, "readline");
        if (!og_readline) {
            fprintf(stderr, "Failed to find original readline\n");
            exit(1);
        }

        // check if we are being executed by a shell
        char exe_path[1024];
        char path[1024];
        pid_t pid = getpid();
        sprintf(exe_path, "/proc/%d/exe", pid);

        size_t n = readlink(exe_path, path, 1023);
        // ignore error case, shell is false
        if (n > 0) {
            path[n] = 0;
            in_shell = check_shell(path);
        }
    }
   
    char *input = og_readline(prompt);
    if (!in_shell)
        return input;

    if (!input)
        return NULL;

    return rewrite(input);
}

int main() {
    char *s = readline("bash $");
    printf("%s\n", s);
}
