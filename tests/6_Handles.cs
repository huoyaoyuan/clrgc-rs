using System.Diagnostics;
using System.Runtime;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

var M = () =>
{
    var obj = new object();
    var r = new Recurse(obj);
    var recurse = new WeakGCHandle<object>(r, trackResurrection: true);
    var weak = new WeakGCHandle<object>(r, trackResurrection: false);
    var recursedep = new WeakGCHandle<object>(obj, trackResurrection: true);
    var weakdep = new WeakGCHandle<object>(obj, trackResurrection: false);
    var dep = new DependentHandle(r, obj);
    var dep2 = new DependentHandle(new object(), new Recurse(null));
    return (recurse, weak, recursedep, weakdep, dep, dep2);
};

var (recurse, weak, recursedep, weakdep, dep, dep2) = M();
for (int i = 0; i < 10; i++)
{
    GC.Collect();
    GC.WaitForPendingFinalizers();
    Console.WriteLine($"Recurse.IsAlive: {recurse.TryGetTarget(out _)}, Weak.IsAlive: {weak.TryGetTarget(out _)}");
    Console.WriteLine($"DependentHandle.TargetAlive: {dep.Target is not null}, DependentAlive: {dep.Dependent is not null}");
    Console.WriteLine($"RecurseDep.IsAlive: {recursedep.TryGetTarget(out _)}, WeakDep.IsAlive: {weakdep.TryGetTarget(out _)}");
    Console.WriteLine($"DependentHandle2.TargetAlive: {dep2.Target is not null}, DependentAlive: {dep2.Dependent is not null}");
}

Debug.Assert(recurse.TryGetTarget(out _));
Debug.Assert(!weak.TryGetTarget(out _));
Debug.Assert(dep.Target is not null);
Debug.Assert(dep.Dependent is not null);
Debug.Assert(recursedep.TryGetTarget(out _));
Debug.Assert(!weakdep.TryGetTarget(out _));
Debug.Assert(dep2.Target is null);
Debug.Assert(dep2.Dependent is null);

var CWT = () =>
{
    int seed = Random.Shared.Next();
    Console.WriteLine($"Seed: {seed}");
    var random = new Random(seed);
    var alive = new List<object>();
    var cwt = new ConditionalWeakTable<object, object>();
    for (int i = 0; i < 1000; i++)
    {
        var array = new byte[random.Next(1000)];
        cwt.Add(array, new { Transient = new Finalizable() });
        if (i % 10 == 0)
        {
            alive.Add(array);
        }
    }
    return (alive, cwt);
};

var (alive, cwt) = CWT();
GC.Collect();
GC.WaitForPendingFinalizers();
Console.WriteLine($"Finalized: {Finalizable.FinalizedCount}");
Debug.Assert(Finalizable.FinalizedCount == 900);
GC.KeepAlive(alive);
GC.KeepAlive(cwt);

class Recurse(object? field)
{
    private object? field = field;

    ~Recurse()
    {
        Console.WriteLine("Recurse!");
        GC.ReRegisterForFinalize(this);
    }
}

class Finalizable()
{
    public static int FinalizedCount = 0;
    ~Finalizable()
    {
        Interlocked.Increment(ref FinalizedCount);
    }
}
