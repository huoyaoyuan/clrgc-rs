cargo build
$gcPath = $([System.IO.Path]::GetFullPath(".\target\debug\clrgc_rs.dll"))

foreach ($file in $(Get-ChildItem -Path ".\tests" -Filter "*.cs")) {
    dotnet build $file
    $exe = ".\tests\bin\debug\$([System.IO.Path]::GetFileNameWithoutExtension($file)).exe"
    $process = Start-Process -FilePath $exe -NoNewWindow -PassThru -RedirectStandardOutput $([System.IO.Path]::ChangeExtension($file, ".log")) -Wait -Environment @{
        "DOTNET_GCPath" = $gcPath
        "DOTNET_DbgEnableMiniDump" = "1"
        "RUST_BACKTRACE" = "1"
    }
    if ($process.ExitCode -ne 0) {
        Write-Host "$($file.Name) exited with code $('0x{0:X8}' -f $process.ExitCode)" -ForegroundColor Red
    }
    else {
        Write-Host "$($file.Name) passed." -ForegroundColor Green
    }
}
