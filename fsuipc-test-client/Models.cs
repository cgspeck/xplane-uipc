namespace FsuipcTestClient;

public enum OffsetType
{
    U8, I8, U16, I16, U32, I32, F32, U64, I64, F64, String, Bytes
}

public record OffsetDefinition(int Address, OffsetType Type, int Size);

public record OffsetValue(OffsetDefinition Def, string? DisplayValue, bool HasError, string? Error);

public record Snapshot(string Timestamp, string Source, List<OffsetSnapshotEntry> Offsets);

public record OffsetSnapshotEntry(string Address, string Type, int? Size, object? Value);

public static class TypeInfo
{
    public static int FixedSize(OffsetType t) => t switch
    {
        OffsetType.U8 or OffsetType.I8 => 1,
        OffsetType.U16 or OffsetType.I16 => 2,
        OffsetType.U32 or OffsetType.I32 or OffsetType.F32 => 4,
        OffsetType.U64 or OffsetType.I64 or OffsetType.F64 => 8,
        _ => throw new ArgumentException($"Variable-size type {t} has no fixed size")
    };

    public static bool IsFixedSize(OffsetType t) => t is not OffsetType.String and not OffsetType.Bytes;
}
