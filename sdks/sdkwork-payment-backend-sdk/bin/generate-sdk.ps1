param(
    [string[]]$Languages = @("typescript"),
    [string]$BaseUrl = "http://localhost:8080",
    [string]$SdkVersion = "1.0.0"
)

$ErrorActionPreference = "Stop"

function Resolve-PackageName {
    param([string]$Language)

    switch ($Language) {
        "typescript" { return "@sdkwork/payment-backend-sdk" }
        default { return "sdkwork-payment-backend-sdk-$Language" }
    }
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$FamilyRoot = (Get-Item $ScriptDir).Parent.FullName
$GeneratorPath = Join-Path $FamilyRoot "..\..\sdkwork-sdk-generator\bin\sdkgen.js"
$InputPath = Join-Path $FamilyRoot "openapi\sdkwork-payment-backend-api.sdkgen.yaml"
$SdkName = "sdkwork-payment-backend-sdk"
$ApiPrefix = "/backend/v3/api"

if (-not (Test-Path $GeneratorPath)) {
    throw "Canonical SDK generator not found: $GeneratorPath. Materialize the sdkwork-sdk-generator workspace (sibling of sdkwork-payment) before running this script. Per SDK_WORKSPACE_GENERATION_SPEC.md the generator MUST be present; the script refuses to fall back to a missing or partial generator."
}
if (-not (Test-Path $InputPath)) {
    throw "OpenAPI sdkgen input not found: $InputPath. The authority spec must be mirrored into the family openapi/ directory before generation."
}

foreach ($LanguageValue in $Languages) {
    foreach ($LanguagePart in "$LanguageValue".Split(",")) {
        $Language = $LanguagePart.Trim()
        if ([string]::IsNullOrWhiteSpace($Language)) {
            continue
        }

        $LanguageWorkspace = Join-Path $FamilyRoot "$SdkName-$Language"
        $OutputPath = Join-Path $LanguageWorkspace "generated\server-openapi"
        $PackageName = Resolve-PackageName $Language
        $ResolvedLanguageWorkspace = [System.IO.Path]::GetFullPath($LanguageWorkspace)
        $ResolvedOutputPath = [System.IO.Path]::GetFullPath($OutputPath)
        $LanguageWorkspacePrefix = $ResolvedLanguageWorkspace.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar

        if (-not $ResolvedOutputPath.StartsWith($LanguageWorkspacePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to clean SDK output outside language workspace: $ResolvedOutputPath"
        }

        if (Test-Path $OutputPath) {
            Remove-Item -LiteralPath $OutputPath -Recurse -Force
        }
        Write-Host "Generating $Language SDK at $OutputPath" -ForegroundColor Cyan
        & node $GeneratorPath generate `
            -i $InputPath `
            -o $OutputPath `
            -n $SdkName `
            -t backend `
            -l $Language `
            --fixed-sdk-version $SdkVersion `
            --base-url $BaseUrl `
            --api-prefix $ApiPrefix `
            --package-name $PackageName `
            --standard-profile sdkwork-v3 `
            --sdk-root $FamilyRoot `
            --sdk-name $SdkName `
            --no-sync-published-version

        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
}
