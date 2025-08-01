Param(
    [String]$MooseBin='.\target\release\moose2.exe',
    [String]$ServiceAccount='NT Service\Moose2',
    [bool]$AddToPath=$false
)
$ErrorActionPreference = 'Stop'

# install binary
$MOOSE2_BIN = "$env:ProgramFiles\moose2\bin"
New-Item -Type Directory -Force -Path "$MOOSE2_BIN"
Copy-Item -Path $MooseBin -Destination "$MOOSE2_BIN\moose2.exe"
if ($AddToPath) {
    $mpath = [System.Environment]::GetEnvironmentVariable('PATH', [System.EnvironmentVariableTarget]::Machine)
    $mpath += ";$MOOSE2_BIN"
    [System.Environment]::SetEnvironmentVariable('PATH', $mpath, [System.EnvironmentVariableTarget]::Machine)
}

# set config and state home.
$MOOSE2_HOME = "$env:ProgramData\moose2"
New-Item -Type Directory -Force -Path $MOOSE2_HOME
[System.Environment]::SetEnvironmentVariable('MOOSE2_HOME', $MOOSE2_HOME, [System.EnvironmentVariableTarget]::Machine)
# create new acl
$acl = New-Object System.Security.AccessControl.DirectorySecurity
# first disables inheritance (?), second removes all existing inherited access.
$acl.SetAccessRuleProtection($true, $false)
$owner = New-Object System.Security.Principal.NTAccount -ArgumentList $ServiceAccount
$acl.SetOwner($owner)
$aclrules = 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'
$aclrules =
    (New-Object System.Security.AccessControl.FileSystemAccessRule -ArgumentList (, $ServiceAccount + $aclrules)),
    (New-Object System.Security.AccessControl.FileSystemAccessRule -ArgumentList (, 'NT AUTHORITY\SYSTEM' + $aclrules)),
    (New-Object System.Security.AccessControl.FileSystemAccessRule -ArgumentList (, 'BUILTIN\Administrators' + $aclrules))
foreach ($rule in $aclrules) {
    $acl.AddAccessRule($rule)
}
$acl | Set-Acl -Path $MOOSE2_HOME

@{
    # Stateful storage will default to using %MOOSE2_HOME%
    # Best to just change what %MOOSE2_HOME% is.
    listen        = '[::1]:5921'
    # This key is used to derive the symmetric key used to encrypt session cookies.
    # If you want sessions to persist restarts, set this, otherwise comment it out.
    cookie_secret = 'super secret value; delete this to randomly generate one.'
    # See: https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/creating-an-oauth-app
    # Use GitHub Apps for prod & dev redirect support.
    github_oauth2 = @{
        id       = 'oauth2 client id'
        secret   = 'oauth2 client secret'
        redirect = 'http://localhost:5921/auth'
    }
} |
ConvertTo-Json |
Out-File -FilePath "$MOOSE2_HOME\config.json" `
    -Encoding ASCII `
    -NoClobber `
    -ErrorAction 'SilentlyContinue'
# Note that Out-File in PowerShell 5.1 does not have UTF8 without a BOM.

# Firewall
New-NetFirewallRule `
    -DisplayName 'Allow Moose2 Web Server' `
    -Program "$MOOSE2_BIN\moose2.exe" `
    -Direction Inbound -Action Allow

# create service
# New-Service demands a password for -Credential, which we do not have.
# amazing broken quoting... my favorite Powershell 5.1 issues.
& cmd.exe /c (
    'sc.exe create Moose2 binPath= """{0}""" obj= "{1}" start= auto' `
    -f "$MOOSE2_BIN\moose2.exe", $ServiceAccount
)
& sc.exe description Moose2 'Moose2 Web Application'

## TODO: FOR IIS ##
# '{}' | Out-File -NoClobber -FilePath "$MOOSE2_HOME\moose2.json" -Encoding ASCII -ErrorAction 'SilentlyContinue'
# $WEB_ROOT = "C:\inetpub\wwwroot"
# New-Item -Type Directory -Path "$WEB_ROOT\dump" -Force
# New-Item -Type HardLink -Path "$WEB_ROOT\dump\dump.json" -Value "$MOOSE2_HOME\moose2.json"
# fixup ACL inheritance for hardlink.
# & icalcs.exe $WEB_ROOT\dump\dump.json /reset
