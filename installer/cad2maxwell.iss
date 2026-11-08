; Per-user installer. Build via scripts/build-release.ps1, which passes the /D defines below.
#define AppName "CAD2Maxwell"

[Setup]
; Stable id so upgrades replace the previous install. App identity: com.cad2maxwell.desktop.
AppId={{7C0B6B7E-4D0E-4B8E-9A55-C2D2A0A1F001}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=CAD2Maxwell contributors
AppCopyright=MIT License
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=CAD2Maxwell_{#AppVersion}_x64-setup
SetupIconFile={#IconFile}
UninstallDisplayIcon={app}\CAD2Maxwell.exe
LicenseFile={#LicenseFile}
Compression=lzma2
SolidCompression=yes

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\CAD2Maxwell.exe"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\CAD2Maxwell.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Run]
Filename: "{app}\CAD2Maxwell.exe"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
