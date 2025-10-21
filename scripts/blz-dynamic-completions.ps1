# Dynamic completions for blz (PowerShell)
%
# Provides runtime completions for:
# - Aliases: `search --alias/-s/--source`, positional for `get`, `update`, `remove`, `toc`, and `anchor list|get` (alias: `anchors`)
# - Anchors: `anchor get <alias> <anchor>` values
#
# Usage (PowerShell profile):
#   . "$PSScriptRoot/blz-dynamic-completions.ps1"

function Get-BlzAliases {
    try {
        $json = blz list --format json 2>$null | ConvertFrom-Json
    } catch {
        return @()
    }

    $seen = @{}
    $out = @()
    foreach ($item in $json) {
        if ($null -ne $item.alias -and -not $seen.ContainsKey($item.alias)) {
            $out += $item.alias
            $seen[$item.alias] = $true
        }
        if ($null -ne $item.aliases) {
            foreach ($a in $item.aliases) {
                if (-not $seen.ContainsKey($a)) {
                    $out += $a
                    $seen[$a] = $true
                }
            }
        }
    }
    return $out
}

function Get-BlzAnchors {
    param([string]$Alias)
    if (-not $Alias) { return @() }
    try {
        $json = blz toc $Alias --format json 2>$null | ConvertFrom-Json
    } catch {
        return @()
    }
    $out = @()
    if ($null -ne $json -and $json -is [System.Collections.IEnumerable]) {
        foreach ($e in $json) {
            if ($null -ne $e.anchor -and [string]::IsNullOrEmpty($e.anchor) -eq $false) {
                $out += $e.anchor
            }
        }
    }
    return ($out | Select-Object -Unique)
}

Register-ArgumentCompleter -CommandName blz -ScriptBlock {
    param($wordToComplete, $commandAst)

    # Tokenize command line
    $tokens = @()
    $commandAst.CommandElements | ForEach-Object { $tokens += $_.Extent.Text }
    if ($tokens.Count -lt 2) { return }

    $sub = $tokens[1]
    $aliases = Get-BlzAliases

    # Complete for --alias/-s on search
    if ($sub -eq 'search') {
        for ($i = 2; $i -lt $tokens.Count; $i++) {
            if ($tokens[$i] -eq '--alias' -or $tokens[$i] -eq '-s' -or $tokens[$i] -eq '--source') {
                foreach ($a in $aliases) {
                    if ($a -like "$wordToComplete*") {
                        [System.Management.Automation.CompletionResult]::new($a, $a, 'ParameterValue', $a)
                    }
                }
                return
            }
        }
    }

    # Positional alias for common subcommands
    if (@('get','update','remove','toc','anchors') -contains $sub) {
        # tokens: 0=blz 1=sub 2=<alias>
        if ($tokens.Count -le 2) {
            foreach ($a in $aliases) {
                if ($a -like "$wordToComplete*") {
                    [System.Management.Automation.CompletionResult]::new($a, $a, 'ParameterValue', $a)
                }
            }
            return
        }
    }

    # Nested 'anchor' subcommands: list|get
    if ($sub -eq 'anchor') {
        $sub2 = if ($tokens.Count -gt 2) { $tokens[2] } else { '' }
        if ($sub2 -eq 'list') {
            # tokens: 0=blz 1=anchor 2=list 3=<alias>
            if ($tokens.Count -le 3) {
                foreach ($a in $aliases) {
                    if ($a -like "$wordToComplete*") {
                        [System.Management.Automation.CompletionResult]::new($a, $a, 'ParameterValue', $a)
                    }
                }
                return
            }
        }
        elseif ($sub2 -eq 'get') {
            # tokens: 0=blz 1=anchor 2=get 3=<alias> 4=<anchor>
            if ($tokens.Count -le 3) {
                foreach ($a in $aliases) {
                    if ($a -like "$wordToComplete*") {
                        [System.Management.Automation.CompletionResult]::new($a, $a, 'ParameterValue', $a)
                    }
                }
                return
            } elseif ($tokens.Count -le 4) {
                $alias = $tokens[3]
                $anchors = Get-BlzAnchors -Alias $alias
                foreach ($anc in $anchors) {
                    if ($anc -like "$wordToComplete*") {
                        [System.Management.Automation.CompletionResult]::new($anc, $anc, 'ParameterValue', $anc)
                    }
                }
                return
            }
        }
    }
}
