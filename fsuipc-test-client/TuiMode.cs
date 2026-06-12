using System.Globalization;
using System.Net;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace FsuipcTestClient;

public static class TuiMode
{
    static readonly TimeSpan TickInterval = TimeSpan.FromMilliseconds(100);
    static readonly TimeSpan RefreshInterval = TimeSpan.FromMilliseconds(500);

    enum KeyAction { None, Quit, Reload, Save, Add, Edit, Delete, WriteFile }

    public static async Task<int> Run(string inputPath)
    {
        var (defs, errors) = OffsetParser.Parse(inputPath);
        string hostname = Dns.GetHostName().Split(".")[0];

        if (errors.Count > 0)
        {
            foreach (var e in errors)
                AnsiConsole.MarkupLine($"[red]{e.EscapeMarkup()}[/]");
            return 1;
        }

        if (defs.Count == 0)
        {
            AnsiConsole.MarkupLine("[red]No offsets defined in input file[/]");
            return 1;
        }

        using var client = new FsuipcClient();
        Exception? connError = null;

        try
        {
            client.Connect();
        }
        catch (Exception ex)
        {
            connError = ex;
        }

        if (connError != null)
        {
            AnsiConsole.MarkupLine($"[red]FSUIPC connection failed: {connError.Message.EscapeMarkup()}[/]");
            AnsiConsole.MarkupLine("[yellow]Make sure FSUIPC7 or the uipc-debug IPC host is running[/]");
            return 1;
        }

        client.RegisterOffsets(defs);

        var state = new TuiState
        {
            FilePath = inputPath,
            Defs = defs,
        };

        try
        {
            client.Process();
        }
        catch (Exception ex)
        {
            state.LastError = $"Process failed: {ex.Message}";
        }

        while (true)
        {
            var action = await RunLive(client, state);

            switch (action)
            {
                case KeyAction.Quit:
                    if (state.IsDirty)
                    {
                        if (!AnsiConsole.Confirm("Unsaved changes to mappings. Quit anyway?"))
                            continue;
                    }
                    return 0;

                case KeyAction.Add:
                    var added = PromptAdd();
                    if (added != null)
                    {
                        state.Defs.Add(added);
                        state.IsDirty = true;
                        Reregister(client, state.Defs);
                        state.LastError = $"Added offset 0x{FormatAddress(added.Address)}";
                    }
                    break;

                case KeyAction.Edit:
                    if (state.Defs.Count > 0)
                    {
                        var existing = state.Defs[state.SelectedIndex];
                        var edited = PromptEdit(existing);
                        if (edited != null)
                        {
                            state.Defs[state.SelectedIndex] = edited;
                            state.IsDirty = true;
                            Reregister(client, state.Defs);
                            state.LastError = $"Edited offset 0x{FormatAddress(edited.Address)}";
                        }
                    }
                    break;

                case KeyAction.Delete:
                    if (state.Defs.Count > 0)
                    {
                        var removed = state.Defs[state.SelectedIndex];
                        state.Defs.RemoveAt(state.SelectedIndex);
                        if (state.SelectedIndex >= state.Defs.Count && state.SelectedIndex > 0)
                            state.SelectedIndex--;
                        state.IsDirty = true;
                        Reregister(client, state.Defs);
                        state.LastError = $"Deleted offset 0x{FormatAddress(removed.Address)}";
                    }
                    break;

                case KeyAction.Reload:
                    {
                        var (newDefs, newErrors) = OffsetParser.Parse(state.FilePath);
                        if (newErrors.Count > 0)
                        {
                            state.LastError = $"Reload errors: {string.Join("; ", newErrors)}";
                        }
                        else if (newDefs.Count == 0)
                        {
                            state.LastError = "Reload: no offsets in file";
                        }
                        else
                        {
                            state.Defs = newDefs;
                            state.SelectedIndex = 0;
                            state.IsDirty = false;
                            Reregister(client, state.Defs);
                            state.LastError = $"Reloaded {newDefs.Count} offsets from {Path.GetFileName(state.FilePath)}";
                        }
                        break;
                    }

                case KeyAction.Save:
                    {
                        var snapshot = BatchMode.FormatSnapshot(client.Handles);
                        var json = System.Text.Json.JsonSerializer.Serialize(snapshot, new System.Text.Json.JsonSerializerOptions
                        {
                            WriteIndented = true,
                            PropertyNamingPolicy = System.Text.Json.JsonNamingPolicy.CamelCase
                        });
                        var savePath = $"fsuipc-snapshot-{DateTime.Now:yyyyMMdd-HHmmss}-{hostname}.json";
                        File.WriteAllText(savePath, json);
                        state.LastError = $"Saved to {savePath}";
                        break;
                    }

                case KeyAction.WriteFile:
                    {
                        OffsetParser.WriteFile(state.FilePath, state.Defs);
                        state.IsDirty = false;
                        state.LastError = $"Written {state.Defs.Count} offsets to {Path.GetFileName(state.FilePath)}";
                        break;
                    }
            }
        }
    }

