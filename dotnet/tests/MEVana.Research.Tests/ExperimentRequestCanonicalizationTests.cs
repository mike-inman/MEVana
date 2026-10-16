using System.Globalization;
using System.Text.Json;
using MEVana.Research.Core;
using Xunit;
namespace MEVana.Research.Tests;
public class ExperimentRequestCanonicalizationTests
{
    static readonly DateTimeOffset Start=DateTimeOffset.Parse("2026-01-01T00:00:00Z");
    static ExperimentRequest Request(string? id="Fixture",string? strategy="V",string?[]? pair=null,string?[]? pools=null,long? min=1,long? max=long.MaxValue,DateTimeOffset? start=null,DateTimeOffset? end=null)=>new(id,strategy,start??Start,end??Start.AddHours(1),pair??["A","a"],pools??["p","P"],min,max);
    static string Canon(ExperimentRequest r)=>ExperimentRequestCanonicalizer.Canonicalize(r).Content;
    [Fact] public void ExactVersionOrderAndNumbers()
    {
        var r=Request();var c=ExperimentRequestCanonicalizer.Canonicalize(r);Assert.Equal(1,c.Version);Assert.Equal(c,ExperimentRequestCanonicalizer.Canonicalize(r));
        using var json=JsonDocument.Parse(c.Content);Assert.Equal(new[]{"canonicalizationVersion","datasetId","strategyVersion","windowStartUtc","windowEndUtc","tokenPair","poolIds","minTradeSizeQuoteAtoms","maxTradeSizeQuoteAtoms"},json.RootElement.EnumerateObject().Select(p=>p.Name));
        Assert.Equal(1,json.RootElement.GetProperty("canonicalizationVersion").GetInt32());Assert.Equal(long.MaxValue,json.RootElement.GetProperty("maxTradeSizeQuoteAtoms").GetInt64());
        Assert.Equal("2026-01-01T00:00:00.0000000Z",json.RootElement.GetProperty("windowStartUtc").GetString());
        Assert.Equal(new[]{"P","p"},json.RootElement.GetProperty("poolIds").EnumerateArray().Select(e=>e.GetString()));
        Assert.Equal(new[]{"A","a"},json.RootElement.GetProperty("tokenPair").EnumerateArray().Select(e=>e.GetString()));
    }
    [Fact] public void EquivalentOffsetsAndPoolsIgnoreCultureWithoutMutation()
    {
        var pools=new string?[]{"p","P"};var pair=new string?[]{"A","a"};var normal=Canon(Request(pair:pair,pools:pools));
        var previous=CultureInfo.CurrentCulture;
        try {CultureInfo.CurrentCulture=new("tr-TR");Assert.Equal(normal,Canon(Request(pools:["P","p"],start:Start.ToOffset(TimeSpan.FromHours(2)),end:Start.AddHours(1).ToOffset(TimeSpan.FromHours(-3)))));}
        finally {CultureInfo.CurrentCulture=previous;}
        Assert.Equal(new string?[]{"p","P"},pools);Assert.Equal(new string?[]{"A","a"},pair);
    }
    [Fact] public void SemanticChangesRemainDistinctAndCatalogIndependent()
    {
        var baseline=Canon(Request());
        foreach(var r in new[]{Request(id:"fixture"),Request(strategy:"v"),Request(pair:["a","A"]),Request(pools:["P","Q"]),Request(start:Start.AddTicks(1)),Request(end:Start.AddHours(2)),Request(min:2),Request(max:long.MaxValue-1)})Assert.NotEqual(baseline,Canon(r));
        Assert.NotEqual(Canon(Request(min:9007199254740992)),Canon(Request(min:9007199254740993)));
        Assert.NotEqual(Canon(Request(max:long.MaxValue)),Canon(Request(max:long.MaxValue-1)));
        Assert.Contains("Fixture",baseline);Assert.Empty(ResearchCatalog.Empty.Datasets);
    }
    [Fact] public void InvalidInputsCannotCanonicalize()
    {
        Assert.Throws<ArgumentNullException>(()=>ExperimentRequestCanonicalizer.Canonicalize(null!));
        foreach(var r in new[]{Request(id:null),Request(id:""),Request(strategy:"é"),Request(pair:["A","A"]),Request(pair:["A",null]),Request(pools:["P","P"]),Request(pools:["P"]),Request(min:null),Request(min:0),Request(min:2,max:1),Request(end:Start),Request(end:Start.AddHours(24).AddTicks(1)),new ExperimentRequest("F","V",null,Start,["A","B"],["P","Q"],1,2)})Assert.ThrowsAny<ArgumentException>(()=>Canon(r));
    }
}
