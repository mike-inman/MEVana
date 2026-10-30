namespace MEVana.Research.Core;

public static class ExperimentResultV2Canonicalizer
{
    public static CanonicalResult Canonicalize(ExperimentResult result)
    {
        ArgumentNullException.ThrowIfNull(result);
        if (ExperimentResultV2SyntaxValidator.Validate(result).Count != 0)
            throw new ArgumentException("Invalid typed result.", nameof(result));
        return new(1, ExperimentResultCanonicalizer.WriteCanonical(result));
    }
}
