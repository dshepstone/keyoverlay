; Inno Setup script for the KeyOverlay Windows installer.
;
; Build locally (Inno Setup 6, https://jrsoftware.org/isinfo.php):
;   cargo build --release -p keyoverlay-app
;   iscc /DAppVersion=0.2.0 installer\keyoverlay.iss
;
; The installer is written to target\installer\. CI builds and publishes it
; automatically for every pushed `v*` tag (see .github/workflows/release.yml).

#ifndef AppVersion
  #define AppVersion "0.0.0-dev"
#endif

; Windows file version resources must be purely numeric, so drop any
; pre-release suffix (e.g. "0.3.0-beta.1" -> "0.3.0").
#if Pos("-", AppVersion) > 0
  #define NumericVersion Copy(AppVersion, 1, Pos("-", AppVersion) - 1)
#else
  #define NumericVersion AppVersion
#endif

#define AppName "KeyOverlay"
#define AppPublisher "KeyOverlay contributors"
#define AppURL "https://github.com/dshepstone/keyoverlay"
#define AppExeName "keyoverlay-app.exe"
#define RepoRoot ".."

[Setup]
; AppId uniquely identifies this application. Never change it, or upgrades
; will install side by side instead of replacing the previous version.
AppId={{E9957338-3990-4183-ADC5-C978BBEAFBFB}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}/issues
AppUpdatesURL={#AppURL}/releases
AppCopyright=Copyright (c) KeyOverlay contributors. MIT License.
VersionInfoVersion={#NumericVersion}
VersionInfoProductName={#AppName}
VersionInfoDescription={#AppName} Setup

DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
LicenseFile={#RepoRoot}\LICENSE

; Install for the current user by default (no admin prompt); the user can
; choose "install for all users" in the dialog, which elevates.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0

OutputDir={#RepoRoot}\target\installer
OutputBaseFilename=KeyOverlay-Setup-{#AppVersion}-x64
SetupIconFile=keyoverlay.ico
UninstallDisplayIcon={app}\{#AppExeName}
UninstallDisplayName={#AppName}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startup"; Description: "Start {#AppName} automatically when I sign in to Windows"; GroupDescription: "Startup:"; Flags: unchecked

[Files]
Source: "{#RepoRoot}\target\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "{#RepoRoot}\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start menu
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"
Name: "{group}\{#AppName} Website"; Filename: "{#AppURL}"
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"
; Optional desktop shortcut
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; Tasks: desktopicon
; Optional launch at sign-in (per-user or all-users Startup folder, matching install mode)
Name: "{autostartup}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; Tasks: startup

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Make sure a running instance does not block file removal.
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#AppExeName}"; Flags: runhidden; RunOnceId: "KillKeyOverlay"

; User settings in %APPDATA%\keyoverlay are intentionally left in place so a
; reinstall keeps the user's configuration.