    static void Reregister(FsuipcClient client, List<OffsetDefinition> defs)
    {
        client.ClearOffsets();
        client.RegisterOffsets(defs);
    }

    static async Task<KeyAction> RunLive(FsuipcClient client, TuiState state)
    {
        KeyAction exitAction = KeyAction.None;

        await AnsiConsole.Live(CreateTable(client, state))
            .AutoClear(false)
            .Overflow(VerticalOverflow.Ellipsis)
            .StartAsync(async ctx =>
            {
                var lastRefresh = DateTime.UtcNow;

                while (true)
                {
                    var now = DateTime.UtcNow;
                    if (now - lastRefresh >= RefreshInterval)
                    {
                        try
                        {
                            client.Process();
                            state.LastError = null;
                        }
                        catch (Exception ex)
                        {
                            state.LastError = $"Process failed: {ex.Message}";
                        }
                        lastRefresh = now;
                    }

                    ctx.UpdateTarget(CreateTable(client, state));

                    if (Console.KeyAvailable)
                    {
                        var key = Console.ReadKey(true);
                        var action = HandleKey(key, state, client);

                        if (action != KeyAction.None)
                        {
                            exitAction = action;
                            break;
                        }
                    }

                    await Task.Delay(TickInterval);
                }
            });

        return exitAction;
    }

    static KeyAction HandleKey(ConsoleKeyInfo key, TuiState state, FsuipcClient client)
    {
        switch (key.Key)
        {
            case ConsoleKey.UpArrow:
                if (state.SelectedIndex > 0) state.SelectedIndex--;
                return KeyAction.None;
            case ConsoleKey.DownArrow:
                if (state.SelectedIndex < client.Handles.Count - 1) state.SelectedIndex++;
                return KeyAction.None;
            case ConsoleKey.H:
            case ConsoleKey.F1:
                state.ShowHelp = !state.ShowHelp;
                return KeyAction.None;
            case ConsoleKey.Escape:
                state.ShowHelp = false;
                return KeyAction.None;
            case ConsoleKey.Q:
                return KeyAction.Quit;
            case ConsoleKey.R:
                return KeyAction.Reload;
            case ConsoleKey.S:
                return KeyAction.Save;
            case ConsoleKey.A:
                return KeyAction.Add;
            case ConsoleKey.E:
                return KeyAction.Edit;
            case ConsoleKey.D:
                return KeyAction.Delete;
            case ConsoleKey.W:
                return KeyAction.WriteFile;
            default:
                return KeyAction.None;
        }
    }

