using System.Diagnostics;

var alive = new List<S>();
int seed = Random.Shared.Next();
Console.WriteLine($"Seed: {seed}");
var random = new Random(seed);
for (int i = 0; i < 10000; i++)
{
    string s = i.ToString();
    _ = new object[random.Next(1000)];
    object o = i;
    if (i % 10 == 0)
    {
        alive.Add(new S { o = o, i = i, n = new Nested { s = s } });
    }
}

foreach (var s in alive)
{
    Debug.Assert(s.i == (int)s.o);
    Debug.Assert(s.n.s == s.i.ToString());
}

Console.WriteLine("Completed!");

struct S
{
    public object o;
    public int i;
    public Nested n;
}

struct Nested
{
    public string s;
}
