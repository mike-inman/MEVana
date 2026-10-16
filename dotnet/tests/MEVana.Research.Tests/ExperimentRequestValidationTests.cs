using MEVana.Research.Core;
using Xunit;
namespace MEVana.Research.Tests;
public class ExperimentRequestValidationTests
{
    static readonly DateTimeOffset Start=DateTimeOffset.Parse("2026-01-01T00:00:00Z");
    static ResearchCatalog Fixture(bool available=true,long bound=10) => new([new("Fixture","v1","Test fixture",available,"Tests only",Start,Start.AddHours(2),["V"],[new("A",0,1),new("B",18,10),new("C",0,10)],[new("P",["A","B"]),new("Q",["B","A"]),new("X",["A","C"])],bound)]);
    static ExperimentRequest Request(string id="Fixture",string strategy="V",string?[]? pair=null,string?[]? pools=null,long max=10,DateTimeOffset? start=null,DateTimeOffset? end=null) => new(id,strategy,start??Start,end??Start.AddHours(2),pair??["A","B"],pools??["P","Q"],1,max);
    static void Invalid(ExperimentRequest r,ResearchCatalog c,string code) => Assert.Contains(ExperimentRequestValidator.Validate(r,c),e=>e.Code==code);
    [Fact] public void SyntaxErrorsAreDelegatedBeforeLookup()
    {
        Assert.Equal(ExperimentRequestSyntaxValidator.Validate(null),ExperimentRequestValidator.Validate(null,ResearchCatalog.Empty));
        var raw = new ExperimentRequest("Fixture","V",Start,Start.AddHours(1),null,null,-1,null);
        Assert.Equal(ExperimentRequestSyntaxValidator.Validate(raw),ExperimentRequestValidator.Validate(raw,Fixture()));
        var r=Request(id:"");Assert.Equal(ExperimentRequestSyntaxValidator.Validate(r),ExperimentRequestValidator.Validate(r,Fixture()));
        Assert.Throws<ArgumentNullException>(()=>ExperimentRequestValidator.Validate(r,null!));
    }
    [Fact] public void CatalogAvailabilityAndOrdinalIdentity()
    {
        Invalid(Request(),ResearchCatalog.Empty,"unknown_dataset");Invalid(Request(id:"fixture"),Fixture(),"unknown_dataset");
        Invalid(Request(),Fixture(false),"dataset_unavailable");Invalid(Request(strategy:"v"),Fixture(),"unsupported_strategy");
        Invalid(Request(pair:["a","B"]),Fixture(),"incompatible_pair");Invalid(Request(pools:["P","q"]),Fixture(),"incompatible_pool");Invalid(Request(pools:["P","X"]),Fixture(),"incompatible_pool");
    }
    [Fact] public void CoverageAndSelectedQuoteBound()
    {
        Assert.Empty(ExperimentRequestValidator.Validate(Request(),Fixture()));
        Invalid(Request(start:Start.AddTicks(-1)),Fixture(),"invalid_window");Invalid(Request(end:Start.AddHours(2).AddTicks(1)),Fixture(),"invalid_window");
        Assert.Empty(ExperimentRequestValidator.Validate(Request(start:Start.ToOffset(TimeSpan.FromHours(3))),Fixture()));
        Invalid(Request(max:11),Fixture(),"invalid_trade_size");Invalid(Request(),Fixture(bound:9),"invalid_trade_size");
        Invalid(Request(pair:["B","A"],max:2),Fixture(),"invalid_trade_size");Assert.Empty(ExperimentRequestValidator.Validate(Request(pair:["B","A"],max:1),Fixture()));
        var unusable=Request(pair:["Unknown","B"],pools:["Missing","Q"],max:11,start:Start.AddTicks(-1));
        var issues=ExperimentRequestValidator.Validate(unusable,Fixture());Assert.Contains(issues,e=>e.Code=="invalid_window");Assert.Contains(issues,e=>e.Code=="incompatible_pair");Assert.DoesNotContain(issues,e=>e.Code is "incompatible_pool" or "invalid_trade_size");
        var r=Request();_ = ExperimentRequestValidator.Validate(r,Fixture());Assert.Equal(new string?[]{"A","B"},r.TokenPair);
    }
}
