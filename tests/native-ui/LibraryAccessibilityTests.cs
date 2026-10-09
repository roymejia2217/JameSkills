using System.Diagnostics;
using FlaUI.Core;
using FlaUI.Core.AutomationElements;
using FlaUI.Core.Definitions;
using FlaUI.Core.Input;
using FlaUI.Core.Tools;
using FlaUI.UIA3;
using FlaUI.Core.WindowsAPI;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace JameSkills.NativeUiTests;

[TestClass]
[DoNotParallelize]
public sealed class LibraryAccessibilityTests
{
    [TestMethod]
    public void Native_library_supports_accessible_selection_search_and_creation()
    {
        var repositoryRoot = FindRepositoryRoot();
        BuildNativeSmokeHost(repositoryRoot);

        var smokeRoot = Path.Combine(Path.GetTempPath(), $"jameskills-uia-smoke-{Guid.NewGuid():N}");
        Assert.IsFalse(Directory.Exists(smokeRoot), "The test root must be unique and new.");

        FlaUI.Core.Application? application = null;
        try
        {
            var executable = Path.Combine(
                repositoryRoot,
                "target",
                "debug",
                "examples",
                "native_smoke.exe"
            );
            Assert.IsTrue(File.Exists(executable), "The Rust native smoke host must build first.");

            var startInfo = new ProcessStartInfo
            {
                FileName = executable,
                WorkingDirectory = repositoryRoot,
                UseShellExecute = false,
            };
            startInfo.Environment["JAMESKILLS_NATIVE_SMOKE_ROOT"] = smokeRoot;
            application = FlaUI.Core.Application.Launch(startInfo);

            using var automation = new UIA3Automation();
            var window = application.GetMainWindow(automation, TimeSpan.FromSeconds(20));
            Assert.IsNotNull(window, "The native GPUI window did not become available.");

            FindByName(window, "Biblioteca");
            FindByName(window, "2 skills en esta página");
            var importFixture = FindByName(window, "Native Smoke Import Fixture · native-smoke-import · 0.1.0");
            Assert.AreEqual(ControlType.Button, importFixture.ControlType);

            var export = FindByName(window, "Exportar seleccionada").AsButton();
            Assert.IsFalse(
                export.IsPatternSupported(FlaUI.UIA3.Patterns.InvokePattern.Pattern),
                "The unavailable export action must not expose UIA Invoke before selection."
            );
            importFixture.AsButton().Invoke();
            var exportIsInvokable = Retry.WhileFalse(
                () => export.IsPatternSupported(FlaUI.UIA3.Patterns.InvokePattern.Pattern),
                TimeSpan.FromSeconds(5),
                TimeSpan.FromMilliseconds(100)
            ).Result;
            Assert.IsTrue(
                exportIsInvokable,
                "Selecting a published revision exposes the export action to UIA."
            );

            var search = FindByName(window, "Buscar skills").AsTextBox();
            search.Click();
            Keyboard.Type("Native Smoke Import Fixture");
            FindByName(window, "Native Smoke Import Fixture · native-smoke-import · 0.1.0");
            var seedFilteredOut = Retry.WhileTrue(
                () => window.FindFirstDescendant(cf => cf.ByName("Native Smoke Seed · native-smoke-seed · 0.1.0")) is not null,
                TimeSpan.FromSeconds(5),
                TimeSpan.FromMilliseconds(100)
            ).Result;
            Assert.IsTrue(
                seedFilteredOut,
                "The current search must remove nonmatching rows from the accessible tree."
            );

            search.Focus();
            Keyboard.TypeSimultaneously(VirtualKeyShort.CONTROL, VirtualKeyShort.KEY_A);
            Keyboard.Type(VirtualKeyShort.BACK);
            FindByName(window, "Native Smoke Seed · native-smoke-seed · 0.1.0");
            FindByName(window, "Crear skill").AsButton().Invoke();

            var slug = FindByName(window, "Slug portable de la skill").AsTextBox();
            var displayName = FindByName(window, "Nombre visible de la skill").AsTextBox();
            slug.Click();
            Keyboard.Type("uia-created-skill");
            displayName.Click();
            Keyboard.Type("UIA Created Skill");
            FindByName(window, "Crear borrador").AsButton().Invoke();

            var created = FindByName(window, "UIA Created Skill · uia-created-skill · Borrador");
            Assert.AreEqual(ControlType.Button, created.ControlType);

            FindByName(window, "Native Smoke Seed · native-smoke-seed · 0.1.0").AsButton().Invoke();
            FindByName(window, "Importar archivo").AsButton().Invoke();
            var openDialog = FindNativeDialog(automation, "Importar skill portable");
            FindByName(openDialog, "Cancelar").AsButton().Invoke();

            FindByName(window, "Native Smoke Seed · native-smoke-seed · 0.1.0").AsButton().Invoke();
            var exportButton = FindByName(window, "Exportar seleccionada").AsButton();
            Assert.IsTrue(
                Retry.WhileFalse(
                    () => exportButton.IsPatternSupported(FlaUI.UIA3.Patterns.InvokePattern.Pattern),
                    TimeSpan.FromSeconds(5),
                    TimeSpan.FromMilliseconds(100)
                ).Result,
                "Selecting a published revision exposes the export picker action to UIA."
            );
            exportButton.Invoke();
            var saveDialog = FindNativeDialog(automation, "Exportar skill portable");
            FindByName(saveDialog, "Cancelar").AsButton().Invoke();

            FindByName(window, "Native Smoke Seed · native-smoke-seed · 0.1.0").AsButton().Invoke();
            var deleteButton = FindByName(window, "Eliminar").AsButton();
            Assert.IsTrue(
                Retry.WhileFalse(
                    () => deleteButton.IsPatternSupported(FlaUI.UIA3.Patterns.InvokePattern.Pattern),
                    TimeSpan.FromSeconds(5),
                    TimeSpan.FromMilliseconds(100)
                ).Result,
                "Selecting an active published skill exposes its delete preview action."
            );
            deleteButton.Invoke();
            FindByName(window, "Confirmar eliminación reversible").AsButton().Invoke();
            FindByName(
                window,
                "Native Smoke Seed · native-smoke-seed · Eliminada · disponible en historial"
            );

            FindByName(window, "Historial").AsButton().Invoke();
            var historicalContent = FindByNamePrefix(window, "v0.1.0 · Contenido · SHA-256 ");
            historicalContent.AsButton().Invoke();
            var restoreVersion = FindByName(window, "Nueva versión SemVer de restauración").AsTextBox();
            restoreVersion.Click();
            Keyboard.Type("0.1.1");
            FindByName(window, "Restaurar selección").AsButton().Invoke();
            FindByName(window, "Confirmar restauración").AsButton().Invoke();
            FindByName(window, "Native Smoke Seed · native-smoke-seed · 0.1.1");

            Assert.IsTrue(
                Directory.Exists(Path.Combine(smokeRoot, "data")),
                "The native host must use the test's temporary data root."
            );
        }
        finally
        {
            application?.Close();
            if (Directory.Exists(smokeRoot))
            {
                Assert.IsTrue(
                    Path.GetFullPath(smokeRoot).StartsWith(
                        Path.GetFullPath(Path.GetTempPath()),
                        StringComparison.OrdinalIgnoreCase
                    ),
                    "Cleanup is restricted to the generated temporary root."
                );
                Directory.Delete(smokeRoot, recursive: true);
            }
        }
    }

