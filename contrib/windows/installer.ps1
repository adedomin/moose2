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
$acl.SetAccessRuleProtection($true, $true)
$owner = New-Object System.Security.Principal.NTAccount($ServiceAccount)
$acl.SetOwner($owner)
$aclrules = 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'
@(
  (New-Object System.Security.AccessControl.FileSystemAccessRule -ArgumentList (, $ServiceAccount + $aclrules)),
  (New-Object System.Security.AccessControl.FileSystemAccessRule -ArgumentList (, 'NT AUTHORITY\SYSTEM' + $aclrules)),
  (New-Object System.Security.AccessControl.FileSystemAccessRule -ArgumentList (, 'BUILTIN\Administrators' + $aclrules))
) | % {
    $acl.AddAccessRule($_)
}
$acl | Set-Acl -Path $MOOSE2_HOME

@'
{ "//": "OPTIONAL: default: $XDG_DATA_HOME/moose2 or $STATE_DIRECTORY/"
, "moose_path":    "C:\ProgramData\moose2"
, "//": "OPTIONAL: default: $XDG_DATA_HOME/moose2/moose2.json or $STATE_DIRECTORY/moose2.json"
, "moose_dump":    "C:\ProgramData\moose2\moose2.json"
, "//": "OPTIONAL: can use unix:/path/to/socket for uds listening."
, "listen":        "[::1]:5921"
, "//": "A symmetric secret key for session cookies; delete for random; is PBKDF padded to 64 bytes."
, "cookie_secret": "super-duper-sekret"
, "//": "github oauth2 client configuration details, omit whole object to disable authentication."
, "github_oauth2":
    { "id":     "client id"
    , "secret": "client secret"
    , "//": "OPTIONAL: defaults depend on oauth provider, gh will redirect to auth cb url."
    , "redirect": "http://localhost:5921/auth"
    }
}
'@ | Out-File -NoClobber -FilePath "$MOOSE2_HOME\config.json" -Encoding ASCII -ErrorAction 'SilentlyContinue'
# Note, UTF8NoBOM is missing from Powershell 5.1

# Firewall
New-NetFirewallRule `
    -DisplayName 'Allow Moose2 Web Server' `
    -Program "$MOOSE2_BIN\moose2.exe" `
    -Direction Inbound -Action Allow

# create service
# New-Service demands a password for -Credential, which we do not have.
# amazing broken quoting... my favorite Powershell 5.1 issues.
& cmd.exe /c ('sc.exe create Moose2 binPath= """{0}"" svc" obj= "{1}" start= auto' -f "$MOOSE2_BIN\moose2.exe", $ServiceAccount)
& sc.exe description Moose2 'Moose2 Web Application'
