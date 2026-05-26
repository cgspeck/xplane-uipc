namespace FsuipcTestClient.Tests;

public class OffsetParserTests
{
    [Fact]
    public void ParseFixedSizeOffsets()
    {
        var path = WriteTemp(
            "0x02BC,i32\n" +
            "0x0D0C,u16\n" +
            "0x3365,u8\n" +
            "0x0560,i64\n" +
            "0x02CC,f64"
        );

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(errors);
        Assert.Equal(5, offsets.Count);
        Assert.Equal(0x02BC, offsets[0].Address);
        Assert.Equal(OffsetType.I32, offsets[0].Type);
        Assert.Equal(4, offsets[0].Size);
        Assert.Equal(OffsetType.U16, offsets[1].Type);
        Assert.Equal(2, offsets[1].Size);
        Assert.Equal(OffsetType.U8, offsets[2].Type);
        Assert.Equal(1, offsets[2].Size);
        Assert.Equal(OffsetType.I64, offsets[3].Type);
        Assert.Equal(8, offsets[3].Size);
        Assert.Equal(OffsetType.F64, offsets[4].Type);
        Assert.Equal(8, offsets[4].Size);
    }

    [Fact]
    public void ParseVariableSizeTypes()
    {
        var path = WriteTemp(
            "0x3160,string,24\n" +
            "0x0238,bytes,10"
        );

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(errors);
        Assert.Equal(2, offsets.Count);
        Assert.Equal(OffsetType.String, offsets[0].Type);
        Assert.Equal(24, offsets[0].Size);
        Assert.Equal(OffsetType.Bytes, offsets[1].Type);
        Assert.Equal(10, offsets[1].Size);
    }

    [Fact]
    public void SkipBlankLinesAndComments()
    {
        var path = WriteTemp(
            "# This is a comment\n" +
            "\n" +
            "   \n" +
            "0x02BC,i32\n" +
            "# Another comment\n" +
            "0x0D0C,u16"
        );

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(errors);
        Assert.Equal(2, offsets.Count);
    }

    [Fact]
    public void RejectMalformedLine()
    {
        var path = WriteTemp(
            "0x02BC\n" +
            "garbage data here\n" +
            "0x0D0C,u16"
        );

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Single(offsets);
        Assert.Equal(2, errors.Count);
        Assert.Contains("Line 1", errors[0]);
        Assert.Contains("Line 2", errors[1]);
    }

    [Fact]
    public void RejectUnknownType()
    {
        var path = WriteTemp("0x02BC,foobar");

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(offsets);
        Assert.Single(errors);
        Assert.Contains("unknown type", errors[0].ToLowerInvariant());
    }

    [Fact]
    public void RejectInvalidHexAddress()
    {
        var path = WriteTemp("0xZZZZ,i32");

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(offsets);
        Assert.Single(errors);
        Assert.Contains("invalid offset address", errors[0].ToLowerInvariant());
    }

    [Fact]
    public void RejectAddressOutOfRange()
    {
        var path = WriteTemp("0x10000,i32");

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(offsets);
        Assert.Single(errors);
    }

    [Fact]
    public void RejectStringWithoutSize()
    {
        var path = WriteTemp("0x3160,string");

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(offsets);
        Assert.Single(errors);
        Assert.Contains("requires a positive size", errors[0].ToLowerInvariant());
    }

    [Fact]
    public void RejectBytesWithoutSize()
    {
        var path = WriteTemp("0x0238,bytes");

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(offsets);
        Assert.Single(errors);
        Assert.Contains("requires a positive size", errors[0].ToLowerInvariant());
    }

    [Fact]
    public void RejectZeroSizeForString()
    {
        var path = WriteTemp("0x3160,string,0");

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(offsets);
        Assert.Single(errors);
    }

    [Fact]
    public void AcceptAllFixedSizeTypes()
    {
        var path = WriteTemp(
            "0x0001,u8\n" +
            "0x0002,i8\n" +
            "0x0003,u16\n" +
            "0x0004,i16\n" +
            "0x0005,u32\n" +
            "0x0006,i32\n" +
            "0x0007,f32\n" +
            "0x0008,u64\n" +
            "0x0009,i64\n" +
            "0x000A,f64"
        );

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(errors);
        Assert.Equal(10, offsets.Count);
        Assert.Equal(1, offsets[0].Size); // u8
        Assert.Equal(1, offsets[1].Size); // i8
        Assert.Equal(2, offsets[2].Size); // u16
        Assert.Equal(2, offsets[3].Size); // i16
        Assert.Equal(4, offsets[4].Size); // u32
        Assert.Equal(4, offsets[5].Size); // i32
        Assert.Equal(4, offsets[6].Size); // f32
        Assert.Equal(8, offsets[7].Size); // u64
        Assert.Equal(8, offsets[8].Size); // i64
        Assert.Equal(8, offsets[9].Size); // f64
    }

    [Fact]
    public void ParseAddressWithout0xPrefix()
    {
        var path = WriteTemp("02BC,i32");

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(errors);
        Assert.Single(offsets);
        Assert.Equal(0x02BC, offsets[0].Address);
    }

    [Fact]
    public void ParseWithInlineComment()
    {
        var path = WriteTemp("0x02BC,i32      # Indicated airspeed");

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(errors);
        Assert.Single(offsets);
        Assert.Equal(0x02BC, offsets[0].Address);
        Assert.Equal(OffsetType.I32, offsets[0].Type);
    }

    [Fact]
    public void ParseMultipleWithInlineComments()
    {
        var path = WriteTemp(
            "0x02BC,i32      # airspeed\n" +
            "0x0D0C,u16      # lights\n" +
            "0x3365,u8"
        );

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(errors);
        Assert.Equal(3, offsets.Count);
    }

    [Fact]
    public void ParseInlineCommentStripsHashBetweenFields()
    {
        var path = WriteTemp("0x02BC,i32#comment after type");

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(errors);
        Assert.Single(offsets);
        Assert.Equal(OffsetType.I32, offsets[0].Type);
    }

    [Fact]
    public void ParseInlineCommentCommentOnlyLine()
    {
        var path = WriteTemp(
            "# comment\n" +
            "# another\n" +
            "0x02BC,i32"
        );

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(errors);
        Assert.Single(offsets);
    }

    [Fact]
    public void HandlesWhitespaceInLines()
    {
        var path = WriteTemp(
            "  0x02BC , i32  \n" +
            "  0x0D0C  ,  u16  "
        );

        var (offsets, errors) = OffsetParser.Parse(path);

        Assert.Empty(errors);
        Assert.Equal(2, offsets.Count);
        Assert.Equal(0x02BC, offsets[0].Address);
        Assert.Equal(OffsetType.I32, offsets[0].Type);
    }

    static string WriteTemp(string content)
    {
        var path = Path.GetTempFileName();
        File.WriteAllText(path, content);
        return path;
    }
}
