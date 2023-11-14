import math

arr = list(range(512))
length = len(arr)
threads = (length+1) >> 1
i = 0
while threads > 0:
    len_per_thread = int(math.ceil(length / threads))
    print(f"threads are {threads}")
    for thread in range(threads):
        thread = thread * len_per_thread
        arr[thread] = arr[thread] + arr[thread+2**i]
    threads >>= 1
    i += 1
print(arr[0])
