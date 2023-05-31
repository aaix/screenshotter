import math

arr = list(range(16))
length = len(arr)
threads = (length+1) >> 1
print(arr)
i = 0
while threads > 0:
    len_per_thread = int(math.ceil(length / threads))
    print(f"threads are {threads}")
    for thread in range(threads):
        print(f"running thread {thread}")
        thread = thread * len_per_thread
        arr[thread] = arr[thread] + arr[thread+2**i]
        print(arr)
    threads >>= 1
    i += 1
    print(arr)
