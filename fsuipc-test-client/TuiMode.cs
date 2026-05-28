using Spectre.Console;
using Spectre.Console.Rendering;

namespace FsuipcTestClient;

public static class TuiMode
{
    static readonly TimeSpan TickInterval = TimeSpan.FromMilliseconds(100);
    static readonly TimeSpan RefreshInterval = TimeSpan.FromMilliseconds(500);

    public static async Task<int> Run(string inputPath)
    {
        var (defs, errors) = OffsetParser.Parse(inputPath);

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

        string? lastError = null;
        client.RegisterOffsets(defs);
        var selectedIndex = 0;
        var showHelp = false;
        var filePath = inputPath;

        try
        {
            client.Process();
        }
        catch (Exception ex)
        {
            lastError = $"Process failed: {ex.Message}";
        }

        await AnsiConsole.Live(CreateTable(client, selectedIndex, lastError, filePath, showHelp))
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
                            lastError = null;
                        }
                        catch (Exception ex)
                        {
                            lastError = $"Process failed: {ex.Message}";
                        }
                        lastRefresh = now;
                    }

                    ctx.UpdateTarget(CreateTable(client, selectedIndex, lastError, filePath, showHelp));

                    if (Console.KeyAvailable)
                    {
                        var key = Console.ReadKey(true);
                        var handled = HandleKey(key, ref selectedIndex, client, ref filePath, ref showHelp, ref lastError);

                        if (handled == KeyAction.Quit)
                            break;

                        if (handled == KeyAction.Reload)
                        {
                            var (newDefs, newErrors) = OffsetParser.Parse(filePath);
                            if (newErrors.Count > 0)
                            {
                                lastError = $"Reload errors: {string.Join("; ", newErrors)}";
                            }
                            else if (newDefs.Count == 0)
                            {
                                lastError = "Reload: no offsets in file";
                            }
                            else
                            {
                                client.ClearOffsets();
                                client.RegisterOffsets(newDefs);
                                selectedIndex = 0;
                                lastError = $"Reloaded {newDefs.Count} offsets from {Path.GetFileName(filePath)}";
                            }
                        }

                        if (handled == KeyAction.Save)
                        {
                            var snapshot = BatchMode.FormatSnapshot(client.Handles);
                            var json = System.Text.Json.JsonSerializer.Serialize(snapshot, new System.Text.Json.JsonSerializerOptions
                            {
                                WriteIndented = true,
                                PropertyNamingPolicy = System.Text.Json.JsonNamingPolicy.CamelCase
                            });
                            var savePath = $"fsuipc-snapshot-{DateTime.Now:yyyyMMdd-HHmmss}.json";
                            File.WriteAllText(savePath, json);
                            lastError = $"Saved to {savePath}";
                        }
                    }

                    await Task.Delay(TickInterval);
                }
            });

        return 0;
    }

    enum KeyAction { None, Quit, Reload, Save }

    static KeyAction HandleKey(ConsoleKeyInfo key, ref int selectedIndex,
        FsuipcClient client, ref string filePath, ref bool showHelp, ref string? lastError)
    {
        switch (key.Key)
        {
            case ConsoleKey.UpArrow:
                if (selectedIndex > 0) selectedIndex--;
                break;
            case ConsoleKey.DownArrow:
                if (selectedIndex < client.Handles.Count - 1) selectedIndex++;
                break;
            case ConsoleKey.Q:
                return KeyAction.Quit;
            case ConsoleKey.R:
                return KeyAction.Reload;
            case ConsoleKey.S:
                return KeyAction.Save;
            case ConsoleKey.H:
            case ConsoleKey.F1:
                showHelp = !showHelp;
                break;
            case ConsoleKey.Escape:
                showHelp = false;
                break;
        }

        return KeyAction.None;
    }

    static IRenderable CreateTable(FsuipcClient client, int selectedIndex,
        string? lastError, string filePath, bool showHelp)
    {
        if (showHelp)
            return HelpPanel();

        var table = new Table()
            .Border(TableBorder.Simple)
            .Caption($"File: {Path.GetFileName(filePath).EscapeMarkup()}  |  {client.Handles.Count} offsets  |  {(client.IsConnected ? "CONNECTED" : "DISCONNECTED")}")
            .AddColumn(new TableColumn(" ").Width(2))
            .AddColumn(new TableColumn("Address").Width(8))
            .AddColumn(new TableColumn("Type").Width(8))
            .AddColumn(new TableColumn("Size").Width(4))
            .AddColumn(new TableColumn("Value").Width(30));

        for (int i = 0; i < client.Handles.Count; i++)
        {
            var h = client.Handles[i];
            var isSelected = i == selectedIndex;
            var prefix = isSelected ? "▸" : " ";
            var addr = $"0x{h.Def.Address:X4}";
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

        if (lastError != null)
            table.Caption(lastError.EscapeMarkup());

        return table;
    }

    static Panel HelpPanel()
    {
        var content = new Markup(
            "[bold]Keybindings[/]\n" +
            "──────────────────────────\n" +
            "[yellow]↑[/]/[yellow]↓[/]  Navigate rows\n" +
            "[yellow]r[/]    Reload input file\n" +
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
}
