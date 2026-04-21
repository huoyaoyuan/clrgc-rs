using System.Diagnostics;

var survive = new List<byte[]>();
int seed = Random.Shared.Next();
Console.WriteLine($"Seed: {seed}");
var random = new Random(seed);
int hash = 0;
for (int i = 0; i < 10000; i++)
{
    var array = new byte[random.Next(1000)];
    array.AsSpan().Fill(0xCC);
    if (i % 10 == 0)
    {
        survive.Add(array);
        hash ^= array.GetHashCode();
        foreach (var item in survive)
        {
            Debug.Assert(!item.AsSpan().ContainsAnyExcept((byte)0xCC));
        }
    }
}

int newHash = 0;
foreach (var item in survive)
{
    newHash ^= item.GetHashCode();
}
Debug.Assert(hash == newHash);

Console.WriteLine("Completed!");