    static OffsetDefinition? PromptAdd()
    {
        AnsiConsole.MarkupLine("[bold]Add Offset[/]");
        AnsiConsole.MarkupLine("──────────────────────────");

        var addrStr = AnsiConsole.Prompt(
            new TextPrompt<string>("Address (hex, e.g. 0x02BC — or c to cancel):")
                .Validate(s =>
                {
                    if (s.Equals("c", StringComparison.OrdinalIgnoreCase))
                        return ValidationResult.Success();
                    return TryParseAddress(s, out _)
                        ? ValidationResult.Success()
                        : ValidationResult.Error("Expected hex address 0x0000-0xFFFFF");
                }));

        if (addrStr.Equals("c", StringComparison.OrdinalIgnoreCase))
            return null;

        TryParseAddress(addrStr, out var address);

        var typeStr = AnsiConsole.Prompt(
            new SelectionPrompt<string>()
                .Title("Type:")
                .AddChoices("Cancel", "u8", "i8", "u16", "i16", "u32", "i32", "f32", "u64", "i64", "f64", "string", "bytes"));

        if (typeStr == "Cancel")
            return null;

        var type = ParseType(typeStr);

        int size;
        if (TypeInfo.IsFixedSize(type))
        {
            size = TypeInfo.FixedSize(type);
        }
        else
        {
            size = AnsiConsole.Prompt(
                new TextPrompt<int>("Size (or 0 to cancel):")
                    .Validate(n => n >= 0
                        ? ValidationResult.Success()
                        : ValidationResult.Error("Size must be non-negative")));

            if (size == 0)
                return null;
        }

        return new OffsetDefinition(address, type, size);
    }

    static OffsetDefinition? PromptEdit(OffsetDefinition existing)
    {
        AnsiConsole.MarkupLine("[bold]Edit Offset[/]");
        AnsiConsole.MarkupLine("──────────────────────────");

        var defaultAddr = $"0x{FormatAddress(existing.Address)}";
        var addrStr = AnsiConsole.Prompt(
            new TextPrompt<string>("Address (hex, e.g. 0x02BC — or c to cancel):")
                .DefaultValue(defaultAddr)
                .Validate(s =>
                {
                    if (s.Equals("c", StringComparison.OrdinalIgnoreCase))
                        return ValidationResult.Success();
                    return TryParseAddress(s, out _)
                        ? ValidationResult.Success()
                        : ValidationResult.Error("Expected hex address 0x0000-0xFFFFF");
                }));

        if (addrStr.Equals("c", StringComparison.OrdinalIgnoreCase))
            return null;

        TryParseAddress(addrStr, out var address);

        var existingTypeStr = TypeLabel(existing.Type);
        var choices = new List<string> { "Cancel", "u8", "i8", "u16", "i16", "u32", "i32", "f32", "u64", "i64", "f64", "string", "bytes" };

        var typeStr = AnsiConsole.Prompt(
            new SelectionPrompt<string>()
                .Title($"Type (current: {existingTypeStr}):")
                .AddChoices(choices));

        if (typeStr == "Cancel")
            return null;

        var type = ParseType(typeStr);

        int size;
        if (TypeInfo.IsFixedSize(type))
        {
            size = TypeInfo.FixedSize(type);
        }
        else
        {
            var defaultSize = (!TypeInfo.IsFixedSize(existing.Type)) ? existing.Size : 1;
            size = AnsiConsole.Prompt(
                new TextPrompt<int>("Size (or 0 to cancel):")
                    .DefaultValue(defaultSize)
                    .Validate(n => n >= 0
                        ? ValidationResult.Success()
                        : ValidationResult.Error("Size must be non-negative")));

            if (size == 0)
                return null;
        }

        return new OffsetDefinition(address, type, size);
    }

