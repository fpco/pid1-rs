# Testing

This needs to be automated in future. But as of now they have to be
mostly tested manually.

There are some programs under the directory `examples`, that will be
used for testing this library.

- simple.rs: Program which demonstrate the usage of pid1-rs library
- zombie.rs: Creates zombie process
- sigterm_handler.rs: Program which has SIGTERM handler and exits on receiving it.
- sigterm_loop.rs: A buggy program which doesn't exit on SIGTERM


## Environment setup

- Run the test program via the just recipe: `build-image`
- You can get shell access to it via the recipe: `exec-shell`

``` shellsession
❯ just exec-shell
docker exec -it pid1rs sh
/ # ps aux
PID   USER     TIME  COMMAND
    1 root      0:05 /simple
    7 root      0:00 /simple
    8 root      0:00 sh
   14 root      0:00 [date]
   15 root      0:00 ps aux
```

# Tests

## Basic functionality

This is also tested in CI, so you can likely skip this. But here are the steps:

``` shellsession
❯ just test
...
docker run --name pid1rs -t pid1rstest
pid1-rs: Process running as PID 1
pid1-rs: Process not running as Pid 1: PID 7
In the simple process, going to sleep. Process ID is 7
Args: ["/simple"]
Wed Sep 27 08:29:35 UTC 2023
Wed Sep 27 08:29:37 UTC 2023
Wed Sep 27 08:29:39 UTC 2023
```

You have to ensure that it exits with status code 0. The above test
confirms the following things:

- The `simple` program was executed as pid 1.
- It relaunched the process as pid 7 which executed various processes.

## Zombie process

``` shellsession
❯ just run-image
...
docker rm pid1rs || exit 0
pid1rs
docker run --name pid1rs -t pid1rstest /simple --sleep
pid1-rs: Process running as PID 1
pid1-rs: Process not running as Pid 1: PID 7
In the simple process, going to sleep. Process ID is 7
Args: ["/simple", "--sleep"]
Wed Sep 27 08:34:57 UTC 2023
Wed Sep 27 08:34:59 UTC 2023
Wed Sep 27 08:35:01 UTC 2023
Going to sleep 500 seconds
Wed Sep 27 08:35:03 UTC 2023
```

And while it's executing create a new shell run the recipe which will
create zombie process:

``` shellsession
❯ just run-zombie
docker exec -t pid1rs zombie
Process ID is 9
Parent process: going to sleep and exit
```

You could see the following logs on the `simple` process about a
process being reaped

``` shellsession
...
Wed Sep 27 09:40:56 UTC 2023
Reaped pid: 15
```

## Child Exit code status propagation

- Run recipe `run-image`:

``` shellsession
❯ just run-image
...
docker rm pid1rs || exit 0
pid1rs
docker run --name pid1rs -t pid1rstest /simple --sleep
pid1-rs: Process running as PID 1
pid1-rs: Process not running as Pid 1: PID 7
In the simple process, going to sleep. Process ID is 7
Args: ["/simple", "--sleep"]
Wed Sep 27 08:34:57 UTC 2023
Wed Sep 27 08:34:59 UTC 2023
Wed Sep 27 08:35:01 UTC 2023
Going to sleep 500 seconds
Wed Sep 27 08:35:03 UTC 2023
```

- Run recipe `exec-shell` and do the testing:

``` shellsession
❯ just exec-shell
docker exec -it pid1rs sh
/ # ps -aux
USER         PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND
root           1  0.0  0.0    996     4 pts/0    Ss+  12:34   0:00 /simple --sleep
root           7  0.0  0.0    976     4 pts/0    S+   12:34   0:00 /simple --sleep
root           8  0.0  0.0      0     0 pts/0    Z+   12:34   0:00 [date] <defunct>
root           9  0.0  0.0   1672  1048 pts/1    Ss   12:34   0:00 sh
root          14  0.0  0.0      0     0 pts/0    Z+   12:34   0:00 [date] <defunct>
root          15  0.0  0.0      0     0 pts/0    Z+   12:34   0:00 [date] <defunct>
root          16  0.0  0.0      0     0 pts/0    Z+   12:34   0:00 [date] <defunct>
root          17  0.0  0.0   2460  1608 pts/1    R+   12:34   0:00 ps -aux
/ # kill -12 7
```

- See the logs of the `run-image` recipe and find the exit status
  code:

``` shellsession
...
error: Recipe `run-image` failed on line 21 with exit code 140
```

And the exit code status 140 is indeed correct (128 + 12).

## SIGINT/SIGTERM handling

The aim is to check that the child process's SIGTERM handler is called
in case it's defined.

These are the signal codes:

- SIGINT: 2
- SIGTERM: 15

Execute the recipe `sigterm-test`:

``` shellsession
❯ just sigterm-test
docker rm pid1rs || exit 0
pid1rs
docker run --name pid1rs -t pid1rstest sigterm_handler
pid1-rs: Process running as PID 1
pid1-rs: Process not running as Pid 1: PID 7
This APP can be killed by SIGTERM (15)
```

And now send SIGTERM to the pid1:

``` shellsession
❯ just send-sigterm
```

Confirm from the logs that the SIGTERM handler is called and the exit
status is 0:

``` shellsession
App got SIGTERM 15, going to exit
```

Now do the same test with `SIGINT` and you can confirm that it won't
print anything since it is not handled.

## SIGTERM ignore

This is for testing an application where it ignore SIGTERM and
continus on doing some work. This library should be able to force kill
such processes.

Execute the recipe `sigloop-test`:

``` shellsession
❯ just sigloop-test
docker rm pid1rs || exit 0
pid1rs
docker run --name pid1rs -t pid1rstest sigterm_loop
pid1-rs: Process running as PID 1
pid1-rs: Process not running as Pid 1: PID 7
This APP ignores SIGTERM (15)
```

And now send SIGTERM to the pid1:

``` shellsession
❯ just send-sigterm
```

You will see that it would have exited:

``` shellsession
App got SIGTERM 15, but will not exit
App got SIGTERM 15, but will not exit
error: Recipe `sigloop-test` failed on line 44 with exit code 137
```

You can confirm from the status code that the child process got killed
by (137 - 128 = 9) SIGKIL as the application ignores SIGTERM.
