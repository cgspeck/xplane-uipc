using FSUIPC;

namespace FsuipcTestClient;

public class FsuipcClient : IDisposable
{
    readonly List<RegisteredOffset> _handles = new();

    public bool IsConnected { get; private set; }
    public IReadOnlyList<RegisteredOffset> Handles => _handles;

    public void Connect()
    {
        FSUIPCConnection.Open();
        IsConnected = true;
    }

    public void RegisterOffsets(IEnumerable<OffsetDefinition> defs)
    {
        foreach (var def in defs)
            _handles.Add(RegisteredOffset.Create(def));
    }

    public void ClearOffsets()
    {
        foreach (var h in _handles)
            h.Disconnect();
        _handles.Clear();
    }

    public void Process()
    {
        if (!IsConnected) return;
        FSUIPCConnection.Process();
    }

    public void Dispose()
    {
        if (IsConnected)
        {
            ClearOffsets();
            FSUIPCConnection.Close();
            IsConnected = false;
        }
    }
}

public abstract class RegisteredOffset
{
    public OffsetDefinition Def { get; protected set; } = null!;
    public Offset Offset { get; protected set; } = null!;
    public abstract object? Value { get; }
    public void Disconnect() => Offset.Disconnect();

    public static RegisteredOffset Create(OffsetDefinition def) => def.Type switch
    {
        OffsetType.U8 or OffsetType.I8 => new OffsetHandle<byte>(def),
        OffsetType.U16 => new OffsetHandle<ushort>(def),
        OffsetType.I16 => new OffsetHandle<short>(def),
        OffsetType.U32 => new OffsetHandle<uint>(def),
        OffsetType.I32 => new OffsetHandle<int>(def),
        OffsetType.F32 => new OffsetHandle<float>(def),
        OffsetType.U64 => new OffsetHandle<ulong>(def),
        OffsetType.I64 => new OffsetHandle<long>(def),
        OffsetType.F64 => new OffsetHandle<double>(def),
        OffsetType.String => new StringOffsetHandle(def),
        OffsetType.Bytes => new BytesOffsetHandle(def),
        _ => throw new ArgumentException($"Unknown type: {def.Type}")
    };
}

sealed class OffsetHandle<T> : RegisteredOffset
{
    readonly Offset<T> _offset;

    public OffsetHandle(OffsetDefinition def)
    {
        Def = def;
        _offset = new Offset<T>(def.Address);
        Offset = _offset;
    }

    public override object? Value => _offset.Value;
}

sealed class StringOffsetHandle : RegisteredOffset
{
    readonly Offset<string> _offset;

    public StringOffsetHandle(OffsetDefinition def)
    {
        Def = def;
        _offset = new Offset<string>(def.Address, def.Size);
        Offset = _offset;
    }

    public override object? Value => _offset.Value;
}

sealed class BytesOffsetHandle : RegisteredOffset
{
    readonly Offset<byte[]> _offset;

    public BytesOffsetHandle(OffsetDefinition def)
    {
        Def = def;
        _offset = new Offset<byte[]>(def.Address, def.Size);
        Offset = _offset;
    }

    public override object? Value => _offset.Value;
}
