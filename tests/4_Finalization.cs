using System.Diagnostics;

M();
GC.Collect();
GC.WaitForPendingFinalizers();
Console.WriteLine($"Finalized: {Finalizable.Finalized}");
Debug.Assert(Finalizable.Finalized == 10);
Console.WriteLine("Hello, world!");

static void M()
{
    int seed = Random.Shared.Next();
    Console.WriteLine($"Seed: {seed}");
    var random = new Random(seed);
    for (int i = 0; i < 1000; i++)
    {
        _ = new object[random.Next(1000)];
        if (i % 100 == 0)
        {
            GC.Collect();
            _ = new Finalizable(i / 100);
        }
    }
}

class Finalizable(int index)
{
    public static int Finalized = 0;
    ~Finalizable()
    {
        Interlocked.Increment(ref Finalized);
        Console.WriteLine($"Finalizer {index} called");
    }
}
