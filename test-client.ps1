# OpenPivot API test client (multi-session)
# Usage:
#   .\test-client.ps1 register -u <user> -p <pass> -n <nick>
#   .\test-client.ps1 login -u <user> -p <pass> [-As <name>]
#   .\test-client.ps1 sessions
#   .\test-client.ps1 status [-As <name>]
#   .\test-client.ps1 me [-As <name>]
#   .\test-client.ps1 refresh [-As <name>]
#   .\test-client.ps1 call -Method POST -Path /v1/collaborator -Body '{...}' [-As <name>]
#   .\test-client.ps1 logout [-As <name>]
#   .\test-client.ps1 help

param(
    [Parameter(Position = 0)]
    [string]$Command = "help",

    [string]$u,
    [string]$p,
    [string]$n,
    [string]$As = "default",
    [string]$Method = "GET",
    [string]$Path,
    [string]$Body
)

$ErrorActionPreference = "Stop"
$BaseUrl = "http://127.0.0.1:3000"

function Get-SessionFile([string]$Name) {
    Join-Path $PSScriptRoot (".session." + $Name + ".json")
}

# ---------- session helpers ----------
function Load-Session([string]$Name) {
    $f = Get-SessionFile $Name
    if (Test-Path $f) { return Get-Content $f -Raw | ConvertFrom-Json }
    return $null
}
function Save-Session([string]$Name, $s) {
    $s | ConvertTo-Json | Set-Content (Get-SessionFile $Name) -Encoding utf8
}
function Remove-SessionFile([string]$Name) {
    $f = Get-SessionFile $Name
    if (Test-Path $f) { Remove-Item $f }
}

# ---------- http helper ----------
function Invoke-Api {
    param(
        [string]$HttpMethod,
        [string]$Route,
        [object]$Payload,
        [string]$AuthAs
    )
    $headers = @{ "Content-Type" = "application/json" }
    if ($AuthAs) {
        $s = Load-Session $AuthAs
        if (-not $s -or -not $s.access_token) {
            Write-Host "[!] No session for '$AuthAs'. Run:  .\test-client.ps1 login -u <user> -p <pass> -As $AuthAs" -ForegroundColor Red
            exit 1
        }
        $headers["Authorization"] = "Bearer $($s.access_token)"
    }

    $params = @{
        Uri     = "$BaseUrl$Route"
        Method  = $HttpMethod
        Headers = $headers
    }
    if ($Payload) { $params.Body = ($Payload | ConvertTo-Json -Compress) }

    try {
        $resp = Invoke-RestMethod @params
        $tag = if ($AuthAs) { "[$AuthAs] " } else { "" }
        Write-Host "$tag[$HttpMethod $Route] 200" -ForegroundColor DarkGray
        return $resp
    } catch {
        $r = $_.Exception.Response
        if ($r) {
            $code = [int]$r.StatusCode
            $reader = New-Object System.IO.StreamReader($r.GetResponseStream())
            $errBody = $reader.ReadToEnd()
            Write-Host "[$HttpMethod $Route] $code" -ForegroundColor Yellow
            if ($errBody) { Write-Host $errBody -ForegroundColor Yellow }
        } else {
            Write-Host "[!] Request failed: $($_.Exception.Message)" -ForegroundColor Red
        }
        return $null
    }
}

# ---------- subcommands ----------
switch ($Command.ToLower()) {

    "register" {
        if (-not $u -or -not $p -or -not $n) {
            Write-Host "Usage: .\test-client.ps1 register -u <user> -p <pass> -n <nick>" -ForegroundColor Yellow
            exit 1
        }
        Invoke-Api -HttpMethod POST -Route "/v1/auth/register" `
            -Payload @{ username = $u; password = $p; nickname = $n }
    }

    "login" {
        if (-not $u -or -not $p) {
            Write-Host "Usage: .\test-client.ps1 login -u <user> -p <pass> [-As <name>]" -ForegroundColor Yellow
            exit 1
        }
        $resp = Invoke-Api -HttpMethod POST -Route "/v1/auth/login" `
            -Payload @{ username = $u; password = $p }
        if ($resp) {
            Save-Session $As $resp
            Write-Host "[+] Session '$As' saved: user=$u" -ForegroundColor Green
        }
    }

    "logout" {
        $s = Load-Session $As
        if ($s) {
            Invoke-Api -HttpMethod POST -Route "/v1/auth/logout" -AuthAs $As | Out-Null
        }
        Remove-SessionFile $As
        Write-Host "[+] Session '$As' cleared" -ForegroundColor Green
    }

    "me" {
        Invoke-Api -HttpMethod GET -Route "/v1/auth/me" -AuthAs $As
    }

    "refresh" {
        $s = Load-Session $As
        if (-not $s -or -not $s.refresh_token) {
            Write-Host "[!] No refresh_token in session '$As'" -ForegroundColor Red
            exit 1
        }
        $resp = Invoke-Api -HttpMethod POST -Route "/v1/auth/refresh" `
            -Payload @{ refresh_token = $s.refresh_token }
        if ($resp) {
            Save-Session $As $resp
            Write-Host "[+] Session '$As' refreshed" -ForegroundColor Green
        }
    }

    "status" {
        $s = Load-Session $As
        if ($s) {
            Write-Host "--- session: $As ---" -ForegroundColor Cyan
            $s | Format-List
        } else {
            Write-Host "(no session '$As')" -ForegroundColor DarkGray
        }
    }

    "sessions" {
        Get-ChildItem $PSScriptRoot -Filter ".session.*.json" | ForEach-Object {
            $name = $_.BaseName -replace '^\.session\.', ''
            $data = Get-Content $_.FullName -Raw | ConvertFrom-Json
            [pscustomobject]@{
                Name   = $name
                UserId = $data.sub
            }
        } | Format-Table -AutoSize
    }

    "call" {
        if (-not $Path) {
            Write-Host "Usage: .\test-client.ps1 call -Method GET -Path /v1/xxx [-Body '{...}'] [-As <name>]" -ForegroundColor Yellow
            exit 1
        }
        $payload = $null
        if ($Body) { $payload = $Body | ConvertFrom-Json }
        Invoke-Api -HttpMethod $Method -Route $Path -Payload $payload -AuthAs $As
    }

    default {
        $help = @(
            "OpenPivot test client (multi-session)",
            "=====================================",
            "  register -u <user> -p <pass> -n <nick>      register",
            "  login -u <user> -p <pass> -As <name>       login, store session as <name>",
            "  sessions                                     list all stored sessions",
            "  status -As <name>                            show one session",
            "  me -As <name>                                GET /v1/auth/me as <name>",
            "  refresh -As <name>                          refresh tokens for <name>",
            "  call -Method POST -Path /v1/xxx -Body '{}' -As <name>",
            "                                              call any authed endpoint as <name>",
            "  logout -As <name>                            logout and drop session"
        )
        $help -join "`n" | Write-Host
    }
}