    static IRenderable CreateTable(FsuipcClient client, TuiState state)
    {
        if (state.ShowHelp)
            return HelpPanel();

        var modified = state.IsDirty ? " [yellow][[modified]][/]" : "";
        var conn = client.IsConnected ? "CONNECTED" : "DISCONNECTED";
        var statusLine = $"{Path.GetFileName(state.FilePath).EscapeMarkup()}{modified} | {client.Handles.Count} offsets | {conn} | h for help";

        if (state.LastError != null)
            statusLine += $"\n{state.LastError.EscapeMarkup()}";

        var table = new Table()
            .Border(TableBorder.Simple)
            .Caption(statusLine)
            .AddColumn(new TableColumn(" ").Width(2))
            .AddColumn(new TableColumn("Address").Width(8))
            .AddColumn(new TableColumn("Type").Width(8))
            .AddColumn(new TableColumn("Size").Width(4))
            .AddColumn(new TableColumn("Value").Width(30));

        for (int i = 0; i < client.Handles.Count; i++)
        {
            var h = client.Handles[i];
            var isSelected = i == state.SelectedIndex;
            var prefix = isSelected ? "▸" : " ";
            var addr = $"0x{FormatAddress(h.Def.Address)}";
            var typeStr = TypeLabel(h.Def.Type);
            var sizeStr = TypeInfo.IsFixedSize(h.Def.Type) ? "" : h.Def.Size.ToString();

            var val = h.Value;
            string valStr = val switch
            {
                null => "[dim]—[/]",
                byte[] buf => Convert.ToHexString(buf),
                string s when s.Length > 60 => s[..57] + "...",
                _ => val.ToString() ?? "[dim]—[/]"
            };

            if (isSelected)
            {
                table.AddRow(
                    new Markup($"[reverse]{prefix}[/]"),
                    new Markup($"[reverse]{addr}[/]"),
                    new Markup($"[reverse]{typeStr}[/]"),
                    new Markup($"[reverse]{sizeStr}[/]"),
                    new Markup($"[reverse]{valStr.EscapeMarkup()}[/]")
                );
            }
            else
            {
                table.AddRow(
                    new Text(prefix),
                    new Text(addr),
                    new Text(typeStr),
                    new Text(sizeStr),
                    new Markup(valStr.EscapeMarkup())
                );
            }
        }

        return table;
    }

    static Panel HelpPanel()
    {
        var content = new Markup(
            "[bold]Keybindings[/]\n" +
            "──────────────────────────\n" +
            "[yellow]↑[/]/[yellow]↓[/]  Navigate rows\n" +
            "[yellow]a[/]    Add offset\n" +
            "[yellow]e[/]    Edit selected offset\n" +
            "[yellow]d[/]    Delete selected offset\n" +
            "[yellow]w[/]    Write offsets to file\n" +
            "[yellow]r[/]    Reload from file\n" +
            "[yellow]s[/]    Save snapshot to JSON\n" +
            "[yellow]q[/]    Quit\n" +
            "[yellow]h[/]    Toggle help\n" +
            "[yellow]Esc[/]  Close help\n" +
            "──────────────────────────\n" +
            "[dim]Refresh rate: ~2 Hz[/]"
        );

        return new Panel(content)
            .Header(" Help ")
            .Border(BoxBorder.Rounded);
    }

    static string FormatAddress(int address) =>
        address > 0xFFFF ? $"{address:X5}" : $"{address:X4}";

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
        OffsetType.String => "str",
        OffsetType.Bytes => "bytes",
        _ => "?"
    };

    static OffsetType ParseType(string s) => s.ToLowerInvariant() switch
    {
        "u8" => OffsetType.U8,
        "i8" => OffsetType.I8,
        "u16" => OffsetType.U16,
        "i16" => OffsetType.I16,
        "u32" => OffsetType.U32,
        "i32" => OffsetType.I32,
        "f32" => OffsetType.F32,
        "u64" => OffsetType.U64,
        "i64" => OffsetType.I64,
        "f64" => OffsetType.F64,
        "string" => OffsetType.String,
        "bytes" => OffsetType.Bytes,
        _ => throw new ArgumentException($"Unknown type: {s}")
    };

    static bool TryParseAddress(string s, out int address)
    {
        address = 0;
        if (s.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
            s = s[2..];
        return int.TryParse(s, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out address)
            && address >= 0 && address <= 0xFFFFF;
    }
}

class TuiState
{
    public string FilePath { get; set; } = "";
    public List<OffsetDefinition> Defs { get; set; } = new();
    public int SelectedIndex { get; set; }
    public bool ShowHelp { get; set; }
    public bool IsDirty { get; set; }
    public string? LastError { get; set; }
}
