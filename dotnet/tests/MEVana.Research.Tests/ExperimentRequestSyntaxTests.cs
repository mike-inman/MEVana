using MEVana.Research.Core;
using Xunit;
namespace MEVana.Research.Tests;
public class ExperimentRequestSyntaxTests
{
    static readonly DateTimeOffset Start = DateTimeOffset.Parse("2026-01-01T00:00:00Z");
    static ExperimentRequest Request(string? id = "Dataset", string? strategy = "V1", IReadOnlyList<string?>? pair = null, IReadOnlyList<string?>? pools = null, long? min = 1, long? max = long.MaxValue, DateTimeOffset? start = null, DateTimeOffset? end = null) => new(id, strategy, start ?? Start, end ?? Start.AddHours(1), pair ?? ["A", "a"], pools ?? ["P", "p"], min, max);
    static void Invalid(ExperimentRequest r, string code) => Assert.Contains(ExperimentRequestSyntaxValidator.Validate(r), e => e.Code == code);
    [Fact] public void RequiredFieldsAreErrorsWithoutExceptions()
    {
        Assert.Equal("required", Assert.Single(ExperimentRequestSyntaxValidator.Validate(null)).Code);
        var errors = ExperimentRequestSyntaxValidator.Validate(new(null, null, null, null, null, null, null, null));
        Assert.Equal(8, errors.Count); Assert.All(errors, e => Assert.Equal("required", e.Code));
        Assert.Equal(errors, ExperimentRequestSyntaxValidator.Validate(new(null, null, null, null, null, null, null, null)));
    }
    [Theory] [InlineData("")] [InlineData(" ")] [InlineData("é")] [InlineData("A/B")] [InlineData("A.B")]
    public void IdentifierCharacters(string id) { Invalid(Request(id: id), "invalid_identifier"); Invalid(Request(strategy: id), "invalid_identifier"); }
    [Fact] public void IdentifierLengthsAndOrdinalLists()
    {
        Assert.Empty(ExperimentRequestSyntaxValidator.Validate(Request(id: new string('A', 64))));
        Invalid(Request(id: new string('A', 65)), "invalid_identifier");
        foreach (var pair in new string?[][] {[], ["A"], ["A", "B", "C"], ["A", "A"], ["A", null], ["A", "é"]}) Invalid(Request(pair: pair), "invalid_token_pair");
        foreach (var pools in new string?[][] {[], ["P"], ["P", "P"], ["P", null], ["P", " "]}) Invalid(Request(pools: pools), "invalid_pool_ids");
        Assert.Empty(ExperimentRequestSyntaxValidator.Validate(Request(pools: Enumerable.Range(0, 16).Select(i => (string?)$"P{i}").ToArray())));
        Invalid(Request(pools: Enumerable.Range(0, 17).Select(i => (string?)$"P{i}").ToArray()), "invalid_pool_ids");
        Assert.Empty(ExperimentRequestSyntaxValidator.Validate(Request(id: "A", strategy: "s-_9")));
    }
    [Fact] public void ExactIntegerAndUtcWindowBoundaries()
    {
        foreach (var (min, max) in new[] {(0L,1L),(-1L,1L),(1L,0L),(2L,1L)}) Invalid(Request(min: min,max: max),"invalid_trade_size");
        Assert.Empty(ExperimentRequestSyntaxValidator.Validate(Request(min:long.MaxValue,max:long.MaxValue)));
        Invalid(Request(min:null), "required"); Invalid(Request(max:null), "required");
        foreach (var end in new[] {Start,Start.AddTicks(-1),Start.AddHours(24).AddTicks(1)}) Invalid(Request(end:end), "invalid_window");
        Assert.Empty(ExperimentRequestSyntaxValidator.Validate(Request(end:Start.AddHours(24))));
        Assert.Empty(ExperimentRequestSyntaxValidator.Validate(Request(start:Start.ToOffset(TimeSpan.FromHours(2)),end:Start.AddHours(1).ToOffset(TimeSpan.FromHours(-2)))));
    }
    [Fact] public void MultipleIndependentErrorsAndNoMutation()
    {
        var pair=new string?[]{"a","A"};var pools=new string?[]{"p","P"};var r=Request(id:"",strategy:"",pair:pair,pools:pools,min:0,end:Start);
        var before=ExperimentRequestSyntaxValidator.Validate(r);Assert.Equal(4,before.Count);Assert.Equal(new string?[]{"a","A"},pair);Assert.Equal(new string?[]{"p","P"},pools);
        Assert.Throws<NotSupportedException>(()=>((IList<ValidationIssue>)before).Clear());
    }
}
