# PowerShell Completions

Setup guide for PowerShell completions on Windows, macOS, and Linux.

## Installation

### Windows PowerShell / PowerShell Core

```powershell
# Generate completions
blz completions powershell | Out-String | Invoke-Expression

# To make permanent, add to profile
blz completions powershell >> $PROFILE

# Or create/edit profile manually
notepad $PROFILE
# Add the output of: blz completions powershell
```

### Check Profile Location

```powershell
# View profile path
$PROFILE

# Check if profile exists
Test-Path $PROFILE

# Create profile if needed
if (!(Test-Path $PROFILE)) {
    New-Item -Type File -Path $PROFILE -Force
}
```

## Features

- Command completion
- Parameter completion
- Dynamic argument completion

## Usage

```powershell
blz <Tab>              # Cycle through commands
blz search -<Tab>      # Cycle through parameters
blz add <Tab>          # Complete with files
```

## Aliases & Functions

Add to your PowerShell profile:

```powershell
# Aliases
Set-Alias bs blz search
Set-Alias bg blz get
Set-Alias ba blz add
Set-Alias bl blz list

# Search function
function Blz-Search {
    param([string]$Query)
    blz search $Query --limit 10
}

# Quick get function
function Blz-Quick {
    param([string]$Query)
    $result = blz search $Query --limit 1 -o json | ConvertFrom-Json
    if ($result) {
        blz get $result[0].alias --lines $result[0].lines
    } else {
        Write-Host "No results for: $Query"
    }
}

# Update all sources
function Blz-UpdateAll {
    blz update --all
}
```

## Integration with Windows Terminal

### Custom Key Bindings

Add to Windows Terminal settings.json:

```json
{
    "command": {
        "action": "sendInput",
        "input": "blz search "
    },
    "keys": "ctrl+b"
}
```

## PowerShell Core (Cross-platform)

```powershell
# Install PowerShell Core
# Windows: winget install Microsoft.PowerShell
# macOS: brew install powershell
# Linux: See https://aka.ms/powershell

# Use same installation steps as above
pwsh
blz completions powershell >> $PROFILE
```

## Troubleshooting

### Execution Policy

If scripts are blocked:

```powershell
# Check policy
Get-ExecutionPolicy

# Allow local scripts
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Profile Not Loading

```powershell
# Test profile loads
. $PROFILE

# Check for errors
$Error[0]
```

### Completions Not Working

```powershell
# Re-import completions
blz completions powershell | Out-String | Invoke-Expression

# Check if Tab completion is enabled
Get-PSReadLineKeyHandler -Key Tab
```

## Advanced Features

### JSON Processing

```powershell
# Parse JSON output
$resp = blz search "hooks" -o json | ConvertFrom-Json
$resp.results | ForEach-Object {
    Write-Host "$($_.alias): $($_.headingPath -join ' > ')"
}

# Filter high-score results
$highScore = blz search "async" -o json | ConvertFrom-Json |
    Select-Object -ExpandProperty results |
    Where-Object { $_.score -gt 50 }
```

### Pipeline Integration

```powershell
# Search and select with Out-GridView
blz search "react" -o json |
    ConvertFrom-Json |
    Select-Object -ExpandProperty results |
    Select-Object alias, lines, @{N='Path';E={$_.headingPath -join ' > '}} |
    Out-GridView -PassThru |
    ForEach-Object { blz get $_.alias --lines $_.lines }
```
