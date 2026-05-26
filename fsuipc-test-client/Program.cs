using FsuipcTestClient;

if (args.Length == 0 || args[0] == "--help" || args[0] == "-h")
{
    Console.Error.WriteLine("""
        FSUIPC offset test client

        Usage:
          fsuipc-test-client <input-file>             TUI mode (interactive)
          fsuipc-test-client <input-file> --batch     Batch mode (JSON to stdout)

        Input file format: <address>,<type>[,<size>]
          Types: u8, i8, u16, i16, u32, i32, f32, u64, i64, f64, string, bytes
          Example:
            0x02BC,i32
            0x3160,string,24
            0x0238,bytes,10
        """);
    return 1;
}

var inputPath = args[0];
var isBatch = args.Length > 1 && (args[1] == "--batch" || args[1] == "-b");

if (!File.Exists(inputPath))
{
    Console.Error.WriteLine($"File not found: {inputPath}");
    return 1;
}

return isBatch
    ? await BatchMode.Run(inputPath)
    : await TuiMode.Run(inputPath);
