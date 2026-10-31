namespace MEVana.Research.Persistence;

/// <summary>A research artifact failed a fail-closed startup check. The API must not serve.</summary>
public sealed class ArtifactIntegrityException(string code, string artifact)
    : Exception($"{code}: {artifact}")
{
    public string Code { get; } = code;
    public string Artifact { get; } = artifact;
}