    private static AutomationElement FindByName(Window window, string name)
    {
        var result = Retry.WhileNull(
            () => window.FindFirstDescendant(cf => cf.ByName(name)),
            TimeSpan.FromSeconds(5),
            TimeSpan.FromMilliseconds(100)
        ).Result;
        if (result is not null)
        {
            return result;
        }

        var visibleElements = window
            .FindAllDescendants()
            .Take(100)
            .Select(element =>
            {
                var controlType = element.Properties.ControlType.TryGetValue(out var type)
                    ? type.ToString()
                    : "<unsupported type>";
                var elementName = element.Properties.Name.TryGetValue(out var name)
                    ? name
                    : "<unsupported name>";
                return $"{controlType}: '{elementName}'";
            });
        throw new AssertFailedException(
            $"UI Automation could not find '{name}'. Accessible descendants: {string.Join("; ", visibleElements)}"
        );
    }

    private static AutomationElement FindByNamePrefix(Window window, string prefix)
    {
        var result = Retry.WhileNull(
            () => window.FindAllDescendants().FirstOrDefault(element =>
                element.Properties.Name.TryGetValue(out var name)
                && name is not null
                && name.StartsWith(prefix, StringComparison.Ordinal)
            ),
            TimeSpan.FromSeconds(10),
            TimeSpan.FromMilliseconds(100)
        ).Result;
        return result ?? throw new AssertFailedException(
            $"UI Automation could not find a name starting with '{prefix}'."
        );
    }

    private static Window FindNativeDialog(UIA3Automation automation, string title)
    {
        var desktop = automation.GetDesktop();
        var result = Retry.WhileNull(
            () => desktop
                .FindAllDescendants(cf => cf.ByControlType(ControlType.Window))
                .Select(element => element.AsWindow())
                .FirstOrDefault(window =>
                    window.Properties.Name.TryGetValue(out var name)
                    && string.Equals(name, title, StringComparison.OrdinalIgnoreCase)
                ),
            TimeSpan.FromSeconds(15),
            TimeSpan.FromMilliseconds(100)
        ).Result;
        if (result is not null)
        {
            return result;
        }

        var openWindows = desktop
            .FindAllDescendants(cf => cf.ByControlType(ControlType.Window))
            .Select(element => element.Properties.Name.TryGetValue(out var name) ? name : "<unnamed>");
        throw new AssertFailedException(
            $"UI Automation could not find native dialog '{title}'. Windows: {string.Join("; ", openWindows)}"
        );
    }

    private static void BuildNativeSmokeHost(string repositoryRoot)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "cargo",
            WorkingDirectory = repositoryRoot,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        startInfo.ArgumentList.Add("build");
        startInfo.ArgumentList.Add("-p");
        startInfo.ArgumentList.Add("jameskills-desktop");
        startInfo.ArgumentList.Add("--example");
        startInfo.ArgumentList.Add("native_smoke");
        startInfo.ArgumentList.Add("--features");
        startInfo.ArgumentList.Add("test-support");
        startInfo.ArgumentList.Add("--locked");
        startInfo.Environment["CARGO_BUILD_JOBS"] = "1";

        using var process = Process.Start(startInfo)
            ?? throw new AssertFailedException("Cargo could not start the native smoke build.");
        var stdout = process.StandardOutput.ReadToEndAsync();
        var stderr = process.StandardError.ReadToEndAsync();
        process.WaitForExit();
        Assert.AreEqual(
            0,
            process.ExitCode,
            $"The native smoke host must compile. stdout: {stdout.Result}; stderr: {stderr.Result}"
        );
    }

    private static string FindRepositoryRoot()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory); directory is not null; directory = directory.Parent)
        {
            if (File.Exists(Path.Combine(directory.FullName, "Cargo.lock"))
                && Directory.Exists(Path.Combine(directory.FullName, "crates", "jameskills-desktop")))
            {
                return directory.FullName;
            }
        }
        throw new AssertFailedException("Could not locate the Rust workspace root from the UI test output.");
    }
}
