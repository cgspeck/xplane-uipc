using System.Globalization;

namespace FsuipcTestClient;

public static class OffsetParser
{
    public static (List<OffsetDefinition> Offsets, List<string> Errors) Parse(string path)
    {
        var offsets = new List<OffsetDefinition>();
        var errors = new List<string>();
        var lines = File.ReadAllLines(path);

        for (int i = 0; i < lines.Length; i++)
        {
            var line = lines[i].Trim();
            var commentIdx = line.IndexOf('#');
            if (commentIdx >= 0)
                line = line[..commentIdx].TrimEnd();
            if (line.Length == 0)
                continue;

            var parts = line.Split(',', StringSplitOptions.TrimEntries);
            if (parts.Length < 2)
            {
                errors.Add($"Line {i + 1}: expected at least <offset>,<type>[,<size>], got: {line}");
                continue;
            }

            if (!TryParseAddress(parts[0], out var address))
            {
                errors.Add($"Line {i + 1}: invalid offset address '{parts[0]}' — expected hex 0x0000-0xFFFFF");
                continue;
            }

            if (!TryParseType(parts[1], out var type))
            {
                errors.Add($"Line {i + 1}: unknown type '{parts[1]}' — valid: u8,i8,u16,i16,u32,i32,f32,u64,i64,f64,string,bytes");
                continue;
            }

            int size;
            if (TypeInfo.IsFixedSize(type))
            {
                size = TypeInfo.FixedSize(type);
            }
            else
            {
                if (parts.Length < 3 || !int.TryParse(parts[2], out size) || size < 1)
                {
                    errors.Add($"Line {i + 1}: {parts[1]} requires a positive size as third column, got: {line}");
                    continue;
                }
            }

            offsets.Add(new OffsetDefinition(address, type, size));
        }

        return (offsets, errors);
    }

    static bool TryParseAddress(string s, out int address)
    {
        address = 0;
        if (s.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
            s = s[2..];
        return int.TryParse(s, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out address)
            && address >= 0 && address <= 0xFFFFF;
    }

    public static void WriteFile(string path, List<OffsetDefinition> defs)
    {
        using var writer = new StreamWriter(path);
        foreach (var def in defs)
        {
            var addr = def.Address > 0xFFFF ? $"0x{def.Address:X5}" : $"0x{def.Address:X4}";
            var typeStr = TypeLabel(def.Type);
            if (TypeInfo.IsFixedSize(def.Type))
                writer.WriteLine($"{addr},{typeStr}");
            else
                writer.WriteLine($"{addr},{typeStr},{def.Size}");
        }
    }

    static string TypeLabel(OffsetType t) => t switch
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

    static bool TryParseType(string s, out OffsetType type)
    {
        switch (s.ToLowerInvariant())
        {
            case "u8": type = OffsetType.U8; return true;
            case "i8": type = OffsetType.I8; return true;
            case "u16": type = OffsetType.U16; return true;
            case "i16": type = OffsetType.I16; return true;
            case "u32": type = OffsetType.U32; return true;
            case "i32": type = OffsetType.I32; return true;
            case "f32": type = OffsetType.F32; return true;
            case "u64": type = OffsetType.U64; return true;
            case "i64": type = OffsetType.I64; return true;
            case "f64": type = OffsetType.F64; return true;
            case "string": type = OffsetType.String; return true;
            case "bytes": type = OffsetType.Bytes; return true;
            default: type = default; return false;
        }
    }
}
