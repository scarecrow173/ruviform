# evidence/rf979 local syntax check — ps1 ParseFile + workflow run-block parse.
$files = @(
    'tools/virtual-audio-driver/Invoke-MidiEndpointProbe.ps1',
    'tools/virtual-audio-driver/Invoke-EndpointProbe.ps1',
    'tools/testsigned-driver/probe-stage.ps1',
    'tools/testsigned-driver/midi-winmm-probe.ps1',
    'tools/testsigned-driver/TestDevice.ps1'
)
$lines = @()
foreach ($f in $files) {
    $errs = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile($f, [ref]$null, [ref]$errs)
    if ($errs.Count -eq 0) {
        $lines += "PARSE OK: $f"
    } else {
        $lines += "PARSE FAIL: $f - $($errs[0].Message)"
    }
}

$yml = Get-Content -Raw '.github/workflows/virtual-audio-driver-probe.yml'
$blocks = [regex]::Matches($yml, '(?ms)^        run: \|\r?\n((?:          [^\r\n]*\r?\n)+)')
$bok = 0
$bng = 0
$ngDetail = @()
$idx = 0
foreach ($b in $blocks) {
    $idx++
    $code = ($b.Groups[1].Value -replace '(?m)^          ', '')
    $errs = $null
    [void][System.Management.Automation.Language.Parser]::ParseInput($code, [ref]$null, [ref]$errs)
    if ($errs.Count -eq 0) {
        $bok++
    } else {
        $bng++
        $ngDetail += "block#${idx}: $($errs[0].Message)"
    }
}
$lines += "workflow run-blocks parse: ok=$bok ng=$bng of $($blocks.Count)"
foreach ($d in $ngDetail) { $lines += "  $d" }
$lines | Tee-Object -FilePath evidence/rf979/local-syntax-check.txt
