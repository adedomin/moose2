Param(
    [String]$MooseBin='.\target\release\moose2.exe',
    [bool]$AddToPath=$false,
    [String]$Username='Moose2',
    [SecureString]$Password
)
$ErrorActionPreference = 'Stop'

Function Get-RandomPassword {
    $seed = [Byte[]]::new(4)
    $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    $rng.GetBytes($seed)
    $seed = [System.BitConverter]::ToInt32($seed, 0)
    $rng = [Random]::new($seed)
    $password = [SecureString]::new()
    $chars = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz'
    for ($i = 0; $i -lt 32; $i++) {
        $password.AppendChar($chars[$rng.Next($chars.Length)])
    }
    return $password
}

if ($Password -eq $null) {
    $Password = Get-RandomPassword
}

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
$MOOSE2_HOME = "$env:AllUsersProfile\moose2"
New-Item -Type Directory -Force -Path $MOOSE2_HOME
[System.Environment]::SetEnvironmentVariable('MOOSE2_HOME', $MOOSE2_HOME, [System.EnvironmentVariableTarget]::Machine)
# create local user and creds
New-LocalUser -Name $Username -Password $Password -FullName 'Moose2 Service' -Description 'Moose2 Service'
$cred = New-Object System.Management.Automation.PSCredential(".\$Username", $Password)
# create new acl
$acl = New-Object System.Security.AccessControl.DirectorySecurity
$acl.SetAccessRuleProtection($true, $true)
$owner = New-Object System.Security.Principal.NTAccount("Moose2")
$acl.SetOwner($owner)
@(
  (New-Object System.Security.AccessControl.FileSystemAccessRule("CREATOR OWNER", "FullControl", "Allow")),
  (New-Object System.Security.AccessControl.FileSystemAccessRule("NT AUTHORITY\SYSTEM", "FullControl", "Allow")),
  (New-Object System.Security.AccessControl.FileSystemAccessRule("BUILTIN\Administrators", "FullControl", "Allow"))
) | % {
    $acl.AddAccessRule($_)
}
$acl | Set-Acl -Path $MOOSE2_HOME

@'
{ "//": "OPTIONAL: default: $XDG_DATA_HOME/moose2 or $STATE_DIRECTORY/"
, "moose_path":    "/path/to/store/meese"
, "//": "OPTIONAL: default: $XDG_DATA_HOME/moose2/moose2.json or $STATE_DIRECTORY/moose2.json"
, "moose_dump":    "/path/to/store/meese.json"
, "//": "OPTIONAL: can use unix:/path/to/socket for uds listening."
, "listen":        "[::1]:5921"
, "//": "A symmetric secret key for session cookies; delete for random; is PBKDF padded to 64 bytes."
, "cookie_secret": "super-duper-sekret"
, "//": "github oauth2 client configuration details, omit whole object to disable authentication."
, "github_oauth2":
    { "id":     "client id"
    , "secret": "client secret"
    , "//": "OPTIONAL: defaults depend on oauth provider, gh will redirect to auth cb url."
    , "redirect": "http://[::1]:5921/auth"
    }
}
'@ | Out-File -NoClobber -FilePath "$MOOSE2_HOME\config.json" -Encoding ASCII -ErrorAction 'SilentlyContinue'
# Note, UTF8NoBOM is missing from Powershell 5.1

# Firewall
New-NetFirewallRule `
    -DisplayName 'Allow Moose2 Web Server' `
    -Program "$MOOSE2_BIN\moose2.exe" `
    -Direction Inbound -Action Allow

# We need SeServiceLogonRight for some insane reason
# this module from Powershell Gallery does it all allegedly.
# need to Install-Module -Name 'Carbon'
Import-Module 'Carbon'
[Carbon.Security.Privilege]::GrantPrivileges("$Username", "SeServiceLogonRight")

# create service
New-Service `
    -Name 'Moose2' `
    -BinaryPathName "`"$MOOSE2_BIN\moose2.exe`" svc" `
    -DisplayName 'Moose2' `
    -Description 'Moose2 Web Application' `
    -Credential $cred
