using System.Collections.Concurrent;
using System.Diagnostics;

var survive = new ConcurrentBag<byte[]>();
var threads = new Thread[10];
for (int t = 0; t < threads.Length; t++)
{
    threads[t] = new Thread(() =>
    {
        int seed = Random.Shared.Next();
        Console.WriteLine($"Seed: {seed}");
        var random = new Random(seed);
        for (int i = 0; i < 10000; i++)
        {
            var array = new byte[random.Next(1000)];
            array.AsSpan().Fill(0xCC);
            if (i % 10 == 0)
            {
                survive.Add(array);
            }
        }
    });
    threads[t].Start();
}
for (int t = 0; t < threads.Length; t++)
{
    threads[t].Join();
}
foreach (var item in survive)
{
    Debug.Assert(!item.AsSpan().ContainsAnyExcept((byte)0xCC));
}
Debug.Assert(survive.Count == 10000);
Console.WriteLine("Completed");
