using System.Text.Json;

namespace FsuipcTestClient;

public static class BatchMode
{
    public static async Task<int> Run(string inputPath)
    {
        var (defs, errors) = OffsetParser.Parse(inputPath);

        if (errors.Count > 0)
        {
            var errResponse = new { ok = false, errors };
            Console.Error.WriteLine(JsonSerializer.Serialize(errResponse, JsonContext.Options));
            return 1;
        }

        if (defs.Count == 0)
        {
            var errResponse = new { ok = false, errors = new[] { "No offsets defined in input file" } };
            Console.Error.WriteLine(JsonSerializer.Serialize(errResponse, JsonContext.Options));
            return 1;
        }

        using var client = new FsuipcClient();

        try
        {
            client.Connect();
        }
        catch (Exception ex)
        {
            var errResponse = new { ok = false, errors = new[] { $"FSUIPC connection failed: {ex.Message}" } };
            Console.Error.WriteLine(JsonSerializer.Serialize(errResponse, JsonContext.Options));
            return 1;
        }

        client.RegisterOffsets(defs);

        try
        {
            client.Process();
        }
        catch (Exception ex)
        {
            var errResponse = new { ok = false, errors = new[] { $"FSUIPC Process failed: {ex.Message}" } };
            Console.Error.WriteLine(JsonSerializer.Serialize(errResponse, JsonContext.Options));
            return 1;
        }

        var snapshot = FormatSnapshot(client.Handles);
        Console.WriteLine(JsonSerializer.Serialize(snapshot, JsonContext.Options));
        return 0;
    }

    public static Snapshot FormatSnapshot(IReadOnlyList<RegisteredOffset> handles)
    {
        var entries = new List<OffsetSnapshotEntry>();
        foreach (var h in handles)
        {
            var val = h.Value;
            entries.Add(new OffsetSnapshotEntry(
                h.Def.Address > 0xFFFF ? $"0x{h.Def.Address:X5}" : $"0x{h.Def.Address:X4}",
                TypeString(h.Def.Type),
                TypeInfo.IsFixedSize(h.Def.Type) ? null : h.Def.Size,
                val is byte[] buf ? Convert.ToHexString(buf) : val
            ));
        }

        return new Snapshot(
            DateTime.UtcNow.ToString("o"),
            "FSUIPC",
            entries
        );
    }

    static string TypeString(OffsetType t) => t switch
    {
        OffsetType.U8 => "u8",
        OffsetType.I8 => "i8",
        OffsetType.U16 => "u16",
        OffsetType.I16 => "i16",
        OffsetType.U32 => "u32",
        OffsetType.I32 => "i32",
        OffsetType.F32 => "f32",
        OffsetType.U64 => "u64",
        OffsetType.I64 => "i64",
        OffsetType.F64 => "f64",
        OffsetType.String => "string",
        OffsetType.Bytes => "bytes",
        _ => "?"
    };
}

file static class JsonContext
{
    public static readonly JsonSerializerOptions Options = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };
}
