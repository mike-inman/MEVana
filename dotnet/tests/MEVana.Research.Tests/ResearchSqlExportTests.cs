using MEVana.Research.Persistence;
using Xunit;

namespace MEVana.Research.Tests;

// 2026-10-31: Export acceptance checks use the real pinned seed and a disposable tamper copy.
public sealed class ResearchSqlExportTests
{
    static readonly string Seed = Path.Combine(AppContext.BaseDirectory, "research-artifacts");

    [Fact]
    public void RealSeedExportsFourExperimentsAndTwelvePolicies()
    {
        var directory = Directory.CreateTempSubdirectory("research-sql-export-").FullName;
        try
        {
            var output = Path.Combine(directory, "seed.sql");
            Assert.Equal(4, SqlExport.WriteValidated(Seed, output));
            var sql = File.ReadAllText(output);
            Assert.Equal(4, Count(sql, "INSERT INTO research.experiments "));
            Assert.Equal(12, Count(sql, "INSERT INTO research.policy_results "));
            Assert.Equal(6, Count(sql, "INSERT INTO research.shape_results "));
            Assert.Contains(",NULL,", sql, StringComparison.Ordinal);
            Assert.StartsWith("--", sql, StringComparison.Ordinal);
            Assert.EndsWith("COMMIT;\n", sql, StringComparison.Ordinal);
        }
        finally { Directory.Delete(directory, true); }
    }

    [Fact]
    public void MissingValuesAndApostrophesBecomeSafeLiterals()
    {
        Assert.Equal("NULL", SqlExport.Quote(null));
        Assert.Equal("'O''Hara\\path'", SqlExport.Quote("O'Hara\\path"));
        Assert.Equal("''", SqlExport.Quote(""));
    }

    [Fact]
    public void TamperedSeedProducesNoOutput()
    {
        var directory = Directory.CreateTempSubdirectory("research-sql-tamper-").FullName;
        try
        {
            foreach (var file in Directory.GetFiles(Seed))
                File.Copy(file, Path.Combine(directory, Path.GetFileName(file)));
            File.AppendAllText(Path.Combine(directory, "catalog.json"), " ");
            var output = Path.Combine(directory, "seed.sql");
            Assert.Throws<ArtifactIntegrityException>(() => SqlExport.WriteValidated(directory, output));
            Assert.False(File.Exists(output));
        }
        finally { Directory.Delete(directory, true); }
    }

    static int Count(string text, string value) => text.Split(value, StringSplitOptions.None).Length - 1;
}
