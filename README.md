# clrgc-rs

A compatible [CLR GC](https://github.com/dotnet/runtime/tree/v10.0.0/src/coreclr/gc) implemented in Rust.

It's a naive proof-of-concept learning project that demonstrates the interface and contract of CLR GC. Check also the [excellent posts](https://minidump.net/2025-28-01-writing-a-net-gc-in-c-part-1/) for implementing GC in C# NativeAOT by Kevin Goose.

## Features
- Precise mark-compact-sweep GC is implemented. Object references on the managed stack and heap objects can be correctly updated.
- Pinning and GC handles.
- Generational collection is not implemented. Programs explicitly observing can see different behavior.
- Background (concurrent) GC is not implemented. Everything happens in a single STW collection. Running the managed application with multiple threads is supported, though only one thread performs the GC.
- Finalization is correctly supported, but the behavior details may differ from CLR GC, especially when relating to generations.
- Although the code is written in platform-neutral code, Windows x86 is known to be unsupported due to calling convention quirks.

## Testing

Just set the environment variable `DOTNET_GCPath` to the compiled library and run any .NET 10 application. The versions of CLR and GC are required to match closely, and this project is written with the interface of .NET 10.

It's also recommended to enable [Page Heap](https://learn.microsoft.com/windows-hardware/drivers/debugger/gflags-and-pageheap) to catch heap corruption early. It can also be enabled by setting the following registry values:

```reg
[HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\<executable.exe>]
"VerifierFlags"=dword:00000001
"PageHeapFlags"="0x3"
"GlobalFlag"="0x02200000"
```
