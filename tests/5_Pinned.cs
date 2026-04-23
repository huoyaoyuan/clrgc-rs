#:property AllowUnsafeBlocks=true

using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

byte[] a = new byte[1000];
int[] b = new int[1234];
string c = "Hello, World!";
unsafe
{
    fixed (byte* ptr = a)
    {
        var h1 = new PinnedGCHandle<int[]>(b);
        var h2 = new PinnedGCHandle<string>(c);
        var d1 = h1.GetAddressOfArrayData();
        var d2 = h2.GetAddressOfStringData();
        var alive = new List<byte[]>();
        var seed = Random.Shared.Next();
        Console.WriteLine($"Seed: {seed}");
        var random = new Random(seed);
        for (int i = 0; i < 1000; i++)
        {
            var array = new byte[random.Next(1000)];
            if (i % 10 == 0)
            {
                alive.Add(array);
                GC.Collect();
                Debug.Assert(ptr == Unsafe.AsPointer(ref a[0]));
                Debug.Assert(d1 == Unsafe.AsPointer(ref b[0]));
                Debug.Assert(d2 == Unsafe.AsPointer(in c.GetPinnableReference()));
            }
        }
    }
}
