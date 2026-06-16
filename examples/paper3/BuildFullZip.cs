using System;
using System.IO;
using System.IO.Compression;

class BuildFullZip
{
    static void Main()
    {
        string source = @"e:\work\Tex2Doc\examples\paper3";
        string zipPath = @"e:\work\Tex2Doc\examples\paper3\upload_full.zip";
        string staging = @"e:\work\Tex2Doc\examples\paper3\.upload_full_staging";

        if (File.Exists(zipPath)) File.Delete(zipPath);
        if (Directory.Exists(staging)) Directory.Delete(staging, true);
        Directory.CreateDirectory(staging);
        Directory.CreateDirectory(Path.Combine(staging, "figures"));
        Directory.CreateDirectory(Path.Combine(staging, "sections", "zh"));

        // top level
        string[] topFiles = { "main-jos.tex", "main-zh.tex", "references.bib", "rjthesis.cls" };
        foreach (var f in topFiles)
        {
            string src = Path.Combine(source, "latex", f);
            string dst = Path.Combine(staging, f);
            if (File.Exists(src)) File.Copy(src, dst);
        }

        // figures
        foreach (var f in Directory.GetFiles(Path.Combine(source, "figures"), "*.png"))
        {
            string dst = Path.Combine(staging, "figures", Path.GetFileName(f));
            File.Copy(f, dst);
        }

        // sections
        foreach (var f in Directory.GetFiles(Path.Combine(source, "latex", "sections", "zh"), "*.tex"))
        {
            string dst = Path.Combine(staging, "sections", "zh", Path.GetFileName(f));
            File.Copy(f, dst);
        }

        ZipFile.CreateFromDirectory(staging, zipPath, CompressionLevel.Optimal, false);
        Directory.Delete(staging, true);
        Console.WriteLine($"OK: {zipPath}");
    }
}
